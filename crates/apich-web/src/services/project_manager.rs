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
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
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
    /// The musl targets are still valid to request (and still built), but produce a fully static
    /// binary that can never actually open the presentation window: `minifb` loads its X11/Wayland
    /// backend via `dlopen()` at runtime, and a fully static binary has no dynamic linker for that
    /// call to work through at all -- musl's own libc stub for it unconditionally fails with
    /// "Dynamic loading not supported", confirmed live, regardless of what's installed on the
    /// machine that runs it. Kept in the allow-list for a headless consumer of the deck's data
    /// (not this UI's own picker default -- see `apich_islands::SlideBuildIsland`'s doc comment).
    pub const SLIDE_BUILD_TARGETS: &'static [(&'static str, &'static str)] = &[
        ("host", "This server's own platform (Linux x86_64, native)"),
        ("aarch64-unknown-linux-gnu", "Linux ARM64 (glibc)"),
        ("x86_64-pc-windows-gnu", "Windows x86_64"),
        ("aarch64-pc-windows-gnullvm", "Windows ARM64"),
        ("x86_64-unknown-linux-musl", "Linux x86_64 (musl, static -- cannot open a window)"),
        ("aarch64-unknown-linux-musl", "Linux ARM64 (musl, static -- cannot open a window)"),
        ("slide_package", "Universal .slide Package (for slide-viewer)"),
        ("wasm_bundle", "WebAssembly Web Bundle (.zip)"),
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
        // Enforce command security policy to block host server inspection (e.g. `df -H`, `lscpu`, `ip a`)
        if let Err(violation) = crate::services::command_security::CommandSecurityGuard::validate_command(command) {
            return Ok(format!("{}\n", violation.to_terminal_message()));
        }

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

        if !args.trim().is_empty() {
            if let Err(violation) = crate::services::command_security::CommandSecurityGuard::validate_command(args) {
                return Ok(crate::services::document_renderer::ScriptRunResult {
                    success: false,
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: format!("{}\n", violation.to_terminal_message()),
                    execution_time_ms: 0,
                    output_images: Vec::new(),
                });
            }
        }

        if ext == "sh" || ext == "bash" {
            if let Ok(content) = tokio::fs::read_to_string(&full_path).await {
                if let Err(violation) = crate::services::command_security::CommandSecurityGuard::validate_command(&content) {
                    return Ok(crate::services::document_renderer::ScriptRunResult {
                        success: false,
                        exit_code: Some(1),
                        stdout: String::new(),
                        stderr: format!("{}\n", violation.to_terminal_message()),
                        execution_time_ms: 0,
                        output_images: Vec::new(),
                    });
                }
            }
        }

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

    /// Compile multiple LaTeX files into a single unified PDF inside the project's sandbox.
    pub async fn compile_multiple_latex_in_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        ordered_files: &[String],
        engine: &str,
        add_pagebreaks: bool,
    ) -> WebResult<Result<Vec<u8>, String>> {
        if ordered_files.is_empty() {
            return Ok(Err("No LaTeX files specified for multi-file rendering".to_string()));
        }

        let container = self.ensure_agent_container(project_id, user_id).await?;
        let run_id = uuid::Uuid::new_v4().to_string();

        // Read all selected files from container workspace
        let mut file_contents: Vec<(String, String)> = Vec::new();
        for f in ordered_files {
            match container.read_file_str(f).await {
                Ok(content) => file_contents.push((f.clone(), content)),
                Err(e) => return Ok(Err(format!("Could not read LaTeX file '{f}': {e}"))),
            }
        }

        let combined_tex = Self::generate_combined_latex_source(&file_contents, add_pagebreaks);

        let master_stem = format!("_multi_{run_id}");
        let master_tex_name = format!("{master_stem}.tex");
        let master_pdf_name = format!("{master_stem}.pdf");
        let master_log_name = format!("{master_stem}.log");

        let proj = self
            .db
            .repository()
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let master_full_path = PathBuf::from(&proj.storage_path).join(&master_tex_name);
        tokio::fs::write(&master_full_path, &combined_tex).await.map_err(|e| {
            WebError::Internal(format!("Failed to write master LaTeX file: {e}"))
        })?;

        let clean_engine = match engine {
            "xelatex" => "xelatex",
            "lualatex" => "lualatex",
            _ => "pdflatex",
        };

        let cmd = vec![
            clean_engine.to_string(),
            "-interaction=nonstopmode".to_string(),
            "-halt-on-error".to_string(),
            master_tex_name.clone(),
        ];

        let _ = container.exec(cmd.clone()).await?;
        let result = container.exec(cmd).await?;

        // Cleanup temporary master .tex and auxiliary files
        let _ = tokio::fs::remove_file(&master_full_path).await;
        let _ = container
            .exec([
                "rm",
                "-f",
                &format!("{master_stem}.aux"),
                &format!("{master_stem}.out"),
                &format!("{master_stem}.toc"),
            ])
            .await;

        if !result.success() {
            // Fallback: compile each standalone file and combine using pdfpages
            let mut compiled_pdfs = Vec::new();
            let mut compile_err = None;

            for f in ordered_files {
                let stem = std::path::Path::new(f)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("doc");
                let sub_cmd = vec![
                    clean_engine.to_string(),
                    "-interaction=nonstopmode".to_string(),
                    "-halt-on-error".to_string(),
                    f.clone(),
                ];
                let _ = container.exec(sub_cmd.clone()).await;
                let sub_res = container.exec(sub_cmd).await?;
                if sub_res.success() {
                    compiled_pdfs.push(format!("{stem}.pdf"));
                } else {
                    let log = container
                        .read_file_str(&format!("{stem}.log"))
                        .await
                        .unwrap_or_else(|_| {
                            format!("{}{}", sub_res.stdout_lossy(), sub_res.stderr_lossy())
                        });
                    compile_err = Some(format!("Failed to compile '{f}':\n{log}"));
                    break;
                }
            }

            if let Some(err) = compile_err {
                let _ = container.exec(["rm", "-f", &master_log_name, &master_pdf_name]).await;
                return Ok(Err(err));
            }

            let mut pdfpages_wrapper = String::from(
                "\\documentclass{article}\n\\usepackage{pdfpages}\n\\begin{document}\n",
            );
            for pdf_file in &compiled_pdfs {
                pdfpages_wrapper.push_str(&format!("\\includepdf[pages=-]{{{pdf_file}}}\n"));
            }
            pdfpages_wrapper.push_str("\\end{document}\n");

            let wrapper_tex_name = format!("_wrap_{run_id}.tex");
            let wrapper_full_path = PathBuf::from(&proj.storage_path).join(&wrapper_tex_name);
            let _ = tokio::fs::write(&wrapper_full_path, &pdfpages_wrapper).await;

            let wrap_cmd = vec![
                "pdflatex".to_string(),
                "-interaction=nonstopmode".to_string(),
                "-halt-on-error".to_string(),
                wrapper_tex_name.clone(),
            ];
            let wrap_res = container.exec(wrap_cmd).await?;
            let _ = tokio::fs::remove_file(&wrapper_full_path).await;
            let _ = container
                .exec([
                    "rm",
                    "-f",
                    &format!("_wrap_{run_id}.aux"),
                    &format!("_wrap_{run_id}.log"),
                ])
                .await;

            let wrapper_pdf_name = format!("_wrap_{run_id}.pdf");
            if wrap_res.success() {
                if let Ok(bytes) = container.read_file(&wrapper_pdf_name).await {
                    let _ = container.exec(["rm", "-f", &wrapper_pdf_name]).await;
                    return Ok(Ok(bytes));
                }
            }

            let log = container
                .read_file_str(&master_log_name)
                .await
                .unwrap_or_else(|_| format!("{}{}", result.stdout_lossy(), result.stderr_lossy()));
            let _ = container.exec(["rm", "-f", &master_log_name, &master_pdf_name]).await;
            return Ok(Err(log));
        }

        let pdf_bytes = match container.read_file(&master_pdf_name).await {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(format!(
                "LaTeX reported success but output PDF could not be read: {e}"
            )),
        };
        let _ = container.exec(["rm", "-f", &master_log_name, &master_pdf_name]).await;

        Ok(pdf_bytes)
    }

    /// Generate combined source code for multiple LaTeX files
    pub fn generate_combined_latex_source(
        file_contents: &[(String, String)],
        add_pagebreaks: bool,
    ) -> String {
        let mut preamble = String::new();
        let mut bodies = Vec::new();

        for (name, content) in file_contents {
            if let Some(doc_start) = content.find(r"\begin{document}") {
                if preamble.is_empty() {
                    preamble = content[..doc_start].to_string();
                }
                let body_start = doc_start + r"\begin{document}".len();
                let body_end = content.find(r"\end{document}").unwrap_or(content.len());
                bodies.push((name, &content[body_start..body_end]));
            } else {
                bodies.push((name, content.as_str()));
            }
        }

        if preamble.is_empty() {
            preamble = String::from(
                "\\documentclass[11pt]{article}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amsmath,amssymb}\n\\usepackage{graphicx}\n\\usepackage{hyperref}\n",
            );
        }

        let mut combined_tex = preamble;
        combined_tex.push_str("\n\\begin{document}\n");
        for (idx, (name, body)) in bodies.iter().enumerate() {
            if idx > 0 && add_pagebreaks {
                combined_tex.push_str("\n\\clearpage\n");
            }
            combined_tex.push_str(&format!("\n% === File: {name} ===\n"));
            combined_tex.push_str(body);
            combined_tex.push('\n');
        }
        combined_tex.push_str("\n\\end{document}\n");
        combined_tex
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
        let download_stem = std::path::Path::new(rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("presentation")
            .to_string();

        let (out_rel_path, download_name, cmd) = if target == "slide_package" {
            let out = format!(".apich_slide_build_{run_id}.slide");
            let name = format!("{download_stem}.slide");
            let command = vec![
                "cargo".to_string(),
                "slide".to_string(),
                "--log-format".to_string(),
                "json".to_string(),
                "pack".to_string(),
                rel_path.to_string(),
                "-o".to_string(),
                out.clone(),
            ];
            (out, name, command)
        } else if target == "wasm_bundle" {
            let out = format!(".apich_slide_build_{run_id}.zip");
            let name = format!("{download_stem}-web.zip");
            let tmp_dir = format!(".apich_wasm_tmp_{run_id}");
            let script = format!(
                "cargo slide --log-format json build '{rel_path}' --format wasm -o '{tmp_dir}' && python3 -c \"import shutil; shutil.make_archive('.apich_slide_build_{run_id}', 'zip', '{tmp_dir}')\" && rm -rf '{tmp_dir}'"
            );
            let command = vec!["sh".to_string(), "-c".to_string(), script];
            (out, name, command)
        } else {
            let is_windows_target = target.contains("windows");
            let name = if is_windows_target {
                format!("{download_stem}-presentation.exe")
            } else {
                format!("{download_stem}-presentation")
            };
            let out = format!(".apich_slide_build_{run_id}");
            let mut command = vec![
                "cargo".to_string(),
                "slide".to_string(),
                "--log-format".to_string(),
                "json".to_string(),
                "build".to_string(),
                rel_path.to_string(),
                "-o".to_string(),
                out.clone(),
            ];
            if target != "host" {
                command.push("--target".to_string());
                command.push(target.to_string());
            }
            (out, name, command)
        };

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

    /// Exports the slide presentation into `deck.json` inside the sandbox container for live
    /// browser presentation playback via `slide-web`.
    pub async fn get_slide_deck_json(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
    ) -> WebResult<String> {
        let container = self.ensure_agent_container(project_id, user_id).await?;
        let run_id = uuid::Uuid::new_v4().simple().to_string();
        let tmp_dir = format!(".apich_deck_preview_{run_id}");
        let clean_path = rel_path.trim_start_matches('/');
        let cmd = vec![
            "cargo".to_string(),
            "slide".to_string(),
            "export".to_string(),
            clean_path.to_string(),
            "--format".to_string(),
            "wasm".to_string(),
            "-o".to_string(),
            tmp_dir.clone(),
        ];
        let opts = apich_sandbox::ExecOptions::new(cmd).timeout(std::time::Duration::from_secs(60));
        let res = container.exec_with_options(opts).await?;
        if res.exit_code != 0 {
            let _ = container.exec(["rm", "-rf", &tmp_dir]).await;
            let err_bytes = if res.stderr.is_empty() {
                res.stdout
            } else {
                res.stderr
            };
            let err = String::from_utf8_lossy(&err_bytes);
            return Err(crate::error::WebError::BadRequest(format!(
                "Failed to compile slide presentation for browser: {err}"
            )));
        }
        let deck_path = format!("{tmp_dir}/deck.json");
        let read_result = container.read_file(&deck_path).await;
        let _ = container.exec(["rm", "-rf", &tmp_dir]).await;
        let bytes = read_result.map_err(|e| {
            crate::error::WebError::Internal(format!("Failed to read compiled deck.json: {e}"))
        })?;
        String::from_utf8(bytes).map_err(|e| crate::error::WebError::Internal(e.to_string()))
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

    /// Retrieve detailed information about a snapshot, including added, modified, and removed files
    pub async fn get_snapshot_details(
        &self,
        project_id: Uuid,
        snapshot_id: Uuid,
    ) -> WebResult<SnapshotDetailsView> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let snap = vcs.get_snapshot(snapshot_id)?;
        let snap_tree = vcs.get_tree(&snap.tree_hash)?;
        let mut all_files: Vec<String> = snap_tree.entries.keys().cloned().collect();
        all_files.sort();

        let (added_files, modified_files, removed_files) = if let Some(parent_id) = snap.parent_snapshot_id {
            vcs.diff_snapshots(parent_id, snapshot_id)
                .unwrap_or_else(|_| (all_files.clone(), Vec::new(), Vec::new()))
        } else {
            (all_files.clone(), Vec::new(), Vec::new())
        };

        Ok(SnapshotDetailsView {
            id: snap.id,
            parent_snapshot_id: snap.parent_snapshot_id,
            created_at: snap.created_at,
            message: snap.message,
            author: snap.author,
            is_milestone: snap.is_milestone,
            milestone_name: snap.milestone_name,
            tree_hash: snap.tree_hash,
            gpg_signature: snap.gpg_signature,
            added_files,
            modified_files,
            removed_files,
            all_files,
        })
    }

    /// Compute line-by-line unified diff of a file in a given snapshot compared to its parent snapshot
    pub async fn get_file_diff_in_snapshot(
        &self,
        project_id: Uuid,
        snapshot_id: Uuid,
        rel_path: &str,
    ) -> WebResult<FileDiffResult> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let snap = vcs.get_snapshot(snapshot_id)?;

        let new_bytes = vcs.read_file_content(Some(snapshot_id), rel_path).ok();
        let old_bytes = if let Some(parent_id) = snap.parent_snapshot_id {
            vcs.read_file_content(Some(parent_id), rel_path).ok()
        } else {
            None
        };

        let status = match (&old_bytes, &new_bytes) {
            | (None, Some(_)) => "added".to_string(),
            | (Some(_), None) => "removed".to_string(),
            | (Some(o), Some(n)) if o == n => "unchanged".to_string(),
            | (Some(_), Some(_)) => "modified".to_string(),
            | (None, None) => {
                return Err(WebError::NotFound(format!("File '{rel_path}' not found in snapshot or parent")));
            },
        };

        let is_binary = match (&old_bytes, &new_bytes) {
            | (_, Some(bytes)) | (Some(bytes), None) => {
                bytes.iter().take(1024).any(|&b| b == 0)
            },
            | (None, None) => false,
        };

        if is_binary {
            return Ok(FileDiffResult {
                file_path: rel_path.to_string(),
                is_binary: true,
                status,
                old_content: None,
                new_content: None,
                diff_lines: Vec::new(),
            });
        }

        let old_str = old_bytes.as_ref().and_then(|b| std::str::from_utf8(b).ok());
        let new_str = new_bytes.as_ref().and_then(|b| std::str::from_utf8(b).ok());

        let diff_lines = compute_unified_diff(old_str.unwrap_or(""), new_str.unwrap_or(""));

        Ok(FileDiffResult {
            file_path: rel_path.to_string(),
            is_binary: false,
            status,
            old_content: old_str.map(std::string::ToString::to_string),
            new_content: new_str.map(std::string::ToString::to_string),
            diff_lines,
        })
    }

    /// Restore a specific file from a historical snapshot, either overwriting the current file
    /// or writing it out to a brand new file path.
    pub async fn restore_file_from_snapshot(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        snapshot_id: Uuid,
        source_file: &str,
        target_file: &str,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let file_bytes = vcs
            .read_file_content(Some(snapshot_id), source_file)
            .map_err(|e| WebError::NotFound(format!("Failed to read '{source_file}' from snapshot: {e}")))?;

        let short_id = if snapshot_id.to_string().len() >= 8 {
            snapshot_id.to_string()[..8].to_string()
        } else {
            snapshot_id.to_string()
        };
        let is_overwrite = source_file == target_file;
        let snap_msg = if is_overwrite {
            format!("Restored '{target_file}' from snapshot {short_id}")
        } else {
            format!("Restored '{source_file}' as '{target_file}' from snapshot {short_id}")
        };

        self.write_file_bytes(project_id, user_id, target_file, &file_bytes, Some(&snap_msg)).await?;

        Ok(snap_msg)
    }

    /// Revert the entire project working copy to a specific snapshot in history
    pub async fn revert_project_to_snapshot(
        &self,
        project_id: Uuid,
        _user_id: Uuid,
        snapshot_id: Uuid,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        vcs.revert_to(snapshot_id)?;

        let short_id = if snapshot_id.to_string().len() >= 8 {
            snapshot_id.to_string()[..8].to_string()
        } else {
            snapshot_id.to_string()
        };
        let msg = format!("Reverted entire project to snapshot {short_id}");
        Ok(msg)
    }

    /// Write arbitrary bytes to file and record VCS snapshot
    pub async fn write_file_bytes(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        bytes: &[u8],
        snapshot_msg: Option<&str>,
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

        tokio::fs::write(&full_path, bytes)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write file: {e}")))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let default_msg = format!("Update {clean} (by user {user_suffix})");
        let msg = snapshot_msg.unwrap_or(&default_msg);
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(msg)?;

        Ok(())
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

    /// Helper to find a user's GitHub access token if remote is github.com
    async fn resolve_remote_auth_token(
        &self,
        user_id: Option<Uuid>,
        vcs: &ProjectVcs,
        remote: &str,
    ) -> Option<String> {
        let uid = user_id?;
        let remotes = vcs.git_remotes().ok()?;
        let (_, url) = remotes.iter().find(|(name, _)| name == remote)?;
        if url.contains("github.com") {
            let repo = self.db.repository();
            let cred = repo.get_user_git_credential(uid, "github").await.ok().flatten()?;
            return Some(cred.access_token);
        }
        None
    }

    /// Fetch from a Git remote without merging.
    pub async fn git_fetch(
        &self,
        project_id: Uuid,
        remote: &str,
        user_id: Option<Uuid>,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let auth_token = self.resolve_remote_auth_token(user_id, &vcs, remote).await;
        vcs.git_fetch_with_auth(remote, auth_token.as_deref())?;
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
        user_id: Option<Uuid>,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let storage_path = std::path::PathBuf::from(&proj.storage_path);
        crate::services::git_server::ensure_working_exported_to_git(&storage_path)
            .await
            .map_err(|e| WebError::Internal(e.to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let target_branch = if branch.trim().is_empty() {
            vcs.current_branch()?.unwrap_or_else(|| "main".to_string())
        } else {
            branch.trim().to_string()
        };
        let auth_token = self.resolve_remote_auth_token(user_id, &vcs, remote).await;
        vcs.git_push_with_auth(remote, &target_branch, auth_token.as_deref())?;
        // Keep internal bare mirror in sync
        let _ = crate::services::git_server::sync_mirror_from_working(&storage_path).await;
        Ok(())
    }

    /// Pull from a Git remote into the current branch, then re-snapshot the result.
    pub async fn git_pull(
        &self,
        project_id: Uuid,
        remote: &str,
        branch: &str,
        user_id: Option<Uuid>,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let target_branch = if branch.trim().is_empty() {
            vcs.current_branch()?.unwrap_or_else(|| "main".to_string())
        } else {
            branch.trim().to_string()
        };
        let auth_token = self.resolve_remote_auth_token(user_id, &vcs, remote).await;
        vcs.git_pull_with_auth(remote, &target_branch, auth_token.as_deref())?;
        let _ = vcs.snapshot_if_changed(format!("Pulled from {remote} {target_branch}"));
        // Keep internal bare mirror in sync with newly pulled state
        let storage_path = std::path::PathBuf::from(&proj.storage_path);
        let _ = crate::services::git_server::sync_mirror_from_working(&storage_path).await;
        Ok(())
    }

    /// Retrieve stored Git credential for a user and provider.
    pub async fn get_user_git_credential(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> WebResult<Option<apich_db::UserGitCredential>> {
        let repo = self.db.repository();
        Ok(repo.get_user_git_credential(user_id, provider).await?)
    }

    /// Save or update a user's Git credential.
    pub async fn upsert_user_git_credential(
        &self,
        user_id: Uuid,
        dto: &apich_db::UpsertGitCredentialDto,
    ) -> WebResult<apich_db::UserGitCredential> {
        let repo = self.db.repository();
        Ok(repo.upsert_user_git_credential(user_id, dto).await?)
    }

    /// Disconnect / delete a user's Git credential.
    pub async fn delete_user_git_credential(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> WebResult<bool> {
        let repo = self.db.repository();
        Ok(repo.delete_user_git_credential(user_id, provider).await?)
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
        // Every subdirectory walked (relative path), so a folder with no *files* anywhere under
        // it -- e.g. one just created via `create_folder`, still holding only its `.gitkeep`
        // placeholder -- can still get a synthetic row below instead of being silently invisible
        // (this loop only ever emits `ProjectFileItem`s for real files as it walks).
        let mut dirs_seen: Vec<String> = Vec::new();

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
                        let rel = path
                            .strip_prefix(&proj_root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/");
                        dirs_seen.push(rel);
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
                                // Text-ish data formats still better served raw than force-fit
                                // into the plain-text editor.
                                "tsv" | "json"
                                // Images
                                | "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "svg"
                                | "avif" | "tiff"
                                // Audio
                                | "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "opus"
                                // Video
                                | "mp4" | "webm" | "mov" | "avi" | "mkv" | "m4v"
                                // Documents/archives/fonts/generic binaries
                                | "pdf" | "zip" | "tar" | "gz" | "7z" | "rar" | "woff" | "woff2"
                                | "ttf" | "otf" | "wasm" | "exe" | "dll" | "so" | "bin"
                            ) {
                                "asset".to_string()
                            } else {
                                "other".to_string()
                            };

                        let open_url = match category.as_str() {
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
                            // "asset" is the only category covering real binary content (images,
                            // audio, video, PDFs, archives, fonts) that the text-based editor
                            // studio's `read_file` (a `String`) simply cannot represent -- routing
                            // these through `/editor` used to either error on invalid UTF-8 or
                            // silently render mangled bytes as "markdown source". The raw endpoint
                            // serves real bytes with a real content-type, so the browser can
                            // actually display (PDF, image) or download (anything else) it.
                            | "asset" => {
                                format!(
                                    "/projects/{}/files/raw?file={}",
                                    project_id,
                                    urlencoding::encode(&rel)
                                )
                            },
                            // Everything else opens in the in-app editor: "slide"/"typst"/
                            // "latex"/"script" each have a dedicated mode there, and "other" --
                            // an extension this app doesn't have dedicated editor or
                            // recognized-binary handling for (.txt, .yaml, .toml, a file with no
                            // extension at all, ...) -- falls back to the same plain-text editor
                            // branch (`document_editor.rs`'s markdown fallback). "other" used to
                            // be lumped in with "asset" and forced through the raw/download
                            // endpoint instead -- confirmed live: creating a plain new file (an
                            // extension not matching any case above, e.g. via "+ New File") made
                            // clicking it download instead of open, even though it's ordinary
                            // UTF-8 text `create_file` itself just wrote.
                            | _ => {
                                format!(
                                    "/projects/{}/editor?file={}",
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

        // A directory with no file anywhere under it (a freshly `create_folder`'d one, still
        // holding only its hidden `.gitkeep`) has no row yet -- add a synthetic one so it's
        // actually visible (and usable as an upload destination) instead of looking like folder
        // creation silently did nothing.
        for dir_rel in dirs_seen {
            let has_file_under = items
                .iter()
                .any(|it| it.path.starts_with(&format!("{dir_rel}/")));
            if has_file_under {
                continue;
            }
            let name = dir_rel
                .rsplit('/')
                .next()
                .unwrap_or(&dir_rel)
                .to_string();
            items.push(ProjectFileItem {
                path: dir_rel,
                name,
                is_dir: true,
                size_bytes: 0,
                extension: String::new(),
                category: "folder".to_string(),
                modified_rfc3339: None,
                open_url: String::new(),
                share_info: None,
            });
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

        let existing_token = file_shares
            .get(file_path)
            .and_then(|val| serde_json::from_value::<FileShareInfo>(val.clone()).ok())
            .map(|s| s.token)
            .filter(|t| !t.is_empty());
        let token = existing_token.unwrap_or_else(|| format!("fsh_{}", uuid::Uuid::new_v4().simple()));
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

    /// Retrieve or generate a sharing token for a file.
    pub async fn ensure_file_share_token(
        &self,
        project_id: Uuid,
        file_path: &str,
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

        if let Some(existing) = file_shares
            .get(file_path)
            .and_then(|val| serde_json::from_value::<FileShareInfo>(val.clone()).ok())
        {
            if !existing.token.is_empty() {
                return Ok(existing);
            }
        }

        let token = format!("fsh_{}", uuid::Uuid::new_v4().simple());
        let share_info = FileShareInfo {
            mode: "private".to_string(),
            role: "read".to_string(),
            allowed_users: Vec::new(),
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

    /// Update project-level sharing configuration (mode: public/specific/private, role: `read_only/read_and_review/read_write_and_review`)
    /// Renames a project. Only the display name and description change -- the slug (and with it
    /// every existing `/projects/<slug>` link) and the on-disk workspace directory deliberately
    /// stay as they are, so renaming can never strand a bookmark or orphan a project's files.
    /// Mainly for the quick-start projects, which get an auto-generated timestamped name the
    /// user will usually want to replace with something meaningful.
    pub async fn rename_project(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> WebResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(WebError::BadRequest(
                "Project name cannot be empty".to_string(),
            ));
        }
        let repo = self.db.repository();
        repo.get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;
        repo.update_project_name(project_id, name, description)
            .await?;
        Ok(())
    }

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
        overwrite: bool,
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
        let existed = full_path.exists();
        if existed && !overwrite {
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
            // if they're not already there (previously only ever written by the demo seeder), or
            // the file this same match arm is about to create would fail its very first compile.
            //
            // Provisioned next to the new file itself, NOT at the project root: Typst resolves a
            // bare relative import like `#import "theme.typ"` against the *importing file's own*
            // directory (the same resolution rule `DocumentRenderer::compile_typst` documents for
            // its annotated-copy placement). Creating a slide inside a subfolder therefore used
            // to fail its first compile with "file not found ... /<subfolder>/theme.typ" -- the
            // helpers existed, just one directory up where this file's import could never see
            // them.
            if template == "slide" {
                let helper_dir = full_path
                    .parent()
                    .map_or_else(|| PathBuf::from(&proj.storage_path), Path::to_path_buf);
                crate::services::cargo_slide_helpers::ensure_cargo_slide_helpers(&helper_dir)
                    .await
                    .map_err(|e| {
                        WebError::Internal(format!("Failed to provision slide.typ/theme.typ: {e}"))
                    })?;
            }
            let initial_content = match template {
                | "empty" => "",
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
                | "script" | "script_python" => {
                    r#"#!/usr/bin/env python3
"""New analysis script.

Runs inside this project's sandbox container (which has Python, R, and Rust
installed) via the editor's Run Script console, not on the web server itself.
Any file this writes -- a saved plot, a generated CSV -- lands in the project
workspace and shows up in the Files tab.
"""

import numpy as np


def main() -> None:
    data = np.array([94.2, 88.5, 102.3, 79.8, 91.4, 86.2])
    print(f"n       = {data.size}")
    print(f"mean    = {data.mean():.2f}")
    print(f"std dev = {data.std(ddof=1):.2f}")


if __name__ == "__main__":
    main()
"#
                },
                | "script_r" => {
                    r#"#!/usr/bin/env Rscript
# New R statistical analysis script.
#
# Runs inside this project's sandbox container via the editor's Run Script console.
# Any generated plots (e.g. ggsave("plot.png")) will appear in your project Files.

cat("=== R Statistical Analysis ===\n")
data <- c(94.2, 88.5, 102.3, 79.8, 91.4, 86.2)
cat(sprintf("n       = %d\n", length(data)))
cat(sprintf("mean    = %.2f\n", mean(data)))
cat(sprintf("std dev = %.2f\n", sd(data)))
cat(sprintf("median  = %.2f\n", median(data)))
"#
                },
                | "script_rust" => {
                    r#"// New Rust computational analysis script.
//
// Runs inside this project's sandbox container via the editor's Run Script console.
// Automatically compiled with rustc and executed directly in your project workspace.

fn main() {
    println!("=== Rust Computational Analysis ===");
    let data: [f64; 6] = [94.2, 88.5, 102.3, 79.8, 91.4, 86.2];
    let n = data.len();
    let sum: f64 = data.iter().sum();
    let mean = sum / n as f64;
    let variance: f64 = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let std_dev = variance.sqrt();

    println!("n       = {}", n);
    println!("mean    = {:.2}", mean);
    println!("std dev = {:.2}", std_dev);
}
"#
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
        let action_verb = if existed { "Overwrite" } else { "Create" };
        let _ = vcs.snapshot_if_changed(format!("{action_verb} {clean} (by user {user_suffix})"))?;

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
        overwrite: bool,
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
        let existed = full_path.exists();
        if existed && !overwrite {
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
        let action_verb = if existed { "Overwrite" } else { "Create" };
        let _ = vcs.snapshot_if_changed(format!(
            "{action_verb} {clean} from template (by user {user_suffix})"
        ))?;

        Ok(())
    }

    /// Creates an empty folder (arbitrarily nested, e.g. `assets/videos`) inside the project's
    /// workspace -- mainly so a user has somewhere to organize uploads (`upload_file`) into
    /// *before* uploading the first file there, rather than only being able to create a folder as
    /// a side effect of writing into it.
    ///
    /// Folders aren't file content, so the `FastCDC` snapshot layer (which versions file bytes, not
    /// directory structure) has nothing to actually track for an empty one -- and `list_files`
    /// only ever walks and lists *files*, so a directory with nothing in it would otherwise be
    /// silently invisible in the Files tab the moment after creating it, with no way to tell it
    /// worked. A `.gitkeep` placeholder (the same convention `git` users already know, since this
    /// project's own VCS layer -- `apich_vcs` -- otherwise has nothing to snapshot either) keeps
    /// the directory both non-empty on disk and present in the file listing (see
    /// `list_files`'s matching change) until real content lands in it.
    pub async fn create_folder(
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

        let clean = rel_path.trim().trim_matches('/');
        if clean.contains("..") || clean.is_empty() {
            return Err(WebError::BadRequest("Invalid folder path".to_string()));
        }

        let full_path = PathBuf::from(&proj.storage_path).join(clean);
        if full_path.exists() {
            return Err(WebError::Conflict(format!(
                "'{rel_path}' already exists"
            )));
        }

        tokio::fs::create_dir_all(&full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to create folder: {e}")))?;
        tokio::fs::write(full_path.join(".gitkeep"), b"")
            .await
            .map_err(|e| WebError::Internal(format!("Failed to create folder: {e}")))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Create folder {clean} (by user {user_suffix})"))?;

        Ok(())
    }

    /// Uploads binary content (a video/audio/image/etc. asset, or anything else -- no type
    /// restriction, this just writes bytes) to `rel_path` inside the project's workspace,
    /// creating any parent folders that don't exist yet. Same path-safety/VCS-snapshot shape as
    /// `create_file_with_content`, except: this overwrites an existing file at the same path
    /// instead of rejecting the upload outright -- re-uploading the same asset (a corrected
    /// image, a re-exported video) to replace what's there is the expected use, not an error case
    /// the way accidentally clobbering a hand-written source file via "+ New File" would be.
    /// The caller (`upload_file_action`) is responsible for any size cap -- this just writes
    /// whatever bytes it's given.
    pub async fn upload_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        content: &[u8],
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

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to write uploaded file: {e}")))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Upload {clean} (by user {user_suffix})"))?;

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

        // Handles both a plain file and a folder (e.g. one from `create_folder`, or any directory
        // that accumulated real uploads) through the same action -- the Files tab's "Del" button
        // posts the same form regardless of which kind of row it's on (see `project_detail.rs`),
        // so this needs to actually delete whichever it's given rather than erroring on a
        // directory the way `remove_file` alone would.
        if full_path.is_dir() {
            tokio::fs::remove_dir_all(&full_path)
                .await
                .map_err(|e| WebError::Internal(format!("Failed to delete folder: {e}")))?;
        } else {
            tokio::fs::remove_file(&full_path)
                .await
                .map_err(|e| WebError::Internal(format!("Failed to delete file: {e}")))?;
        }

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Delete {clean} (by user {user_suffix})"))?;

        Ok(())
    }

    /// Renames a file within the project workspace.
    pub async fn rename_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        new_name: &str,
        overwrite: bool,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean_old = rel_path.trim().trim_start_matches('/');
        let clean_new_name = new_name.trim().trim_matches('/');
        if clean_old.contains("..") || clean_old.is_empty() || clean_new_name.contains("..") || clean_new_name.contains('/') || clean_new_name.is_empty() {
            return Err(WebError::BadRequest("Invalid file path or name".to_string()));
        }

        let old_full_path = PathBuf::from(&proj.storage_path).join(clean_old);
        if !old_full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        let parent = old_full_path.parent().unwrap_or_else(|| std::path::Path::new(""));
        let new_full_path = parent.join(clean_new_name);

        let new_rel_path = if let Some(parent_rel) = std::path::Path::new(clean_old).parent().filter(|p| !p.as_os_str().is_empty()) {
            format!("{}/{}", parent_rel.display(), clean_new_name)
        } else {
            clean_new_name.to_string()
        };

        if new_full_path.exists() && !overwrite {
            return Err(WebError::Conflict(format!("File '{new_rel_path}' already exists")));
        }

        tokio::fs::rename(&old_full_path, &new_full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to rename file: {e}")))?;

        // Update file_shares if old_rel_path had share settings
        let mut settings = proj.settings.as_object().cloned().unwrap_or_default();
        if let Some(shares) = settings.get_mut("file_shares").and_then(|v| v.as_object_mut()) {
            if let Some(share_val) = shares.remove(clean_old) {
                shares.insert(new_rel_path.clone(), share_val);
                let _ = repo.update_project_settings(project_id, serde_json::Value::Object(settings)).await;
            }
        }

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Rename {clean_old} to {new_rel_path} (by user {user_suffix})"))?;

        Ok(new_rel_path)
    }

    /// Moves a file to another destination folder within the project workspace.
    pub async fn move_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        dest_folder: &str,
        overwrite: bool,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean_old = rel_path.trim().trim_start_matches('/');
        let clean_dest = dest_folder.trim().trim_matches('/');
        if clean_old.contains("..") || clean_old.is_empty() || clean_dest.contains("..") {
            return Err(WebError::BadRequest("Invalid file or folder path".to_string()));
        }

        let old_full_path = PathBuf::from(&proj.storage_path).join(clean_old);
        if !old_full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        let file_name = std::path::Path::new(clean_old)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| WebError::BadRequest("Invalid file name".to_string()))?;

        let new_rel_path = if clean_dest.is_empty() {
            file_name.to_string()
        } else {
            format!("{clean_dest}/{file_name}")
        };

        let new_full_path = PathBuf::from(&proj.storage_path).join(&new_rel_path);

        if new_full_path.exists() && !overwrite {
            return Err(WebError::Conflict(format!("File '{new_rel_path}' already exists")));
        }

        if let Some(parent) = new_full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                WebError::Internal(format!("Failed to create destination folder: {e}"))
            })?;
        }

        tokio::fs::rename(&old_full_path, &new_full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to move file: {e}")))?;

        // Update file_shares if old_rel_path had share settings
        let mut settings = proj.settings.as_object().cloned().unwrap_or_default();
        if let Some(shares) = settings.get_mut("file_shares").and_then(|v| v.as_object_mut()) {
            if let Some(share_val) = shares.remove(clean_old) {
                shares.insert(new_rel_path.clone(), share_val);
                let _ = repo.update_project_settings(project_id, serde_json::Value::Object(settings)).await;
            }
        }

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Move {clean_old} to {new_rel_path} (by user {user_suffix})"))?;

        Ok(new_rel_path)
    }

    /// Copies a file to another destination folder (optionally with a new name) within the project workspace.
    pub async fn copy_file(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        rel_path: &str,
        dest_folder: &str,
        new_name: Option<&str>,
        overwrite: bool,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let clean_src = rel_path.trim().trim_start_matches('/');
        let clean_dest = dest_folder.trim().trim_matches('/');
        if clean_src.contains("..") || clean_src.is_empty() || clean_dest.contains("..") {
            return Err(WebError::BadRequest("Invalid file or folder path".to_string()));
        }

        let src_full_path = PathBuf::from(&proj.storage_path).join(clean_src);
        if !src_full_path.exists() {
            return Err(WebError::NotFound(format!("File '{rel_path}' not found")));
        }

        let final_name = if let Some(n) = new_name.map(str::trim).filter(|s| !s.is_empty()) {
            if n.contains('/') || n.contains("..") {
                return Err(WebError::BadRequest("Invalid new file name".to_string()));
            }
            n.to_string()
        } else {
            std::path::Path::new(clean_src)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| WebError::BadRequest("Invalid file name".to_string()))?
                .to_string()
        };

        let new_rel_path = if clean_dest.is_empty() {
            final_name
        } else {
            format!("{clean_dest}/{final_name}")
        };

        let dest_full_path = PathBuf::from(&proj.storage_path).join(&new_rel_path);

        if dest_full_path.exists() && !overwrite {
            return Err(WebError::Conflict(format!("File '{new_rel_path}' already exists")));
        }

        if let Some(parent) = dest_full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                WebError::Internal(format!("Failed to create destination folder: {e}"))
            })?;
        }

        tokio::fs::copy(&src_full_path, &dest_full_path)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to copy file: {e}")))?;

        let user_hex = user_id.simple().to_string();
        let user_suffix = &user_hex[24..];
        let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
        let _ = vcs.snapshot_if_changed(format!("Copy {clean_src} to {new_rel_path} (by user {user_suffix})"))?;

        Ok(new_rel_path)
    }

    /// Search for text occurrences across single or multiple files in a project.
    /// Search for text occurrences across single or multiple files in a project.
    pub async fn search_text_in_project(
        &self,
        project_id: Uuid,
        options: SearchProjectOptions,
    ) -> WebResult<SearchProjectResult> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let proj_root = PathBuf::from(&proj.storage_path);
        let all_files = self.list_files(project_id).await?;

        let candidate_paths: Vec<String> = if let Some(ref paths) = options.file_paths {
            if paths.is_empty() {
                all_files.into_iter().filter(|f| !f.is_dir).map(|f| f.path).collect()
            } else {
                paths.clone()
            }
        } else {
            all_files.into_iter().filter(|f| !f.is_dir).map(|f| f.path).collect()
        };

        Self::search_text_in_dir(&proj_root, &candidate_paths, options).await
    }

    /// Helper to normalize replacement string for regex replacement.
    /// Converts `\0`..`\9` capture group references to `$0`..`$9` while keeping escaped `\\` intact.
    pub fn normalize_regex_replacement(rep: &str) -> String {
        let mut out = String::with_capacity(rep.len());
        let mut chars = rep.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_digit() {
                        out.push('$');
                        out.push(next_ch);
                        chars.next();
                        continue;
                    } else if next_ch == '\\' {
                        out.push('\\');
                        out.push('\\');
                        chars.next();
                        continue;
                    }
                }
                out.push('\\');
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// Search for text occurrences within a directory for specified candidate file paths.
    pub async fn search_text_in_dir(
        proj_root: &Path,
        candidate_paths: &[String],
        options: SearchProjectOptions,
    ) -> WebResult<SearchProjectResult> {
        let query = options.query.trim();
        if query.is_empty() {
            return Ok(SearchProjectResult {
                query: String::new(),
                total_matches: 0,
                files_count: 0,
                results: Vec::new(),
            });
        }

        let pattern_str = if options.is_regex {
            if options.whole_word {
                format!(r"\b(?:{query})\b")
            } else {
                query.to_string()
            }
        } else {
            let esc = regex::escape(query);
            if options.whole_word {
                format!(r"\b{esc}\b")
            } else {
                esc
            }
        };

        let re = regex::RegexBuilder::new(&pattern_str)
            .case_insensitive(!options.case_sensitive)
            .multi_line(true)
            .build()
            .map_err(|e| WebError::BadRequest(format!("Invalid search regex pattern: {e}")))?;

        let ext_filters: Option<Vec<String>> = options.extension_filter.as_ref().and_then(|ef| {
            let trimmed = ef.trim();
            if trimmed.is_empty() || trimmed == "*" || trimmed == "*.*" {
                None
            } else {
                Some(
                    trimmed
                        .split(&[',', ';', ' '][..])
                        .map(|s| s.trim().trim_start_matches('*').trim_start_matches('.').to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            }
        });

        let mut results = Vec::new();
        let mut total_matches = 0usize;

        for rel_path in candidate_paths {
            let clean = rel_path.trim().trim_start_matches('/');
            if clean.contains("..") || clean.is_empty() {
                continue;
            }

            if let Some(ref allowed_exts) = ext_filters {
                let ext = Path::new(clean)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !allowed_exts.iter().any(|allowed| allowed == &ext) {
                    continue;
                }
            }

            let full_path = proj_root.join(clean);
            if !full_path.is_file() {
                continue;
            }

            let bytes = match tokio::fs::read(&full_path).await {
                Ok(b) => b,
                Err(_) => continue,
            };

            let sample_len = bytes.len().min(8192);
            if let Some(sample) = bytes.get(..sample_len) {
                if sample.contains(&0u8) {
                    continue;
                }
            }

            let content = match std::str::from_utf8(&bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut file_matches = Vec::new();
            for (idx, line) in content.lines().enumerate() {
                let line_num = idx.saturating_add(1);
                for m in re.find_iter(line) {
                    let preview = options.replacement.as_ref().map(|rep| {
                        if options.is_regex {
                            let norm = Self::normalize_regex_replacement(rep);
                            re.replace_all(line, norm.as_str()).to_string()
                        } else {
                            re.replace_all(line, regex::NoExpand(rep.as_str())).to_string()
                        }
                    });
                    file_matches.push(SearchMatchOccurrence {
                        line_number: line_num,
                        line_content: line.to_string(),
                        match_text: m.as_str().to_string(),
                        match_start: m.start(),
                        match_end: m.end(),
                        preview_replaced: preview,
                    });
                }
            }

            if !file_matches.is_empty() {
                let match_count = file_matches.len();
                total_matches = total_matches.saturating_add(match_count);
                results.push(SearchFileResult {
                    file_path: clean.to_string(),
                    match_count,
                    matches: file_matches,
                });
            }
        }

        let files_count = results.len();
        Ok(SearchProjectResult {
            query: options.query,
            total_matches,
            files_count,
            results,
        })
    }

    /// Replace text occurrences across single or multiple files in a project, recording a VCS snapshot.
    pub async fn replace_text_in_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        options: ReplaceProjectOptions,
    ) -> WebResult<ReplaceProjectResult> {
        let repo = self.db.repository();
        let proj = repo
            .get_project_by_id(project_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

        let proj_root = PathBuf::from(&proj.storage_path);
        let query_snapshot = options.query.clone();
        let replacement_snapshot = options.replacement.clone();
        let res = Self::replace_text_in_dir(&proj_root, options).await?;

        if res.total_replacements > 0 {
            let user_hex = user_id.simple().to_string();
            let user_suffix = user_hex.get(24..).unwrap_or("user");
            let vcs = ProjectVcs::open_or_init(&proj.storage_path)?;
            let file_count = res.files_modified;
            let _ = vcs.snapshot_if_changed(format!(
                "Search & replace '{query_snapshot}' -> '{replacement_snapshot}' in {file_count} files (by user {user_suffix})"
            ))?;
        }

        Ok(res)
    }

    /// Replace text occurrences within a directory across specified candidate file paths.
    pub async fn replace_text_in_dir(
        proj_root: &Path,
        options: ReplaceProjectOptions,
    ) -> WebResult<ReplaceProjectResult> {
        let query = options.query.trim();
        if query.is_empty() {
            return Err(WebError::BadRequest("Search query cannot be empty".to_string()));
        }

        if options.file_paths.is_empty() {
            return Err(WebError::BadRequest("At least one target file must be selected for replacement".to_string()));
        }

        let pattern_str = if options.is_regex {
            if options.whole_word {
                format!(r"\b(?:{query})\b")
            } else {
                query.to_string()
            }
        } else {
            let esc = regex::escape(query);
            if options.whole_word {
                format!(r"\b{esc}\b")
            } else {
                esc
            }
        };

        let re = regex::RegexBuilder::new(&pattern_str)
            .case_insensitive(!options.case_sensitive)
            .multi_line(true)
            .build()
            .map_err(|e| WebError::BadRequest(format!("Invalid search regex pattern: {e}")))?;

        let mut modified_files = Vec::new();
        let mut total_replacements = 0usize;

        for rel_path in &options.file_paths {
            let clean = rel_path.trim().trim_start_matches('/');
            if clean.contains("..") || clean.is_empty() {
                continue;
            }

            let full_path = proj_root.join(clean);
            if !full_path.is_file() {
                continue;
            }

            let bytes = tokio::fs::read(&full_path)
                .await
                .map_err(|e| WebError::Internal(format!("Failed to read '{clean}': {e}")))?;

            let sample_len = bytes.len().min(8192);
            if let Some(sample) = bytes.get(..sample_len) {
                if sample.contains(&0u8) {
                    continue;
                }
            }

            let content = match std::str::from_utf8(&bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let match_count = re.find_iter(content).count();
            if match_count > 0 {
                let replaced = if options.is_regex {
                    let norm = Self::normalize_regex_replacement(&options.replacement);
                    re.replace_all(content, norm.as_str()).to_string()
                } else {
                    re.replace_all(content, regex::NoExpand(options.replacement.as_str())).to_string()
                };
                tokio::fs::write(&full_path, replaced.as_bytes())
                    .await
                    .map_err(|e| WebError::Internal(format!("Failed to write '{clean}': {e}")))?;

                modified_files.push(clean.to_string());
                total_replacements = total_replacements.saturating_add(match_count);
            }
        }

        let files_modified = modified_files.len();
        Ok(ReplaceProjectResult {
            success: true,
            files_modified,
            total_replacements,
            modified_files,
        })
    }
}

/// Options for searching text across project files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProjectOptions {
    /// Text or pattern to search for
    pub query: String,
    /// Optional replacement text (used to compute line replacement previews)
    pub replacement: Option<String>,
    /// Match case if true
    pub case_sensitive: bool,
    /// Match whole word boundaries if true
    pub whole_word: bool,
    /// Treat query as a regular expression if true
    pub is_regex: bool,
    /// Optional specific files to search; if None or empty, all text files in the project are searched
    pub file_paths: Option<Vec<String>>,
    /// Optional extension filter, e.g. "typ,tex,md,py" or "*.typ, *.tex"
    pub extension_filter: Option<String>,
}

/// A single occurrence of a search match in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatchOccurrence {
    /// Line number (1-indexed)
    pub line_number: usize,
    /// Content of the matched line
    pub line_content: String,
    /// Exact text matched by the search pattern
    pub match_text: String,
    /// Character offset start of match in the line
    pub match_start: usize,
    /// Character offset end of match in the line
    pub match_end: usize,
    /// Preview of the line if replacement is applied
    pub preview_replaced: Option<String>,
}

/// Search results for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFileResult {
    /// Project-relative path to the file
    pub file_path: String,
    /// Total occurrences in this file
    pub match_count: usize,
    /// List of occurrences
    pub matches: Vec<SearchMatchOccurrence>,
}

/// Overall search results across all searched files in a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProjectResult {
    /// The query that was searched
    pub query: String,
    /// Total number of matches across all files
    pub total_matches: usize,
    /// Number of files that had at least one match
    pub files_count: usize,
    /// List of per-file results
    pub results: Vec<SearchFileResult>,
}

/// Options for replacing text across project files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceProjectOptions {
    /// Text or pattern to find
    pub query: String,
    /// Replacement string
    pub replacement: String,
    /// Match case if true
    pub case_sensitive: bool,
    /// Match whole word boundaries if true
    pub whole_word: bool,
    /// Treat query as a regular expression if true
    pub is_regex: bool,
    /// Target file paths to execute replacements in (must not be empty)
    pub file_paths: Vec<String>,
}

/// Outcome of a search & replace operation across files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceProjectResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Number of files modified
    pub files_modified: usize,
    /// Total occurrences replaced
    pub total_replacements: usize,
    /// List of modified file paths
    pub modified_files: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDetailsView {
    pub id: Uuid,
    pub parent_snapshot_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub message: String,
    pub author: String,
    pub is_milestone: bool,
    pub milestone_name: Option<String>,
    pub tree_hash: String,
    pub gpg_signature: Option<String>,
    pub added_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub all_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: String, // "added", "removed", "context"
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffResult {
    pub file_path: String,
    pub is_binary: bool,
    pub status: String, // "added", "modified", "removed", "unchanged"
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub diff_lines: Vec<DiffLine>,
}

/// Compute line-by-line unified diff between two text strings using dynamic programming LCS
pub fn compute_unified_diff(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    if old_lines.is_empty() {
        return new_lines
            .into_iter()
            .enumerate()
            .map(|(i, l)| DiffLine {
                kind: "added".to_string(),
                old_lineno: None,
                new_lineno: Some(i + 1),
                content: l.to_string(),
            })
            .collect();
    }

    if new_lines.is_empty() {
        return old_lines
            .into_iter()
            .enumerate()
            .map(|(i, l)| DiffLine {
                kind: "removed".to_string(),
                old_lineno: Some(i + 1),
                new_lineno: None,
                content: l.to_string(),
            })
            .collect();
    }

    let n = old_lines.len();
    let m = new_lines.len();

    // Fallback if either file is excessively large to avoid huge memory allocations
    if n * m > 3_000_000 {
        let mut lines = Vec::new();
        for (i, l) in old_lines.into_iter().enumerate().take(500) {
            lines.push(DiffLine {
                kind: "removed".to_string(),
                old_lineno: Some(i + 1),
                new_lineno: None,
                content: l.to_string(),
            });
        }
        for (i, l) in new_lines.into_iter().enumerate().take(500) {
            lines.push(DiffLine {
                kind: "added".to_string(),
                old_lineno: None,
                new_lineno: Some(i + 1),
                content: l.to_string(),
            });
        }
        return lines;
    }

    // Standard LCS table
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if old_lines[i] == new_lines[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Backtrack to build diff
    let mut i = n;
    let mut j = m;
    let mut reversed_diff = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            reversed_diff.push(DiffLine {
                kind: "context".to_string(),
                old_lineno: Some(i),
                new_lineno: Some(j),
                content: old_lines[i - 1].to_string(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            reversed_diff.push(DiffLine {
                kind: "added".to_string(),
                old_lineno: None,
                new_lineno: Some(j),
                content: new_lines[j - 1].to_string(),
            });
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            reversed_diff.push(DiffLine {
                kind: "removed".to_string(),
                old_lineno: Some(i),
                new_lineno: None,
                content: old_lines[i - 1].to_string(),
            });
            i -= 1;
        }
    }

    reversed_diff.reverse();
    reversed_diff
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_and_replace_text_in_dir() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let f1 = root.join("main.typ");
        let f2 = root.join("chapter.tex");
        let f3 = root.join("notes.md");
        let f_bin = root.join("binary.dat");

        tokio::fs::write(&f1, b"Hello World!\nThis is a test.\nHello again!").await.unwrap();
        tokio::fs::write(&f2, b"\\section{Hello}\nWorld is hello.\nDone.").await.unwrap();
        tokio::fs::write(&f3, b"Unrelated content here.").await.unwrap();
        tokio::fs::write(&f_bin, b"Hello\x00Binary\x00World").await.unwrap();

        let candidate_paths = vec![
            "main.typ".to_string(),
            "chapter.tex".to_string(),
            "notes.md".to_string(),
            "binary.dat".to_string(),
        ];

        // 1. Case-insensitive search
        let opts = SearchProjectOptions {
            query: "hello".to_string(),
            replacement: Some("Hi".to_string()),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            file_paths: None,
            extension_filter: None,
        };

        let res = ProjectManager::search_text_in_dir(root, &candidate_paths, opts).await.unwrap();
        assert_eq!(res.files_count, 2);
        assert_eq!(res.total_matches, 4);

        // 2. Case-sensitive search
        let opts_case = SearchProjectOptions {
            query: "Hello".to_string(),
            replacement: None,
            case_sensitive: true,
            whole_word: false,
            is_regex: false,
            file_paths: None,
            extension_filter: None,
        };
        let res_case = ProjectManager::search_text_in_dir(root, &candidate_paths, opts_case).await.unwrap();
        assert_eq!(res_case.total_matches, 3);

        // 3. Whole word search
        let opts_word = SearchProjectOptions {
            query: "World".to_string(),
            replacement: None,
            case_sensitive: true,
            whole_word: true,
            is_regex: false,
            file_paths: None,
            extension_filter: None,
        };
        let res_word = ProjectManager::search_text_in_dir(root, &candidate_paths, opts_word).await.unwrap();
        assert_eq!(res_word.total_matches, 2);

        // 4. Regex search
        let opts_regex = SearchProjectOptions {
            query: "H[a-z]+o".to_string(),
            replacement: None,
            case_sensitive: true,
            whole_word: false,
            is_regex: true,
            file_paths: None,
            extension_filter: None,
        };
        let res_regex = ProjectManager::search_text_in_dir(root, &candidate_paths, opts_regex).await.unwrap();
        assert_eq!(res_regex.total_matches, 3);

        // 5. Extension filter
        let opts_filter = SearchProjectOptions {
            query: "Hello".to_string(),
            replacement: None,
            case_sensitive: true,
            whole_word: false,
            is_regex: false,
            file_paths: None,
            extension_filter: Some("typ".to_string()),
        };
        let res_filter = ProjectManager::search_text_in_dir(root, &candidate_paths, opts_filter).await.unwrap();
        assert_eq!(res_filter.files_count, 1);
        assert_eq!(res_filter.total_matches, 2);

        // 6. Replace in selected multiple files
        let rep_opts = ReplaceProjectOptions {
            query: "Hello".to_string(),
            replacement: "Greetings".to_string(),
            case_sensitive: true,
            whole_word: false,
            is_regex: false,
            file_paths: vec!["main.typ".to_string(), "chapter.tex".to_string()],
        };
        let rep_res = ProjectManager::replace_text_in_dir(root, rep_opts).await.unwrap();
        assert_eq!(rep_res.files_modified, 2);
        assert_eq!(rep_res.total_replacements, 3);

        let content_f1 = tokio::fs::read_to_string(&f1).await.unwrap();
        assert!(content_f1.contains("Greetings World!"));
        assert!(content_f1.contains("Greetings again!"));

        let content_f2 = tokio::fs::read_to_string(&f2).await.unwrap();
        assert!(content_f2.contains("\\section{Greetings}"));
        assert!(content_f2.contains("World is hello."));
    }

    #[tokio::test]
    async fn test_search_and_replace_single_file_scope() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let f1 = root.join("doc1.typ");
        let f2 = root.join("doc2.typ");

        tokio::fs::write(&f1, b"Target Word in doc1").await.unwrap();
        tokio::fs::write(&f2, b"Target Word in doc2").await.unwrap();

        let candidate_paths = vec!["doc1.typ".to_string(), "doc2.typ".to_string()];

        // Search only doc1
        let search_opts = SearchProjectOptions {
            query: "Target Word".to_string(),
            replacement: Some("Replacement".to_string()),
            case_sensitive: true,
            whole_word: true,
            is_regex: false,
            file_paths: Some(vec!["doc1.typ".to_string()]),
            extension_filter: None,
        };
        let search_res = ProjectManager::search_text_in_dir(root, &["doc1.typ".to_string()], search_opts).await.unwrap();
        assert_eq!(search_res.files_count, 1);
        assert_eq!(search_res.total_matches, 1);
        assert_eq!(search_res.results[0].file_path, "doc1.typ");

        // Replace only in doc1
        let rep_opts = ReplaceProjectOptions {
            query: "Target Word".to_string(),
            replacement: "Updated Word".to_string(),
            case_sensitive: true,
            whole_word: true,
            is_regex: false,
            file_paths: vec!["doc1.typ".to_string()],
        };
        let rep_res = ProjectManager::replace_text_in_dir(root, rep_opts).await.unwrap();
        assert_eq!(rep_res.files_modified, 1);
        assert_eq!(rep_res.total_replacements, 1);

        let content_f1 = tokio::fs::read_to_string(&f1).await.unwrap();
        assert_eq!(content_f1, "Updated Word in doc1");

        let content_f2 = tokio::fs::read_to_string(&f2).await.unwrap();
        assert_eq!(content_f2, "Target Word in doc2"); // untouched!
    }

    #[tokio::test]
    async fn test_search_and_replace_validation_and_errors() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        // Empty query search
        let empty_search = SearchProjectOptions {
            query: "".to_string(),
            replacement: None,
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            file_paths: None,
            extension_filter: None,
        };
        let res = ProjectManager::search_text_in_dir(root, &[], empty_search).await.unwrap();
        assert_eq!(res.total_matches, 0);

        // Invalid regex search
        let invalid_regex = SearchProjectOptions {
            query: "[unclosed".to_string(),
            replacement: None,
            case_sensitive: false,
            whole_word: false,
            is_regex: true,
            file_paths: None,
            extension_filter: None,
        };
        let res_err = ProjectManager::search_text_in_dir(root, &[], invalid_regex).await;
        assert!(res_err.is_err());

        // Replace with empty file_paths
        let rep_err = ReplaceProjectOptions {
            query: "find".to_string(),
            replacement: "rep".to_string(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            file_paths: Vec::new(),
        };
        let res_rep_err = ProjectManager::replace_text_in_dir(root, rep_err).await;
        assert!(res_rep_err.is_err());
    }

    #[tokio::test]
    async fn test_search_and_replace_regex_capture_groups() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let f = root.join("code.typ");
        tokio::fs::write(&f, b"#let author = \"Alice Smith\"\n#let reviewer = \"Bob Jones\"").await.unwrap();

        let candidate_paths = vec!["code.typ".to_string()];

        // 1. Search with regex and verify match_text and preview
        let search_opts = SearchProjectOptions {
            query: r#"#let (\w+) = "([^"]+)""#.to_string(),
            replacement: Some(r#"// $1 was $2"#.to_string()),
            case_sensitive: true,
            whole_word: false,
            is_regex: true,
            file_paths: None,
            extension_filter: None,
        };

        let s_res = ProjectManager::search_text_in_dir(root, &candidate_paths, search_opts).await.unwrap();
        assert_eq!(s_res.total_matches, 2);
        assert_eq!(s_res.results[0].matches[0].match_text, r#"#let author = "Alice Smith""#);
        assert_eq!(
            s_res.results[0].matches[0].preview_replaced.as_deref(),
            Some("// author was Alice Smith")
        );

        // 2. Replace using \1 and \2 syntax (normalized to $1, $2)
        let rep_opts = ReplaceProjectOptions {
            query: r#"#let (\w+) = "([^"]+)""#.to_string(),
            replacement: r#"#set \1(name: "\2")"#.to_string(),
            case_sensitive: true,
            whole_word: false,
            is_regex: true,
            file_paths: vec!["code.typ".to_string()],
        };

        let rep_res = ProjectManager::replace_text_in_dir(root, rep_opts).await.unwrap();
        assert_eq!(rep_res.files_modified, 1);
        assert_eq!(rep_res.total_replacements, 2);

        let content = tokio::fs::read_to_string(&f).await.unwrap();
        assert_eq!(content, "#set author(name: \"Alice Smith\")\n#set reviewer(name: \"Bob Jones\")");
    }

    #[tokio::test]
    async fn test_search_and_replace_literal_with_dollar() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let f = root.join("price.txt");
        tokio::fs::write(&f, b"Cost: 50 EUR per item [special]").await.unwrap();

        let candidate_paths = vec!["price.txt".to_string()];

        // Literal search containing regex-special brackets "[special]"
        let search_opts = SearchProjectOptions {
            query: "50 EUR per item [special]".to_string(),
            replacement: Some("$100 USD [special]".to_string()),
            case_sensitive: true,
            whole_word: false,
            is_regex: false,
            file_paths: None,
            extension_filter: None,
        };

        let s_res = ProjectManager::search_text_in_dir(root, &candidate_paths, search_opts).await.unwrap();
        assert_eq!(s_res.total_matches, 1);
        assert_eq!(s_res.results[0].matches[0].match_text, "50 EUR per item [special]");
        // Verify preview doesn't treat $100 as capture group
        assert_eq!(
            s_res.results[0].matches[0].preview_replaced.as_deref(),
            Some("Cost: $100 USD [special]")
        );

        // Replace literal with string containing $ sign
        let rep_opts = ReplaceProjectOptions {
            query: "50 EUR per item [special]".to_string(),
            replacement: "$100 USD [special]".to_string(),
            case_sensitive: true,
            whole_word: false,
            is_regex: false,
            file_paths: vec!["price.txt".to_string()],
        };

        let rep_res = ProjectManager::replace_text_in_dir(root, rep_opts).await.unwrap();
        assert_eq!(rep_res.files_modified, 1);
        assert_eq!(rep_res.total_replacements, 1);

        let content = tokio::fs::read_to_string(&f).await.unwrap();
        assert_eq!(content, "Cost: $100 USD [special]");
    }
}

