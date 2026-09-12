use crate::error::WebError;
use crate::error::WebResult;
use crate::services::agent_login::AgentLoginRegistry;
use crate::services::slide_build::SlideBuildRegistry;
use apich_db::CreateProjectDto;
use apich_db::Database;
use apich_db::Project;
use apich_db::ProjectSandbox;
use apich_sandbox::SandboxConfig;
use apich_sandbox::SandboxManager;
use apich_vcs::api::ProjectVcs;
use apich_vcs::IgnoreFilter;
use apich_vcs::Snapshot;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct ProjectManager {
    db: Arc<Database>,
    sandbox_manager: Arc<SandboxManager>,
    base_storage_dir: PathBuf,
    agent_logins: AgentLoginRegistry,
    pub slide_builds: SlideBuildRegistry,
}

impl ProjectManager {
    /// The four cross-compilation targets this app's sandbox image (see
    /// `docker/Containerfile.sandbox`'s cross-compilation section) is provisioned for, alongside
    /// the host's own native platform. Each value is a real Rust target triple `cargo slide build
    /// --target <triple>` is passed verbatim, validated against this allow-list before it ever
    /// reaches a shell command (see `start_slide_build`).
    pub const SLIDE_BUILD_TARGETS: &'static [(&'static str, &'static str)] = &[
        ("host", "This server's own platform (Linux, native)"),
        ("x86_64-pc-windows-gnu", "Windows x86_64"),
        ("aarch64-pc-windows-gnullvm", "Windows ARM64"),
        ("x86_64-unknown-linux-musl", "Linux x86_64 (musl, static)"),
        ("aarch64-unknown-linux-musl", "Linux ARM64 (musl, static)"),
    ];

    pub fn new(
        db: Arc<Database>,
        sandbox_manager: Arc<SandboxManager>,
        base_storage_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            sandbox_manager,
            base_storage_dir,
            agent_logins: AgentLoginRegistry::new(),
            slide_builds: SlideBuildRegistry::new(),
        }
    }

    /// Create a new project, provision storage workspace directory, and initialize `FastCDC` VCS
    pub async fn create_project(
        &self,
        mut dto: CreateProjectDto,
    ) -> WebResult<Project> {
        // Ensure storage path is set within base directory if not absolute
        let project_dir = if dto.storage_path.is_empty() {
            self.base_storage_dir
                .join(dto.org_id.to_string())
                .join(&dto.slug)
        } else {
            PathBuf::from(&dto.storage_path)
        };

        tokio::fs::create_dir_all(&project_dir).await.map_err(|e| {
            WebError::Internal(format!("Failed to create project workspace directory: {e}"))
        })?;

        // Canonicalize before persisting: `base_storage_dir` (from `APICH_STORAGE_DIR`, default
        // `./scratch/workspace`) is relative by default, and a caller-supplied `dto.storage_path`
        // isn't guaranteed absolute either. A relative path stored as-is is resolved fresh against
        // the server process's cwd every time it's used -- fine as long as the server always
        // launches from the same directory, but a *different* cwd on a later restart (a changed
        // systemd `WorkingDirectory`, a different deploy entrypoint, or simply invoking the binary
        // from elsewhere) silently resolves it to a different directory than the one this
        // project's sandbox container was actually bind-mounted from at creation time. Confirmed
        // live: exactly this drift left a real project's sandbox container serving stale files
        // from a stale path while the web UI (re-resolving the same relative `storage_path` fresh
        // against the new cwd) showed current files from a different directory -- the LaTeX
        // compiler then failed with "I can't find file" because the file genuinely wasn't in the
        // container's (mismatched) mount. Storing an absolute path here removes the cwd from the
        // equation entirely, for the lifetime of the project.
        let project_dir = tokio::fs::canonicalize(&project_dir).await.map_err(|e| {
            WebError::Internal(format!(
                "Failed to resolve project workspace directory: {e}"
            ))
        })?;

        // Initialize FastCDC Version Control in the project directory
        let _ = ProjectVcs::open_or_init(&project_dir)?;

        dto.storage_path = project_dir.to_string_lossy().to_string();

        let repo = self.db.repository();
        let mut proj = repo.create_project(dto).await?;
        repo.set_project_vcs_initialized(proj.id, true).await?;
        proj.vcs_initialized = true;

        info!(project_id = %proj.id, name = %proj.name, path = %proj.storage_path, "Created project with initialized VCS");
        Ok(proj)
    }

    fn project_sandbox_container_name(
        proj: &Project,
        user_id: Uuid,
    ) -> String {
        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        format!("apich-proj-{}-{}", proj.slug, user_suffix)
    }

    fn project_sandbox_config(
        proj: &Project,
        user_id: Uuid,
    ) -> SandboxConfig {
        let container_name = Self::project_sandbox_container_name(proj, user_id);
        // Real toolchain + CLI-agent image (docker/Containerfile.sandbox) -- must be built once
        // via `podman build -t apich-sandbox:latest -f docker/Containerfile.sandbox .` (or
        // scripts/build_sandbox_image.sh) before first run. Overridable for local/CI use with a
        // lighter image (e.g. bare alpine has no typst/rust/agents but starts instantly).
        let image = std::env::var("APICH_SANDBOX_IMAGE")
            .unwrap_or_else(|_| "localhost/apich-sandbox:latest".to_string());
        SandboxConfig::builder(user_id.to_string(), &proj.storage_path)
            .container_name(&container_name)
            .image(image)
            .memory_limit("2048m")
            .cpu_limit(2.0)
            // Without this, rootless Podman's default user-namespace mapping puts the *host*
            // UID that owns the bind-mounted project directory at UID 0 (root) *inside* the
            // container's namespace -- but the container's own non-root `apich` user (UID 1000
            // in Containerfile.sandbox) doesn't map to that, so every file the mounted directory
            // shows as `root root drwxr-xr-x` from inside the container, and any write from a
            // process running as `apich` (an agent editing a file, a script writing output, or
            // here, pdflatex writing its .log/.pdf) fails with a real, silent "Permission
            // denied" -- undermining the actual promise of running agents against real project
            // files. `--userns=keep-id` maps the container's UID 1000 directly to the *host's*
            // UID 1000 instead, so ownership lines up and writes succeed. Caught live: building
            // the LaTeX preview feature, `pdflatex` failed with `I can't write on file
            // 'report.log'` even though nothing about that feature touches permissions itself.
            .keep_id(true)
            .build()
    }

    /// Launch a dedicated container sandbox for a specific (Project + User) pair
    pub async fn launch_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> WebResult<ProjectSandbox> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        if proj.status == "archived" || proj.status == "deleted" {
            return Err(WebError::BadRequest(format!(
                "Cannot launch sandbox for {} project",
                proj.status
            )));
        }

        let container_name = Self::project_sandbox_container_name(&proj, user_id);
        let sandbox_config = Self::project_sandbox_config(&proj, user_id);

        info!(
            project_id = %project_id,
            user_id = %user_id,
            container = %container_name,
            "Launching project sandbox container"
        );

        self.sandbox_manager
            .ensure_running_with_config(sandbox_config)
            .await?;

        let sandbox = repo
            .upsert_project_sandbox(project_id, user_id, &container_name, "running")
            .await?;

        Ok(sandbox)
    }

    /// Execute a shell command inside the project's real sandbox container, auto-starting it
    /// first if it isn't already running (plan.md: containers must start automatically, never
    /// something the user has to manually turn on). Replaces the previous behavior, which ran
    /// the command directly on the host process with no container isolation at all.
    pub async fn exec_in_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        command: &str,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let container_name = format!("apich-proj-{}-{}", proj.slug, user_suffix);

        // Always go through `launch_sandbox` (-> `SandboxManager::ensure_running_with_config`,
        // which does a real `podman inspect` before deciding whether anything needs to happen) --
        // a `project_sandboxes.status == "running"` shortcut used to skip this whole check, which
        // is exactly what let this drift from reality: if the container was ever removed by
        // anything other than this app itself (an admin's `podman rm`, an OOM kill, a host
        // restart wiping containers), the DB row still says "running" forever, and every call
        // here silently tried to exec into a container that no longer existed -- confirmed live,
        // it fell through to reading a stale leftover output file from a much earlier real run
        // and presented that as if it were the current command's result, with no error at all.
        // `ensure_running_with_config`'s own inspect call is cheap when the container genuinely
        // is already running, so there's no real cost to just always taking this path.
        self.launch_sandbox(project_id, user_id).await?;

        // `sh -c` rather than `bash -c`: works on both the real toolchain image (which does
        // have bash) and a lighter override image (e.g. bare alpine, busybox sh only) without
        // needing to know in advance which one is configured.
        let opts = apich_sandbox::ExecOptions::new(["sh", "-c", command]);
        let result = self
            .sandbox_manager
            .driver()
            .exec(&container_name, &opts)
            .await?;

        let _ = repo.touch_sandbox_activity(project_id, user_id).await;

        Ok(format!(
            "{}{}",
            result.stdout_lossy(),
            result.stderr_lossy()
        ))
    }

    /// Auto-starts the project's real sandbox if needed and returns a handle to it. Shared by
    /// every agent-related operation (`run_agent_in_sandbox`, `agent_availability`,
    /// `start_agent_login`) since they all need the same "make sure it's running, then get a
    /// container handle" preamble.
    async fn ensure_agent_container(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> WebResult<apich_sandbox::UserContainer> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        // See `exec_in_sandbox`'s comment on why this always goes through `launch_sandbox`
        // (a real `podman inspect`-backed check) instead of trusting a DB "running" flag that can
        // silently go stale the moment a container disappears for any reason outside this app.
        self.launch_sandbox(project_id, user_id).await?;

        let config = Self::project_sandbox_config(&proj, user_id);
        Ok(apich_sandbox::UserContainer::new(
            config,
            self.sandbox_manager.driver().clone(),
        ))
    }

    /// Run a CLI coding agent (claude/codex/opencode/aider/goose) non-interactively inside the
    /// project's real sandbox container, against the real project files -- per plan.md, this is
    /// not a hosted chat API call, the agent gets a real shell in the real workspace. Auto-starts
    /// the sandbox first if needed. The caller's BYOK API key (if any) is passed straight through
    /// as an env var scoped to this one exec call; it is never persisted here.
    pub async fn run_agent_in_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        agent: apich_sandbox::tools::AgentKind,
        prompt: &str,
        api_key: Option<&str>,
    ) -> WebResult<apich_sandbox::ExecResult> {
        let container = self.ensure_agent_container(project_id, user_id).await?;
        let result = container.agents().run(agent, prompt, api_key).await?;
        let _ = self
            .db
            .repository()
            .touch_sandbox_activity(project_id, user_id)
            .await;
        Ok(result)
    }

    /// Run a Python/R/Rust/shell script inside the project's own sandbox container -- **not**
    /// on this dev host, which is exactly the bug this fixes: the host has no R (`Rscript`)
    /// installed at all, so running an `.R` script threw a real "Failed to spawn script process:
    /// No such file or directory" with no useful explanation to the user, even though R has been
    /// in `docker/Containerfile.sandbox` the whole time. The container's `/workspace` is the
    /// same bind-mounted directory as `project.storage_path` on the host, so a script's own file
    /// writes (a saved plot, a generated CSV) are visible from both sides -- the existing
    /// host-side `DocumentRenderer::scan_images` before/after diff still works unchanged.
    pub async fn run_script_in_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        args: &str,
    ) -> WebResult<crate::services::document_renderer::ScriptRunResult> {
        self.run_script_in_sandbox_with_env(project_id, user_id, rel_path, args, &[])
            .await
    }

    /// Same as `run_script_in_sandbox`, plus extra environment variables for the run -- used by
    /// the table notebook feature to hand a cell's Python/R code the SQLite file path it's
    /// attached to (`APICH_TABLE_DB`) without inventing a second script-execution pathway. A
    /// separate method rather than a defaulted parameter on the original so every existing
    /// caller (the plain script-runner console) stays untouched.
    pub async fn run_script_in_sandbox_with_env(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        args: &str,
        extra_env: &[(&str, &str)],
    ) -> WebResult<crate::services::document_renderer::ScriptRunResult> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let full_path = std::path::Path::new(&proj.storage_path).join(rel_path);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("Script not found: {rel_path}")));
        }
        let ext = full_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let container = self.ensure_agent_container(project_id, user_id).await?;

        // Same per-extension dispatch `DocumentRenderer::run_script` used -- but none of the
        // host-relative-path bookkeeping that used to be needed there, since the container's
        // exec already defaults its cwd to `/workspace` (see `UserContainer::exec`), so a plain
        // `rel_path` argument just works.
        let run_id = uuid::Uuid::new_v4().simple().to_string();
        let rust_bin_path = format!("/tmp/apich_run_{run_id}");
        let cmd: Vec<String> = match ext.as_str() {
            | "py" => vec!["python3".to_string(), rel_path.to_string()],
            | "r" => vec!["Rscript".to_string(), rel_path.to_string()],
            | "rs" => {
                vec![
                    "bash".to_string(),
                    "-c".to_string(),
                    format!(
                        r#"rustc -O "$1" -o "{bin}" && "{bin}" "${{@:2}}""#,
                        bin = rust_bin_path
                    ),
                    "bash".to_string(),
                    rel_path.to_string(),
                ]
            },
            | "js" | "ts" => vec!["node".to_string(), rel_path.to_string()],
            | _ => vec!["bash".to_string(), rel_path.to_string()],
        };
        let mut cmd = cmd;
        if !args.trim().is_empty() {
            cmd.extend(
                args.split_whitespace()
                    .map(std::string::ToString::to_string),
            );
        }

        let before_images = crate::services::document_renderer::DocumentRenderer::scan_images(
            std::path::Path::new(&proj.storage_path),
        );
        let start = std::time::Instant::now();

        let mut opts =
            apich_sandbox::ExecOptions::new(cmd).timeout(std::time::Duration::from_secs(120));
        for (k, v) in extra_env {
            opts = opts.env(*k, *v);
        }
        let result = container.exec_with_options(opts).await?;

        if ext == "rs" {
            let _ = container.exec(["rm", "-f", &rust_bin_path]).await;
        }
        let _ = repo.touch_sandbox_activity(project_id, user_id).await;

        let elapsed = start.elapsed().as_millis();
        let after_images = crate::services::document_renderer::DocumentRenderer::scan_images(
            std::path::Path::new(&proj.storage_path),
        );
        let mut output_images = Vec::new();
        for (img_path, mtime) in &after_images {
            let is_new_or_modified = match before_images.get(img_path) {
                | Some(old_mtime) => mtime > old_mtime,
                | None => true,
            };
            if is_new_or_modified {
                if let Ok(bytes) = tokio::fs::read(img_path).await {
                    let img_ext = img_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png")
                        .to_lowercase();
                    let mime = match img_ext.as_str() {
                        | "svg" => "image/svg+xml",
                        | "jpg" | "jpeg" => "image/jpeg",
                        | _ => "image/png",
                    };
                    let b64 =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                    let data_uri = format!("data:{mime};base64,{b64}");
                    let name = img_path.file_name().map_or_else(
                        || "output.png".to_string(),
                        |n| n.to_string_lossy().to_string(),
                    );
                    output_images.push(crate::services::document_renderer::ScriptOutputImage {
                        name,
                        data_uri,
                    });
                }
            }
        }

        Ok(crate::services::document_renderer::ScriptRunResult {
            success: result.success(),
            exit_code: Some(result.exit_code),
            stdout: result.stdout_lossy().into_owned(),
            stderr: result.stderr_lossy().into_owned(),
            execution_time_ms: elapsed,
            output_images,
        })
    }

    /// Compile a real `.tex` file to PDF inside the project's own sandbox container, which
    /// already has a full TeX Live install (see `docker/Containerfile.sandbox`) -- deliberately
    /// not installed on this host: unlike `typst` (a single static binary, dev-convenience only),
    /// a real LaTeX toolchain is a large, security-sensitive system package set, and on this
    /// Qubes dev qube `dnf install` for it means trusting a new package-signing key, not
    /// something to do silently as a side effect of a feature request. Runs the selected `engine`
    /// (`pdflatex`, `xelatex`, or `lualatex` -- the three actually installed in the image) twice (LaTeX needs a
    /// second pass to resolve cross-references/citations before they're stable -- a well-known
    /// real quirk of the toolchain, not a workaround for a bug here) and returns either the
    /// compiled PDF bytes or, on failure, the compiler's own log so the user sees a real
    /// diagnostic instead of a silent blank preview.
    pub async fn compile_latex_in_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        engine: &str,
    ) -> WebResult<Result<Vec<u8>, String>> {
        let container = self.ensure_agent_container(project_id, user_id).await?;

        let file_stem = std::path::Path::new(rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| WebError::BadRequest("Invalid .tex file path".to_string()))?;
        let parent = std::path::Path::new(rel_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // `engine` is expected to already be validated against the set actually installed in the
        // sandbox image (see `handlers::sanitize_latex_engine`) -- `pdflatex`/`xelatex`/`lualatex`
        // all accept the same CLI flags used here. `-synctex=1` makes the engine also emit a
        // `<stem>.synctex.gz` alongside the PDF, which `synctex_edit_in_sandbox` below queries for
        // click-to-jump (PDF click -> source line) -- without it `synctex edit` has nothing to
        // read and always reports no match.
        let cmd = vec![
            engine.to_string(),
            "-interaction=nonstopmode".to_string(),
            "-halt-on-error".to_string(),
            "-synctex=1".to_string(),
            rel_path.to_string(),
        ];
        // Two passes: the first resolves the document structure, the second resolves any
        // references/citations that depend on it (real LaTeX behavior -- a single pass leaves
        // `??` in place of forward references on any document that has any).
        let _ = container.exec(cmd.clone()).await?;
        let result = container.exec(cmd).await?;

        let _ = self
            .db
            .repository()
            .touch_sandbox_activity(project_id, user_id)
            .await;

        let pdf_rel_path = if parent.is_empty() {
            format!("{file_stem}.pdf")
        } else {
            format!("{parent}/{file_stem}.pdf")
        };

        if !result.success() {
            let log_rel_path = if parent.is_empty() {
                format!("{file_stem}.log")
            } else {
                format!("{parent}/{file_stem}.log")
            };
            let log = container
                .read_file_str(&log_rel_path)
                .await
                .unwrap_or_else(|_| format!("{}{}", result.stdout_lossy(), result.stderr_lossy()));
            return Ok(Err(log));
        }

        match container.read_file(&pdf_rel_path).await {
            | Ok(bytes) => Ok(Ok(bytes)),
            | Err(e) => {
                Ok(Err(format!(
                    "pdflatex reported success but no PDF was found: {e}"
                )))
            },
        }
    }

    /// Compiles a LaTeX file that doesn't belong to any real project -- a Template Library
    /// template's own content, previewed before anyone has applied it anywhere -- by spinning up
    /// a genuinely throwaway container (own scratch host directory, own container name, `pdflatex`
    /// run exactly like `compile_latex_in_sandbox` does for a real project) and tearing both down
    /// again afterward regardless of outcome. Unlike every other sandbox container in this app,
    /// this one is never meant to persist between calls -- there's no project for it to belong to.
    pub async fn compile_latex_preview_ephemeral(
        &self,
        files: &[(String, String)],
    ) -> WebResult<Result<Vec<u8>, String>> {
        let scratch_id = Uuid::new_v4();
        let host_dir = self
            .base_storage_dir
            .join("_template_previews")
            .join(scratch_id.to_string());
        tokio::fs::create_dir_all(&host_dir)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to create preview workspace: {e}")))?;

        for (rel_path, content) in files {
            let target = host_dir.join(rel_path);
            if let Some(parent) = target.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if tokio::fs::write(&target, content).await.is_err() {
                let _ = tokio::fs::remove_dir_all(&host_dir).await;
                return Err(WebError::Internal(format!(
                    "Failed to write {rel_path} into preview workspace"
                )));
            }
        }

        let Some((main_path, _)) = files.first() else {
            let _ = tokio::fs::remove_dir_all(&host_dir).await;
            return Ok(Err("No files to compile".to_string()));
        };
        let file_stem = std::path::Path::new(main_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main")
            .to_string();

        let container_name = format!("apich-preview-{scratch_id}");
        let image = std::env::var("APICH_SANDBOX_IMAGE")
            .unwrap_or_else(|_| "localhost/apich-sandbox:latest".to_string());
        let config = SandboxConfig::builder(format!("preview-{scratch_id}"), &host_dir)
            .container_name(&container_name)
            .image(image)
            .memory_limit("1024m")
            .cpu_limit(1.0)
            .keep_id(true)
            .build();

        let outcome: WebResult<Result<Vec<u8>, String>> = async {
            let container = self
                .sandbox_manager
                .ensure_running_with_config(config)
                .await
                .map_err(|e| {
                    WebError::Internal(format!("Failed to start preview container: {e}"))
                })?;

            let cmd = vec![
                "pdflatex".to_string(),
                "-interaction=nonstopmode".to_string(),
                "-halt-on-error".to_string(),
                main_path.clone(),
            ];
            let _ = container.exec(cmd.clone()).await;
            let result = container
                .exec(cmd)
                .await
                .map_err(|e| WebError::Internal(format!("Failed to run pdflatex: {e}")))?;

            if !result.success() {
                let log = container
                    .read_file_str(format!("{file_stem}.log"))
                    .await
                    .unwrap_or_else(|_| {
                        format!("{}{}", result.stdout_lossy(), result.stderr_lossy())
                    });
                return Ok(Err(log));
            }

            match container.read_file(format!("{file_stem}.pdf")).await {
                | Ok(bytes) => Ok(Ok(bytes)),
                | Err(e) => {
                    Ok(Err(format!(
                        "pdflatex reported success but no PDF was found: {e}"
                    )))
                },
            }
        }
        .await;

        // Always torn down, success or failure -- this container and its scratch directory have
        // no reason to exist a moment longer than this one preview request.
        let _ = self
            .sandbox_manager
            .driver()
            .remove(&container_name, true)
            .await;
        let _ = tokio::fs::remove_dir_all(&host_dir).await;

        outcome
    }

    /// Reverse search: given a click position inside the compiled PDF (page number, plus x/y in
    /// PDF points from the page's top-left -- the units `synctex edit`'s `-o` spec expects),
    /// returns the `.tex` source line that produced whatever's at that spot, via the real
    /// `synctex` CLI (part of every TeX Live install) reading the `.synctex.gz` written by
    /// `compile_latex_in_sandbox`'s `-synctex=1`. This is the same mechanism real LaTeX IDEs
    /// (`TeXstudio`, Overleaf, VS Code's LaTeX Workshop) use for PDF-click-to-source-line --
    /// there's no simpler approximation that's actually correct, since where a glyph ends up on
    /// the page depends on the full TeX layout algorithm, not just line-counting.
    /// Returns `Ok(None)` (not an error) if synctex has no record for that exact spot, which is
    /// normal for a click that lands on whitespace/margin rather than typeset content.
    pub async fn synctex_edit_in_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        page: u32,
        x: f64,
        y: f64,
    ) -> WebResult<Option<u32>> {
        let container = self.ensure_agent_container(project_id, user_id).await?;

        let file_stem = std::path::Path::new(rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| WebError::BadRequest("Invalid .tex file path".to_string()))?;
        let parent = std::path::Path::new(rel_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let pdf_rel_path = if parent.is_empty() {
            format!("{file_stem}.pdf")
        } else {
            format!("{parent}/{file_stem}.pdf")
        };

        let spec = format!("{page}:{x}:{y}:{pdf_rel_path}");
        let cmd = vec![
            "synctex".to_string(),
            "edit".to_string(),
            "-o".to_string(),
            spec,
        ];
        let result = container.exec(cmd).await?;
        let _ = self
            .db
            .repository()
            .touch_sandbox_activity(project_id, user_id)
            .await;

        // `synctex edit`'s output is plain text with one `Key:Value` pair per line inside a
        // `SyncTeX result begin ... end` block, e.g. `Input:/workspace/report.tex` / `Line:12`.
        // Only the numeric line matters here; the caller already knows which file it asked about.
        let stdout = result.stdout_lossy();
        let line = stdout
            .lines()
            .find_map(|l| l.strip_prefix("Line:"))
            .and_then(|n| n.trim().parse::<u32>().ok());
        Ok(line)
    }

    /// Starts compiling a cargo-slide presentation (`.typ` using `#slide(...)`) into a
    /// standalone, self-contained native binary inside the project's sandbox container, for
    /// `target` (a triple from `SLIDE_BUILD_TARGETS`, or `"host"` for the container's own native
    /// platform) -- as a real background job rather than blocking the request for as long as the
    /// build takes (the first build against a fresh cross-compilation target can run for several
    /// minutes: a from-scratch dependency tree, cross-compiled). Returns the new job id
    /// immediately; poll it via `self.slide_builds`.
    ///
    /// The first build in a fresh container can take a while even for the host's own platform:
    /// each container starts with an empty Cargo registry cache, so `cargo slide build`
    /// downloads and compiles its own dependency tree from scratch every time.
    pub async fn start_slide_build(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        target: &str,
    ) -> WebResult<Uuid> {
        let container = self.ensure_agent_container(project_id, user_id).await?;

        let run_id = uuid::Uuid::new_v4().simple().to_string();
        let out_rel_path = format!(".apich_slide_build_{run_id}");
        let download_stem = std::path::Path::new(rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("presentation")
            .to_string();
        let is_windows_target = target.contains("windows");
        let download_name = if is_windows_target {
            format!("{download_stem}-presentation.exe")
        } else {
            format!("{download_stem}-presentation")
        };

        let mut cmd = vec![
            "cargo".to_string(),
            "slide".to_string(),
            "--log-format".to_string(),
            "json".to_string(),
            "build".to_string(),
            rel_path.to_string(),
            "-o".to_string(),
            out_rel_path.clone(),
        ];
        if target != "host" {
            cmd.push("--target".to_string());
            cmd.push(target.to_string());
        }

        let opts = apich_sandbox::ExecOptions::new(cmd).timeout(std::time::Duration::from_mins(30));
        let stream = container.exec_stream(opts).await?;

        let _ = self
            .db
            .repository()
            .touch_sandbox_activity(project_id, user_id)
            .await;

        let job_id = self
            .slide_builds
            .start(user_id, container, stream, out_rel_path, download_name)
            .await;
        Ok(job_id)
    }

    /// Which agent CLIs are actually installed in the image this project's sandbox would use --
    /// checked live inside a real (auto-started) container rather than assumed, since an image
    /// build can silently skip a tool (see docker/Containerfile.sandbox's per-tool guards).
    pub async fn agent_availability(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> WebResult<Vec<(apich_sandbox::tools::AgentKind, bool)>> {
        let container = self.ensure_agent_container(project_id, user_id).await?;
        Ok(container.agents().availability().await?)
    }

    /// Starts a real account-login flow for the given agent (e.g. `claude auth login`, `codex
    /// login --device-auth`) inside the project's sandbox and registers it so its output can be
    /// polled and, if it needs one, a pasted-back code submitted across separate requests.
    /// Returns `None` if this agent has no real login flow (see `AgentKind::login_support`) --
    /// the caller should offer BYOK instead rather than a login button that does nothing real.
    pub async fn start_agent_login(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        agent: apich_sandbox::tools::AgentKind,
    ) -> WebResult<Option<Uuid>> {
        let container = self.ensure_agent_container(project_id, user_id).await?;
        // Kill any earlier run of this same agent binary still alive in the container before
        // starting a new login attempt. Without this, repeated attempts (a user retrying after
        // one that stalled or was abandoned) accumulate competing processes forever -- confirmed
        // live: three separate `agy` processes (plus three zombie `xdg-open` children under them)
        // were all still running in one container from earlier sessions, none ever reaped, each
        // holding its own pty and none of them the one the *current* login session's stdin/stdout
        // actually talk to.
        let _ = container.exec(["pkill", "-9", "-x", agent.binary()]).await;
        let Some(exec) = container.agents().login(agent).await? else {
            return Ok(None);
        };
        let session_id = self
            .agent_logins
            .start(agent, user_id, project_id, exec)
            .await;
        Ok(Some(session_id))
    }

    /// Current accumulated output and status of a login session started by `start_agent_login`.
    /// Returns `None` if the session doesn't exist (never existed, already expired, or belongs
    /// to a different user -- callers must check `owner_user_id` themselves before trusting
    /// output, this only looks the record up).
    pub async fn agent_login_snapshot(
        &self,
        session_id: Uuid,
    ) -> Option<(
        apich_sandbox::tools::AgentKind,
        Uuid,
        String,
        crate::services::agent_login::AgentLoginStatus,
    )> {
        let session = self.agent_logins.get(session_id).await?;
        let (output, status) = session.snapshot().await;
        Some((session.agent, session.owner_user_id, output, status))
    }

    /// Writes one line of pasted-back input (e.g. an OAuth code) into a running login session's
    /// stdin. Fails if the session doesn't exist, already finished, or never needed input in the
    /// first place (a `DeviceCode` flow like Codex's polls on its own).
    pub async fn submit_agent_login_input(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        line: &str,
    ) -> WebResult<()> {
        let session = self
            .agent_logins
            .get(session_id)
            .await
            .ok_or_else(|| WebError::NotFound("Login session not found".to_string()))?;
        if session.owner_user_id != user_id {
            return Err(WebError::Forbidden(
                "This login session belongs to a different user".to_string(),
            ));
        }
        self.agent_logins
            .submit_input(session_id, line)
            .await
            .map_err(|e| WebError::BadRequest(e.to_string()))
    }

    /// Stop a dedicated container sandbox for a (Project + User) pair
    pub async fn stop_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> WebResult<ProjectSandbox> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let container_name = format!("apich-proj-{}-{}", proj.slug, user_suffix);

        info!(
            project_id = %project_id,
            user_id = %user_id,
            container = %container_name,
            "Stopping project sandbox container"
        );

        // Stop container via container manager (ignore error if already stopped)
        let _ = self
            .sandbox_manager
            .stop_container(&container_name, 10)
            .await;

        let sandbox = repo
            .upsert_project_sandbox(project_id, user_id, &container_name, "stopped")
            .await?;

        Ok(sandbox)
    }

    /// Stop every sandbox that's been sitting idle (no exec/agent-run activity, and not just
    /// started) longer than `idle_timeout` -- the other half of "the user shall not feel them":
    /// starting is already automatic (see `exec_in_sandbox`/`ensure_agent_container`), this is
    /// what makes stopping automatic too, instead of a container running forever once opened.
    /// Intended to be called on a periodic background loop (see `main.rs`), not from a request
    /// handler. Returns how many sandboxes it stopped, for logging.
    pub async fn reap_idle_sandboxes(
        &self,
        idle_timeout: chrono::Duration,
    ) -> WebResult<usize> {
        let repo = self.db.repository();
        let idle_since = chrono::Utc::now()
            .checked_sub_signed(idle_timeout)
            .unwrap_or_else(chrono::Utc::now);
        let idle = repo.list_idle_running_sandboxes(idle_since).await?;

        let mut stopped: usize = 0;
        for sandbox in idle {
            match self.stop_sandbox(sandbox.project_id, sandbox.user_id).await {
                | Ok(_) => {
                    stopped = stopped.saturating_add(1);
                    info!(
                        project_id = %sandbox.project_id,
                        user_id = %sandbox.user_id,
                        container = %sandbox.container_name,
                        "Stopped idle sandbox"
                    );
                },
                | Err(e) => {
                    tracing::warn!(
                        project_id = %sandbox.project_id,
                        user_id = %sandbox.user_id,
                        error = %e,
                        "Failed to stop idle sandbox"
                    );
                },
            }
        }
        Ok(stopped)
    }

    /// Reaps stopped sandbox containers exceeding grace period.
    ///
    /// Actually removes (`podman rm`, not just stops) every sandbox container that has sat
    /// `stopped` for longer than `grace_period`, and drops its tracking row once the container is
    /// gone. `reap_idle_sandboxes` only ever stops a running container -- nothing previously
    /// deleted one, so every sandbox this app ever started accumulated on the host forever:
    /// confirmed live, dozens of `apich-proj-*` containers going back over a day, none removed.
    /// The grace period (independent of, and normally much longer than, the idle-stop timeout)
    /// exists so a user who stops a sandbox and comes back an hour later still gets a fast restart
    /// via `ensure_running_with_config`'s "container exists, just start it" path rather than a
    /// full re-create from the image every time.
    pub async fn reap_stopped_sandboxes(
        &self,
        grace_period: chrono::Duration,
    ) -> WebResult<usize> {
        let repo = self.db.repository();
        let stopped_since = chrono::Utc::now()
            .checked_sub_signed(grace_period)
            .unwrap_or_else(chrono::Utc::now);
        let stale = repo.list_stale_stopped_sandboxes(stopped_since).await?;

        let mut removed: usize = 0;
        for sandbox in stale {
            match self
                .sandbox_manager
                .driver()
                .remove(&sandbox.container_name, true)
                .await
            {
                | Ok(()) => {
                    let _ = repo
                        .delete_project_sandbox(sandbox.project_id, sandbox.user_id)
                        .await;
                    removed = removed.saturating_add(1);
                    info!(
                        project_id = %sandbox.project_id,
                        user_id = %sandbox.user_id,
                        container = %sandbox.container_name,
                        "Removed stale stopped sandbox container"
                    );
                },
                | Err(e) => {
                    tracing::warn!(
                        project_id = %sandbox.project_id,
                        user_id = %sandbox.user_id,
                        container = %sandbox.container_name,
                        error = %e,
                        "Failed to remove stale stopped sandbox container"
                    );
                },
            }
        }
        Ok(removed)
    }

    /// Reaps orphaned sandbox containers with no DB tracking row.
    ///
    /// Removes any `apich.managed=true` container that exists on the host but has no tracking row
    /// in this app's own database at all -- a container this app itself started but subsequently
    /// lost track of (its row was deleted, e.g. by `reap_stopped_sandboxes` running concurrently
    /// with something else re-touching the same row, or by manual DB surgery) can otherwise never
    /// be found again through the normal (project, user) lookup path and just sits there forever,
    /// invisible to every other reaper here since they all start from the DB's own row list, not
    /// from the real, ground-truth container list podman itself holds.
    pub async fn reap_orphaned_containers(&self) -> WebResult<usize> {
        let repo = self.db.repository();
        let managed = self.sandbox_manager.list_managed_sandboxes().await?;
        let tracked: std::collections::HashSet<String> = repo
            .list_all_sandbox_container_names()
            .await?
            .into_iter()
            .collect();

        let mut removed: usize = 0;
        for name in managed {
            if tracked.contains(&name) {
                continue;
            }
            match self.sandbox_manager.driver().remove(&name, true).await {
                | Ok(()) => {
                    removed = removed.saturating_add(1);
                    info!(container = %name, "Removed orphaned sandbox container with no tracking row");
                },
                | Err(e) => {
                    tracing::warn!(container = %name, error = %e, "Failed to remove orphaned sandbox container");
                },
            }
        }
        Ok(removed)
    }

    /// Take a VCS snapshot if there are working copy modifications
    pub async fn snapshot_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        message: &str,
    ) -> WebResult<Option<Snapshot>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let commit_msg = format!("{message} (by user {user_suffix})");
        let summary = vcs.snapshot_if_changed(&commit_msg)?;

        Ok(summary)
    }

    /// Retrieve the VCS snapshot timeline for a project
    pub async fn get_project_timeline(
        &self,
        project_id: Uuid,
    ) -> WebResult<Vec<Snapshot>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let mut snapshots = vcs.list_snapshots()?;
        snapshots.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(snapshots)
    }

    /// Verify each signed snapshot's GPG signature against the set of GPG public keys registered
    /// by anyone with access to the project (see `Repository::list_gpg_public_keys_for_project`),
    /// for "vigilant mode" display. Only called when a project has vigilant mode on -- each check
    /// shells out to `gpg`, so this is skipped entirely otherwise. A snapshot with no signature at
    /// all is left out of the returned map (the caller treats "absent" as unsigned).
    pub async fn verify_snapshot_signatures(
        &self,
        project_id: Uuid,
        snapshots: &[Snapshot],
    ) -> WebResult<std::collections::HashMap<Uuid, apich_vcs::SignatureStatus>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let keys = repo.list_gpg_public_keys_for_project(project_id).await?;
        if keys.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let mut results = std::collections::HashMap::new();
        for snap in snapshots {
            if snap.gpg_signature.is_none() {
                continue;
            }
            let mut best: Option<apich_vcs::SignatureStatus> = None;
            for key in &keys {
                match vcs.verify_snapshot_signature(snap.id, &key.public_key) {
                    | Ok(Some(apich_vcs::SignatureStatus::Valid { fingerprint })) => {
                        best = Some(apich_vcs::SignatureStatus::Valid { fingerprint });
                        break;
                    },
                    | Ok(Some(status @ apich_vcs::SignatureStatus::Invalid(_))) => {
                        if best.is_none() {
                            best = Some(status);
                        }
                    },
                    | _ => {},
                }
            }
            if let Some(status) = best {
                results.insert(snap.id, status);
            }
        }
        Ok(results)
    }

    /// Export a project's full apich-vcs history as a downloadable bundle (tar.gz), for
    /// apich-vcs's own "clone"/"pull" over HTTP -- see `apich_vcs::bundle::ProjectBundle`.
    pub async fn export_vcs_bundle(
        &self,
        project_id: Uuid,
    ) -> WebResult<Vec<u8>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let mut bytes = Vec::new();
        vcs.export_bundle(&mut bytes, apich_vcs::BundleOptions::default())?;
        Ok(bytes)
    }

    /// Accept an uploaded bundle as a "push" to this project's apich-vcs history -- see
    /// `apich_vcs::bundle::ProjectBundle::accept_push` for the fast-forward safety rules.
    pub async fn accept_vcs_push(
        &self,
        project_id: Uuid,
        bundle_bytes: &[u8],
    ) -> WebResult<apich_vcs::PushOutcome> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let outcome = vcs.accept_push_bundle(bundle_bytes)?;
        Ok(outcome)
    }

    /// Retrieve branches (current active branch and all branches)
    pub async fn get_branches(
        &self,
        project_id: Uuid,
    ) -> WebResult<(Option<String>, Vec<String>)> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let current = vcs.current_branch()?;
        let branches = vcs.list_branches()?;
        Ok((current, branches))
    }

    /// Create a new branch pointing at the current HEAD
    pub async fn branch_create(
        &self,
        project_id: Uuid,
        name: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.branch_create(name)?;
        Ok(())
    }

    /// Switch the current branch (checks out its HEAD tree into the working directory)
    pub async fn branch_switch(
        &self,
        project_id: Uuid,
        name: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.branch_switch(name)?;
        Ok(())
    }

    /// Mark the current HEAD snapshot (or take a fresh one) as a named milestone
    pub async fn create_milestone(
        &self,
        project_id: Uuid,
        name: &str,
        desc: &str,
    ) -> WebResult<Snapshot> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let snap = vcs.create_milestone(name, desc)?;
        Ok(snap)
    }

    /// List all snapshots marked as milestones
    pub async fn list_milestones(
        &self,
        project_id: Uuid,
    ) -> WebResult<Vec<Snapshot>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let mut ms = vcs.list_milestones()?;
        ms.sort_by_key(|s| std::cmp::Reverse(s.created_at));
        Ok(ms)
    }

    /// Undo the last reversible VCS operation (branch HEAD moves back per the `OpLog`)
    pub async fn vcs_undo(
        &self,
        project_id: Uuid,
    ) -> WebResult<Option<Uuid>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        Ok(vcs.undo()?)
    }

    /// Redo the last undone VCS operation
    pub async fn vcs_redo(
        &self,
        project_id: Uuid,
    ) -> WebResult<Option<Uuid>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        Ok(vcs.redo()?)
    }

    /// Execute a weave-free 3-way merge from another branch
    pub async fn merge_branch(
        &self,
        project_id: Uuid,
        other_branch: &str,
    ) -> WebResult<MergeSummary> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let reconcile = vcs.merge(other_branch)?;
        let conflicts = self.detect_conflicts(project_id).await?;

        Ok(MergeSummary {
            auto_merged_count: reconcile.auto_merged_count,
            conflicts_count: conflicts.len(),
            conflicts,
        })
    }

    /// Detect files with active conflict markers in the workspace
    pub async fn detect_conflicts(
        &self,
        project_id: Uuid,
    ) -> WebResult<Vec<ConflictFileView>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let proj_root = PathBuf::from(&proj.storage_path);
        let mut conflicts = Vec::new();

        let mut dirs = vec![proj_root.clone()];
        while let Some(dir) = dirs.pop() {
            let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
                continue;
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();

                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        dirs.push(path);
                    } else if ft.is_file() {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            if content.contains("<<<<<<<") && content.contains(">>>>>>>") {
                                let rel = path
                                    .strip_prefix(&proj_root)
                                    .unwrap_or(&path)
                                    .to_string_lossy()
                                    .to_string();

                                let (ours, theirs) = parse_conflict_snippets(&content);
                                conflicts.push(ConflictFileView {
                                    path: rel,
                                    full_content: content,
                                    ours_snippet: ours,
                                    theirs_snippet: theirs,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Resolve a conflicted file using ours, theirs, or custom resolved content
    pub async fn resolve_conflict(
        &self,
        project_id: Uuid,
        rel_path: &str,
        choice: &str,
        custom_content: Option<&str>,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let full_path = PathBuf::from(&proj.storage_path).join(rel_path);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        let new_content = if let Some(custom) = custom_content {
            custom.to_string()
        } else {
            let raw = tokio::fs::read_to_string(&full_path)
                .await
                .map_err(|e| WebError::Internal(format!("Failed to read file: {e}")))?;
            resolve_conflict_content(&raw, choice)
        };

        tokio::fs::write(&full_path, new_content.as_bytes())
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write resolved file: {e}")))?;

        // Create snapshot recording the resolution
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Resolved merge conflict in {rel_path}"))?;

        Ok(())
    }

    /// Synchronize project history with Git bridge
    pub async fn git_sync(
        &self,
        project_id: Uuid,
        commit_message: &str,
    ) -> WebResult<Option<String>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_init()?;

        let current_branch = vcs.current_branch()?.unwrap_or_else(|| "main".to_string());
        if vcs.head_snapshot()?.is_some() {
            let commit_oid = vcs.git_export_commit(
                &current_branch,
                commit_message,
                "APICH System",
                "dev@apich.org",
            )?;
            let storage_path = std::path::PathBuf::from(&proj.storage_path);
            let _ = crate::services::git_server::sync_mirror_from_working(&storage_path).await;
            Ok(Some(commit_oid))
        } else {
            Ok(None)
        }
    }

    /// Retrieve Git bridge status
    pub async fn get_git_status(
        &self,
        project_id: Uuid,
    ) -> WebResult<GitStatusView> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let git_dir = PathBuf::from(&proj.storage_path).join(".git");
        let initialized = git_dir.exists();

        let remotes = vcs.git_remotes().unwrap_or_default();
        let remote_url = remotes.first().map(|(_, u)| u.clone());

        let branches = vcs.list_branches().unwrap_or_default();
        let current_branch = vcs.current_branch().ok().flatten();

        Ok(GitStatusView {
            initialized,
            remote_url,
            remotes,
            branches,
            current_branch,
        })
    }

    /// Fetch the project's ignore configuration (profile toggles + custom rules).
    pub async fn get_ignore_config(
        &self,
        project_id: Uuid,
    ) -> WebResult<apich_vcs::IgnoreConfig> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        Ok(vcs.config().ignore.clone())
    }

    /// Enable or disable a named ignore profile and persist it to the project's config file.
    pub async fn set_ignore_profile(
        &self,
        project_id: Uuid,
        profile_id: &str,
        enabled: bool,
    ) -> WebResult<()> {
        let profile = ignore_profile_from_id(profile_id)
            .ok_or_else(|| WebError::BadRequest(format!("Unknown ignore profile: {profile_id}")))?;
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let mut vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        if enabled {
            vcs.enable_ignore_profile(profile)?;
        } else {
            vcs.disable_ignore_profile(profile)?;
        }
        vcs.save_config()?;
        Ok(())
    }

    /// Add a custom ignore/whitelist rule (e.g. `*.tmp` or `!keep-me.csv`) and persist it.
    pub async fn add_ignore_rule(
        &self,
        project_id: Uuid,
        rule: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let mut vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.add_ignore_rule(rule)?;
        vcs.save_config()?;
        Ok(())
    }

    /// Remove a custom ignore/whitelist rule and persist it.
    pub async fn remove_ignore_rule(
        &self,
        project_id: Uuid,
        rule: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let mut vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.remove_ignore_rule(rule)?;
        vcs.save_config()?;
        Ok(())
    }

    /// Read the raw contents of the project's `.gitignore` or `.apichignore` file (empty string
    /// if the file does not exist yet).
    pub async fn read_ignore_file(
        &self,
        project_id: Uuid,
        file_name: &str,
    ) -> WebResult<String> {
        let file_name = validate_ignore_file_name(file_name)?;
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let path = PathBuf::from(&proj.storage_path).join(file_name);
        match tokio::fs::read_to_string(&path).await {
            | Ok(content) => Ok(content),
            | Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            | Err(e) => {
                Err(WebError::Internal(format!(
                    "Failed to read {file_name}: {e}"
                )))
            },
        }
    }

    /// Overwrite the project's `.gitignore` or `.apichignore` file with new raw content, then
    /// reload the live ignore filter so the change takes effect immediately.
    pub async fn write_ignore_file(
        &self,
        project_id: Uuid,
        file_name: &str,
        content: &str,
    ) -> WebResult<()> {
        let file_name = validate_ignore_file_name(file_name)?;
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let path = PathBuf::from(&proj.storage_path).join(file_name);
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write {file_name}: {e}")))?;
        let mut vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.reload_config()?;
        Ok(())
    }

    /// Add or update a Git remote on the project's working repository.
    pub async fn git_add_remote(
        &self,
        project_id: Uuid,
        name: &str,
        url: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_setup_remote(name, url)?;
        Ok(())
    }

    /// Fetch from a Git remote without merging.
    pub async fn git_fetch(
        &self,
        project_id: Uuid,
        remote: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_fetch(remote)?;
        Ok(())
    }

    /// Rebase the current branch onto an upstream branch or commit-ish, then re-snapshot the
    /// resulting working tree so the rebase is reflected in the apich-vcs timeline too.
    pub async fn git_rebase(
        &self,
        project_id: Uuid,
        upstream: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_rebase(upstream)?;
        let _ = vcs.snapshot_if_changed(format!("Rebased onto {upstream}"));
        Ok(())
    }

    /// Push the current (or given) branch to a Git remote.
    pub async fn git_push(
        &self,
        project_id: Uuid,
        remote: &str,
        branch: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_push(remote, branch)?;
        Ok(())
    }

    /// Pull from a Git remote into the current branch, then re-snapshot the result.
    pub async fn git_pull(
        &self,
        project_id: Uuid,
        remote: &str,
        branch: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.git_pull(remote, branch)?;
        let _ = vcs.snapshot_if_changed(format!("Pulled from {remote} {branch}"));
        Ok(())
    }

    /// Archive a project and cleanly terminate any active containers
    pub async fn archive_project(
        &self,
        project_id: Uuid,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        repo.update_project_status(project_id, "archived").await?;
        info!(project_id = %project_id, "Project archived");
        Ok(())
    }

    /// Delete a project (soft delete)
    pub async fn delete_project(
        &self,
        project_id: Uuid,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        repo.update_project_status(project_id, "deleted").await?;
        info!(project_id = %project_id, "Project marked deleted");
        Ok(())
    }

    /// List all working copy files in a project workspace
    pub async fn list_files(
        &self,
        project_id: Uuid,
    ) -> WebResult<Vec<ProjectFileItem>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let proj_root = PathBuf::from(&proj.storage_path);
        if !proj_root.exists() {
            return Ok(Vec::new());
        }

        // Real LaTeX/Typst/Python/R build byproducts (report.aux, report.log, __pycache__/...)
        // land in this same directory once a user compiles/runs something, and without this
        // filter they showed up as regular, shareable "files" right alongside the source the
        // user actually wrote -- confirmed live via a screenshot of a project that had been
        // LaTeX-compiled, listing `report.aux`/`report.log` as ordinary Private files. The VCS
        // layer already excludes these from snapshots via `IgnoreProfile::Academic` etc. (see
        // `apich_vcs::ignore`); this reuses the exact same rules so the file browser agrees with
        // what actually gets versioned.
        let ignore_filter = IgnoreFilter::new_with_defaults(&proj_root).ok();

        let mut items = Vec::new();
        let mut dirs = vec![proj_root.clone()];

        while let Some(dir) = dirs.pop() {
            let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
                continue;
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_name.starts_with('.')
                    || file_name == "target"
                    || file_name == "node_modules"
                {
                    continue;
                }
                // The auto-materialized `<name>.csv.table` sibling `SqliteTableService::
                // resolve_db_path` creates behind a `.csv` file the first time it's opened as a
                // table -- an implementation detail (the CSV's own data mirrored into a real
                // SQLite file so the spreadsheet grid/SQL console work), not a second file the
                // user actually created. `sqlite_table.rs`'s own directory listing already
                // excludes this exact pattern; this file's listing hadn't, so both showed up
                // side by side in the Files tab looking like two separate, unexplained files.
                if file_name.to_lowercase().ends_with(".csv.table") {
                    continue;
                }

                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        dirs.push(path);
                    } else if ft.is_file() {
                        let rel = path
                            .strip_prefix(&proj_root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/");

                        if ignore_filter.as_ref().is_some_and(|f| f.is_ignored(&rel)) {
                            continue;
                        }

                        let meta = entry.metadata().await.ok();
                        let size_bytes = meta.as_ref().map_or(0, std::fs::Metadata::len);
                        let modified_rfc3339 = meta
                            .and_then(|m| m.modified().ok())
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();

                        let category =
                            if file_name == "slides.typ" || file_name.ends_with(".slide.typ") {
                                "slide".to_string()
                            } else if ext == "typ" {
                                "typst".to_string()
                            } else if ext == "tex" || ext == "latex" {
                                "latex".to_string()
                            } else if ext == "table"
                                || ext == "db"
                                || ext == "sqlite"
                                || ext == "sqlite3"
                                || ext == "csv"
                            {
                                "table".to_string()
                            } else if ext == "anote"
                                || ext == "note"
                                || ext == "md"
                                || ext == "markdown"
                            {
                                "note".to_string()
                            } else if matches!(
                                ext.as_str(),
                                "py" | "sh" | "bash" | "r" | "rs" | "js" | "ts"
                            ) {
                                "script".to_string()
                            } else if matches!(
                                ext.as_str(),
                                "tsv" | "json" | "png" | "jpg" | "jpeg" | "svg"
                            ) {
                                "asset".to_string()
                            } else {
                                "other".to_string()
                            };

                        let open_url = match category.as_str() {
                            | "slide" | "typst" | "latex" | "script" => {
                                format!(
                                    "/projects/{}/editor?file={}",
                                    project_id,
                                    urlencoding::encode(&rel)
                                )
                            },
                            | "table" => {
                                format!(
                                    "/projects/{}/table?file={}",
                                    project_id,
                                    urlencoding::encode(&rel)
                                )
                            },
                            | "note" => {
                                format!(
                                    "/projects/{}/note?file={}",
                                    project_id,
                                    urlencoding::encode(&rel)
                                )
                            },
                            // "asset"/"other" cover real binary content (PDFs -- typically the
                            // compiled output sitting next to a .tex source, images, audio) that
                            // the text-based editor studio's `read_file` (a `String`) simply
                            // cannot represent -- routing these through `/editor` used to either
                            // error on invalid UTF-8 or silently render mangled bytes as "markdown
                            // source". The raw endpoint serves real bytes with a real
                            // content-type, so the browser can actually display (PDF, image) or
                            // download (anything else) it.
                            | _ => {
                                format!(
                                    "/projects/{}/files/raw?file={}",
                                    project_id,
                                    urlencoding::encode(&rel)
                                )
                            },
                        };

                        let share_info = proj
                            .settings
                            .get("file_shares")
                            .and_then(|fs| fs.get(&rel))
                            .and_then(|val| {
                                serde_json::from_value::<FileShareInfo>(val.clone()).ok()
                            });

                        items.push(ProjectFileItem {
                            path: rel,
                            name: file_name,
                            is_dir: false,
                            size_bytes,
                            extension: ext,
                            category,
                            modified_rfc3339,
                            open_url,
                            share_info,
                        });
                    }
                }
            }
        }

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }

    /// Update file-level sharing configuration (mode: public/specific/private, role: read/review/write)
    pub async fn update_file_share(
        &self,
        project_id: Uuid,
        file_path: &str,
        mode: &str,
        role: &str,
        allowed_users: Vec<String>,
    ) -> WebResult<FileShareInfo> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let mut settings = proj.settings.as_object().cloned().unwrap_or_default();
        let mut file_shares = settings
            .get("file_shares")
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        let token = format!("fsh_{}", uuid::Uuid::new_v4().simple());
        let share_info = FileShareInfo {
            mode: mode.to_string(),
            role: role.to_string(),
            allowed_users,
            token,
        };

        let share_value = serde_json::to_value(&share_info)
            .map_err(|e| WebError::Internal(format!("Failed to serialize share info: {e}")))?;
        file_shares.insert(file_path.to_string(), share_value);
        settings.insert(
            "file_shares".to_string(),
            serde_json::Value::Object(file_shares),
        );

        repo.update_project_settings(project_id, serde_json::Value::Object(settings))
            .await?;
        Ok(share_info)
    }

    /// Retrieve file-level sharing configuration if set
    pub async fn get_file_share(
        &self,
        project_id: Uuid,
        file_path: &str,
    ) -> WebResult<Option<FileShareInfo>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let info = proj
            .settings
            .get("file_shares")
            .and_then(|fs| fs.get(file_path))
            .and_then(|val| serde_json::from_value::<FileShareInfo>(val.clone()).ok());

        Ok(info)
    }

    /// Update project-level sharing configuration (mode: public/specific/private, role: `read_only/read_and_review/read_write_and_review`)
    pub async fn update_project_share_settings(
        &self,
        project_id: Uuid,
        mode: &str,
        role: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let mut settings = proj.settings.as_object().cloned().unwrap_or_default();
        settings.insert("share_mode".to_string(), serde_json::json!(mode));
        settings.insert("share_role".to_string(), serde_json::json!(role));

        repo.update_project_settings(project_id, serde_json::Value::Object(settings))
            .await?;
        Ok(())
    }

    /// Read file content as text
    pub async fn read_file(
        &self,
        project_id: Uuid,
        rel_path: &str,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to read file: {e}")))?;
        Ok(content)
    }

    /// Read file content as raw bytes -- for download/inline-view of binary files (PDFs, images,
    /// SQLite tables, audio) that `read_file`'s `String` return type can't represent. Same
    /// path-traversal guard as every other file accessor here.
    pub async fn read_file_bytes(
        &self,
        project_id: Uuid,
        rel_path: &str,
    ) -> WebResult<Vec<u8>> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        tokio::fs::read(&full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to read file: {e}")))
    }

    /// Write text content to file and record VCS snapshot
    pub async fn write_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        content: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                WebError::Internal(format!("Failed to create parent directory: {e}"))
            })?;
        }

        tokio::fs::write(&full_path, content.as_bytes())
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write file: {e}")))?;

        // FastCDC snapshot on save
        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Update {clean} (by user {user_suffix})"))?;

        Ok(())
    }

    /// Create new file with starter template
    pub async fn create_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        template: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if full_path.exists() {
            return Err(WebError::Conflict(format!(
                "File '{rel_path}' already exists"
            )));
        }

        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                WebError::Internal(format!("Failed to create parent directory: {e}"))
            })?;
        }

        let ext = std::path::Path::new(clean)
            .extension()
            .and_then(|e| e.to_str());
        let is_table_ext = ext.is_some_and(|e| {
            e.eq_ignore_ascii_case("table")
                || e.eq_ignore_ascii_case("sqlite")
                || e.eq_ignore_ascii_case("db")
        });

        if template == "table" || is_table_ext {
            let conn = rusqlite::Connection::open(&full_path).map_err(|e| {
                WebError::Internal(format!("Failed to initialize SQLite table: {e}"))
            })?;
            conn.execute_batch(
                r"CREATE TABLE records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_name TEXT NOT NULL,
    category TEXT,
    value REAL,
    status TEXT,
    notes TEXT
);
INSERT INTO records (item_name, category, value, status, notes) VALUES
('Sample Data Alpha', 'Experimental', 98.4, 'verified', 'Initial measurement'),
('Sample Data Beta', 'Control', 45.2, 'pending', 'Awaiting cross-validation');",
            )
            .map_err(|e| WebError::Internal(format!("Failed to seed table: {e}")))?;
        } else {
            // A new "slide" (cargo-slide) file imports `theme.typ`/`slide.typ` -- provision them
            // into this project if they're not already there (previously only ever written by
            // the demo seeder), or the file this same match arm is about to create would fail its
            // very first compile.
            if template == "slide" {
                crate::services::cargo_slide_helpers::ensure_cargo_slide_helpers(
                    &proj.storage_path,
                )
                .await
                .map_err(|e| {
                    WebError::Internal(format!("Failed to provision slide.typ/theme.typ: {e}"))
                })?;
            }
            let initial_content = match template {
                | "typst" => {
                    r#"#set page(paper: "a4", margin: 2.5cm)
#set text(font: "Linux Libertine", size: 11pt)

= New Typst Document

== Introduction
This document is authored inside APICH Unified Research Cloud.
"#
                },
                // Matches `slide.typ`'s real exported API (`title-slide(title:, subtitle:,
                // author:, institution:, date:)`, `slide(title:, body)`) -- a previous version of
                // this called a `slide-theme` function and a `step[...]` block that don't exist
                // in that file at all, which failed every single new slide deck's very first
                // compile with "unknown variable: slide-theme" before a single edit was made.
                //
                // That earlier fix dropped the `slide-theme` call entirely rather than confirming
                // theme.typ's own (correct) API for it -- `#show: slide-theme.with(aspect-ratio:,
                // theme:)`, invoked once at the top of the document -- leaving every new slide
                // deck with no page-size setup at all, silently falling back to Typst's default
                // page (not a 16:9 slide) and, confirmed live, failing cargo-slide's own overflow
                // check on virtually any real content as a result. Restored here to match
                // theme.typ's real, current API and cargo-slide's own reference template
                // (crates/slide-theme/typst/template.typ upstream) exactly.
                | "slide" => {
                    r#"#import "theme.typ": *
#import "slide.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9",
  theme: "dark"
)

#title-slide(
  title: "New Presentation",
  subtitle: "APICH Slide Deck",
  author: "Researcher",
  institution: "",
  date: "",
)

#slide(title: "Overview")[
  - First bullet point of our presentation
  - Second critical takeaway
]
"#
                },
                | "latex" => {
                    r"\documentclass{article}
\usepackage[utf8]{inputenc}

\title{New Research Report}
\author{APICH Research Group}
\date{\today}

\begin{document}
\maketitle

\section{Introduction}
Begin drafting your LaTeX manuscript here.

\end{document}
"
                },
                | "note" => {
                    r#"---
title: "Research Log"
tags: ["experiment", "notes"]
status: "in_progress"
---

# Research Log

## Key Observations
- [ ] Initial configuration of benchmark parameters #setup
- [ ] Run evaluation suite on cluster

## Cross References
- [[paper.typ]]
"#
                },
                | _ => "# New File\n",
            };

            tokio::fs::write(&full_path, initial_content.as_bytes())
                .await
                .map_err(|e| WebError::Internal(format!("Failed to write new file: {e}")))?;
        }

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Create {clean} (by user {user_suffix})"))?;

        Ok(())
    }

    /// Same path-safety/existence/directory-creation/VCS-snapshot behavior as `create_file`, but
    /// with the new file's content given directly rather than picked from `create_file`'s fixed
    /// set of builtin starter templates -- used by the "start from a Template Library template"
    /// path (`ui::template_handlers`), which needed a *published* template's content (arbitrary
    /// text, not one of the 8 hardcoded kinds `create_file` knows about).
    pub async fn create_file_with_content(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        content: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if full_path.exists() {
            return Err(WebError::Conflict(format!(
                "File '{rel_path}' already exists"
            )));
        }

        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                WebError::Internal(format!("Failed to create parent directory: {e}"))
            })?;
        }

        tokio::fs::write(&full_path, content.as_bytes())
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write new file: {e}")))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!(
            "Create {clean} from template (by user {user_suffix})"
        ))?;

        Ok(())
    }

    /// Delete file from project workspace
    pub async fn delete_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean = rel_path.trim().trim_start_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid file path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if !full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        tokio::fs::remove_file(&full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to delete file: {e}")))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Delete {clean} (by user {user_suffix})"))?;

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
pub struct FileShareInfo {
    pub mode: String, // "public", "specific", "private"
    pub role: String, // "read", "review", "write"
    pub allowed_users: Vec<String>,
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectFileItem {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub extension: String,
    pub category: String,
    pub modified_rfc3339: Option<String>,
    pub open_url: String,
    pub share_info: Option<FileShareInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConflictFileView {
    pub path: String,
    pub full_content: String,
    pub ours_snippet: String,
    pub theirs_snippet: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MergeSummary {
    pub auto_merged_count: usize,
    pub conflicts_count: usize,
    pub conflicts: Vec<ConflictFileView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitStatusView {
    pub initialized: bool,
    pub remote_url: Option<String>,
    pub remotes: Vec<(String, String)>,
    pub branches: Vec<String>,
    pub current_branch: Option<String>,
}

pub fn resolve_conflict_content(
    raw: &str,
    choice: &str,
) -> String {
    let mut result = String::new();
    let mut in_conflict = false;
    let mut in_ours = false;
    let mut in_theirs = false;
    let mut ours_buf = String::new();
    let mut theirs_buf = String::new();

    for line in raw.lines() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            in_ours = true;
            in_theirs = false;
            ours_buf.clear();
            theirs_buf.clear();
        } else if in_conflict && line.starts_with("=======") {
            in_ours = false;
            in_theirs = true;
        } else if in_conflict && line.starts_with(">>>>>>>") {
            in_conflict = false;
            in_ours = false;
            in_theirs = false;
            if choice == "theirs" {
                result.push_str(&theirs_buf);
            } else {
                result.push_str(&ours_buf);
            }
        } else if in_ours {
            ours_buf.push_str(line);
            ours_buf.push('\n');
        } else if in_theirs {
            theirs_buf.push_str(line);
            theirs_buf.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Map a stable profile id (as used in the ignore-config UI/forms) to an `IgnoreProfile` variant.
pub fn ignore_profile_from_id(id: &str) -> Option<apich_vcs::IgnoreProfile> {
    apich_vcs::IgnoreProfile::all()
        .iter()
        .copied()
        .find(|p| p.id() == id)
}

/// Restrict raw ignore-file editing to the two known file names, rejecting any path traversal.
fn validate_ignore_file_name(file_name: &str) -> WebResult<&'static str> {
    match file_name {
        | ".gitignore" => Ok(".gitignore"),
        | ".apichignore" => Ok(".apichignore"),
        | _ => {
            Err(WebError::BadRequest(format!(
                "Unsupported ignore file: {file_name}"
            )))
        },
    }
}

fn parse_conflict_snippets(content: &str) -> (String, String) {
    let mut ours = String::new();
    let mut theirs = String::new();
    let mut in_ours = false;
    let mut in_theirs = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_ours = true;
            in_theirs = false;
        } else if in_ours && line.starts_with("=======") {
            in_ours = false;
            in_theirs = true;
        } else if in_theirs && line.starts_with(">>>>>>>") {
            in_ours = false;
            in_theirs = false;
        } else if in_ours {
            ours.push_str(line);
            ours.push('\n');
        } else if in_theirs {
            theirs.push_str(line);
            theirs.push('\n');
        }
    }

    (ours.trim().to_string(), theirs.trim().to_string())
}
