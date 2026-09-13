use crate::error::WebError;
use crate::error::WebResult;
use crate::services::knowledge_sync::MarkdownTask;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypstRenderResult {
    pub success: bool,
    pub pages_svg: Vec<String>,
    pub total_pages: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptOutputImage {
    pub name: String,
    pub data_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRunResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u128,
    pub output_images: Vec<ScriptOutputImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownRenderResult {
    pub html: String,
    pub tasks: Vec<MarkdownTask>,
}

pub struct DocumentRenderer;

impl DocumentRenderer {
    /// Check if file is an executable script (.py, .sh, .bash, .r, .rs, .js, .ts)
    pub fn is_script(path: &str) -> bool {
        let p = Path::new(path);
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(
            ext.as_str(),
            "py" | "sh" | "bash" | "r" | "rs" | "js" | "ts"
        )
    }

    /// Compile Typst or cargo-slide presentation to SVGs with line-level reverse search hyperlinks
    pub async fn compile_typst<P: AsRef<Path>>(
        project_root: P,
        rel_path: &str,
        dev_mode: bool,
    ) -> TypstRenderResult {
        let root = project_root.as_ref();
        let target_file = root.join(rel_path);

        if !target_file.exists() {
            return TypstRenderResult {
                success: false,
                pages_svg: Vec::new(),
                total_pages: 0,
                error_message: Some(format!("Source file not found: {rel_path}")),
            };
        }

        let tmp_parent = Path::new("/home/user/tmp");
        if !tmp_parent.exists() {
            std::fs::create_dir_all(tmp_parent).ok();
        }

        let run_id = uuid::Uuid::new_v4().to_string();
        let compile_dir = tmp_parent.join(format!("typst_{run_id}"));
        if let Err(e) = std::fs::create_dir_all(&compile_dir) {
            return TypstRenderResult {
                success: false,
                pages_svg: Vec::new(),
                total_pages: 0,
                error_message: Some(format!("Failed to create tmp dir: {e}")),
            };
        }

        let mut input_to_compile = target_file.clone();
        let mut did_annotate = false;

        // The annotated copy must live in the *same directory* as the original file, not some
        // separate scratch directory -- Typst resolves a bare relative import like
        // `#import "theme.typ"` relative to the importing file's own location, not `--root`. A
        // copy placed anywhere else (this used to be `root/.typst_dev_tmp/`) can no longer find
        // its sibling `theme.typ`/`slide.typ`, so the annotated compile always failed with "file
        // not found", silently falling back to the un-annotated source -- meaning reverse-search
        // links never existed for any file that imports another file (which is every real
        // cargo-slide/Typst document, including the demo project's own `slides.typ`). Reproduced
        // live: compiling the annotated copy from `.typst_dev_tmp/` threw exactly that error.
        let annotated_dir = target_file
            .parent()
            .map_or_else(|| root.to_path_buf(), Path::to_path_buf);
        let mut annotated_path: Option<PathBuf> = None;
        let mut zero_content_calls: Vec<u32> = Vec::new();

        if dev_mode {
            if let Ok(source) = std::fs::read_to_string(&target_file) {
                let (annotated, calls) = Self::annotate_typst_lines_for_reverse_search(&source);
                zero_content_calls = calls;
                let file_path = annotated_dir.join(format!(".apich_sync_{run_id}.typ"));
                if std::fs::write(&file_path, annotated.as_bytes()).is_ok() {
                    input_to_compile.clone_from(&file_path);
                    annotated_path = Some(file_path);
                    did_annotate = true;
                }
            }
        }

        let out_pattern = compile_dir.join(format!("page-{{{}}}.svg", "p"));
        let typst_bin = if Path::new("/home/user/.cargo/bin/typst").exists() {
            "/home/user/.cargo/bin/typst"
        } else {
            "typst"
        };

        let mut cmd = tokio::process::Command::new(typst_bin);
        cmd.env("TMPDIR", "/home/user/tmp")
            .arg("compile")
            .arg("--format")
            .arg("svg")
            .arg("--root")
            .arg(root)
            .arg(&input_to_compile)
            .arg(&out_pattern);

        let output = match cmd.output().await {
            | Ok(o) => o,
            | Err(e) => {
                if let Some(ref p) = annotated_path {
                    let _ = std::fs::remove_file(p);
                }
                let _ = std::fs::remove_dir_all(&compile_dir);
                return TypstRenderResult {
                    success: false,
                    pages_svg: Vec::new(),
                    total_pages: 0,
                    error_message: Some(format!("Failed to execute typst binary: {e}")),
                };
            },
        };

        if !output.status.success() {
            // If dev-mode annotated compile failed, fallback immediately to compiling the original source file
            if did_annotate {
                let mut fallback_cmd = tokio::process::Command::new(typst_bin);
                fallback_cmd
                    .env("TMPDIR", "/home/user/tmp")
                    .arg("compile")
                    .arg("--format")
                    .arg("svg")
                    .arg("--root")
                    .arg(root)
                    .arg(&target_file)
                    .arg(&out_pattern);

                if let Ok(fb_out) = fallback_cmd.output().await {
                    if let Some(ref p) = annotated_path {
                        let _ = std::fs::remove_file(p);
                    }

                    if fb_out.status.success() {
                        let pages = Self::collect_svg_pages(&compile_dir);
                        let _ = std::fs::remove_dir_all(&compile_dir);
                        let count = pages.len();
                        return TypstRenderResult {
                            success: true,
                            pages_svg: pages,
                            total_pages: count,
                            error_message: None,
                        };
                    }
                    let stderr = String::from_utf8_lossy(&fb_out.stderr).to_string();
                    let _ = std::fs::remove_dir_all(&compile_dir);
                    return TypstRenderResult {
                        success: false,
                        pages_svg: Vec::new(),
                        total_pages: 0,
                        error_message: Some(stderr),
                    };
                }
            }

            if let Some(ref p) = annotated_path {
                let _ = std::fs::remove_file(p);
            }
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let _ = std::fs::remove_dir_all(&compile_dir);
            return TypstRenderResult {
                success: false,
                pages_svg: Vec::new(),
                total_pages: 0,
                error_message: Some(stderr),
            };
        }

        if let Some(ref p) = annotated_path {
            let _ = std::fs::remove_file(p);
        }

        let pages = Self::collect_svg_pages(&compile_dir);
        let pages = Self::inject_fallback_lines(pages, &zero_content_calls);
        let _ = std::fs::remove_dir_all(&compile_dir);
        let count = pages.len();

        TypstRenderResult {
            success: true,
            pages_svg: pages,
            total_pages: count,
            error_message: None,
        }
    }

    /// Compile a `.typ` file straight to PDF bytes for download (separate from `compile_typst`,
    /// which produces per-page SVGs for the live in-browser preview). Runs on this dev host, same
    /// as `compile_typst` -- `typst` is a small static binary, unlike the LaTeX toolchain, which
    /// only exists inside the project's sandbox container.
    pub async fn compile_typst_pdf<P: AsRef<Path>>(
        project_root: P,
        rel_path: &str,
    ) -> Result<Vec<u8>, String> {
        let root = project_root.as_ref();
        let target_file = root.join(rel_path);
        if !target_file.exists() {
            return Err(format!("Source file not found: {rel_path}"));
        }

        let tmp_parent = Path::new("/home/user/tmp");
        if !tmp_parent.exists() {
            std::fs::create_dir_all(tmp_parent).ok();
        }
        let run_id = uuid::Uuid::new_v4().to_string();
        let out_path = tmp_parent.join(format!("typst_pdf_{run_id}.pdf"));

        let typst_bin = if Path::new("/home/user/.cargo/bin/typst").exists() {
            "/home/user/.cargo/bin/typst"
        } else {
            "typst"
        };

        let mut cmd = tokio::process::Command::new(typst_bin);
        cmd.env("TMPDIR", "/home/user/tmp")
            .arg("compile")
            .arg("--format")
            .arg("pdf")
            .arg("--root")
            .arg(root)
            .arg(&target_file)
            .arg(&out_path);

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute typst binary: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        let bytes = std::fs::read(&out_path)
            .map_err(|e| format!("typst reported success but no PDF was found: {e}"))?;
        let _ = std::fs::remove_file(&out_path);
        Ok(bytes)
    }

    /// Helper to collect and sort generated SVG pages
    fn collect_svg_pages(dir: &Path) -> Vec<String> {
        let mut svgs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "svg"))
                .collect();

            paths.sort_by_key(|p| {
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let num_str = stem.trim_start_matches("page-");
                num_str.parse::<usize>().unwrap_or(0)
            });

            for path in paths {
                if let Ok(content) = std::fs::read_to_string(path) {
                    svgs.push(content);
                }
            }
        }
        svgs
    }

    /// Annotate source lines with #link("sync:line:L")[...] for dev-mode reverse search.
    ///
    /// This used to wrap every non-blank, non-directive line unconditionally -- which silently
    /// broke on any real Typst document. Typst has two parsing modes (code, inside `(...)`/`{...}`
    /// argument lists and blocks, vs. markup, inside `[...]` content and top-level document text),
    /// and only markup-mode *lines that are wholly self-contained* (open and close any brackets
    /// they use within the same line) can be safely rewrapped in a new `#link(...)[...]` --
    /// wrapping a bare continuation line like a lone `[` that opens a multi-line content block, a
    /// code-mode `key: "value",` argument line, or a line whose only `#` is inside a string (e.g.
    /// `color: "#38bdf8"`) corrupts the surrounding bracket structure. Reproduced live: compiling
    /// the demo project's own cargo-slide deck (which uses exactly these multi-line
    /// `#cols([...], [...])`/`#chart(...)` constructs) through the old annotator produced a wall
    /// of "the character `#` is not valid in code" / "invalid number suffix" / "unclosed
    /// delimiter" errors, so the dev-mode compile always fell through to the un-annotated
    /// fallback -- meaning reverse-search links silently never existed on any non-trivial file.
    ///
    /// Fix: track the stack of currently-open `(`/`{`/`[` delimiters across the whole document
    /// (skipping string-literal contents so a `#` or bracket character inside a quoted string
    /// never perturbs it), and only wrap a line when: we're in markup mode at the start of the
    /// line (stack is empty or topped by `[`), the line doesn't start with `#` (a code
    /// invocation), and the line's own delimiters are perfectly balanced (leaves the stack
    /// exactly as it found it) -- i.e. only lines that are themselves complete, self-contained
    /// markup content get a jump target. Fewer lines are clickable than the old (broken) attempt
    /// aimed for, but every annotated file now actually compiles.
    ///
    /// A real consequence of this line-level-only approach: a `#slide(...)`/`#title-slide(...)`
    /// call whose entire body is named arguments (`title: "...", author: "...", ...`, no `[...]`
    /// markup content at all -- exactly what a title slide typically is) has *no* line eligible
    /// for wrapping, so it ends up with zero reverse-search links -- confirmed live: the demo
    /// deck's title slide (the first, default-visible slide) had 0 clickable links while every
    /// content slide after it had dozens, making click-to-jump look completely broken to anyone
    /// who tried it on the slide they'd actually see first. The second return value tracks the
    /// starting line of every such "zero-content" top-level `#slide`/`#title-slide` call, in
    /// source order, so the compiled page that call produced can fall back to jumping to the call
    /// itself instead of doing nothing (see `compile_typst`'s use of it).
    pub fn annotate_typst_lines_for_reverse_search(source: &str) -> (String, Vec<u32>) {
        let mut result = Vec::new();
        let mut in_multiline_comment = false;
        let mut in_code_block = false;
        let mut stack: Vec<char> = Vec::new();
        let mut zero_content_calls: Vec<u32> = Vec::new();
        let mut current_call: Option<(u32, bool)> = None;

        for (idx, line) in source.lines().enumerate() {
            let line_num = idx.saturating_add(1);
            let trimmed = line.trim();

            if trimmed.contains("/*") && !trimmed.contains("*/") {
                in_multiline_comment = true;
            }
            if in_multiline_comment {
                result.push(line.to_string());
                if trimmed.contains("*/") {
                    in_multiline_comment = false;
                }
                continue;
            }

            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                result.push(line.to_string());
                continue;
            }
            if in_code_block {
                result.push(line.to_string());
                continue;
            }

            let in_markup_before = stack.last().is_none_or(|c| *c == '[');
            let stack_was_empty = stack.is_empty();
            let stack_len_before = stack.len();
            let mut balanced = true;
            let mut in_string = false;
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                if in_string {
                    if c == '\\' {
                        chars.next();
                    } else if c == '"' {
                        in_string = false;
                    }
                    continue;
                }
                match c {
                    | '"' => in_string = true,
                    | '/' if chars.peek() == Some(&'/') => break, // rest of line is a comment
                    | '(' | '{' | '[' => stack.push(c),
                    | ')' | '}' | ']' => {
                        if stack.pop().is_none() {
                            balanced = false;
                        }
                    },
                    | _ => {},
                }
            }
            let stack_unchanged = balanced && stack.len() == stack_len_before;

            if current_call.is_none()
                && stack_was_empty
                && in_markup_before
                && (trimmed.starts_with("#slide(") || trimmed.starts_with("#title-slide("))
            {
                current_call = Some((line_num as u32, false));
            }

            let skip = trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || !in_markup_before
                || !stack_unchanged;

            if skip {
                result.push(line.to_string());
            } else {
                if let Some((_, annotated_any)) = current_call.as_mut() {
                    *annotated_any = true;
                }
                let line_to_wrap = if line.trim_end().ends_with('\\') {
                    format!("{line} ")
                } else {
                    line.to_string()
                };
                result.push(format!("#link(\"sync:line:{line_num}\")[{line_to_wrap}]"));
            }

            if let Some((start, annotated_any)) = current_call {
                if stack.is_empty() {
                    if !annotated_any {
                        zero_content_calls.push(start);
                    }
                    current_call = None;
                }
            }
        }

        (result.join("\n"), zero_content_calls)
    }

    /// Inserts `data-fallback-line="N"` into the root `<svg>` tag of every compiled page that
    /// ended up with zero `sync:line:` reverse-search links (see
    /// `annotate_typst_lines_for_reverse_search`'s doc comment) -- `zero_content_calls` supplies
    /// the fallback line for each such page, matched in the same source order both lists are
    /// already in (a link-less page and its originating zero-content call appear in the same
    /// relative order, since Typst renders pages in source order). The click handler
    /// (`REVERSE_SEARCH_CLICK_JS`) reads this attribute when a click doesn't land on any
    /// finer-grained `sync:line:` link.
    fn inject_fallback_lines(
        pages: Vec<String>,
        zero_content_calls: &[u32],
    ) -> Vec<String> {
        let mut calls = zero_content_calls.iter();
        pages
            .into_iter()
            .map(|mut page| {
                if !page.contains("sync:line:") {
                    if let (Some(&line), Some(pos)) = (calls.next(), page.find("<svg")) {
                        page.insert_str(
                            pos.saturating_add(4),
                            &format!(" data-fallback-line=\"{line}\""),
                        );
                    }
                }
                page
            })
            .collect()
    }

    /// Execute a script directly on the host running this process, capturing stdout/stderr,
    /// runtime, and output plot images. **Not the production path** -- `run_script_action`
    /// (the real HTTP endpoint) calls `ProjectManagerService::run_script_in_sandbox` instead,
    /// which runs the interpreter inside the project's own sandbox container (which has
    /// R/Python/Rust actually installed; this dev host doesn't have R at all, which is exactly
    /// the bug that motivated moving execution into the sandbox -- see that method's doc
    /// comment). This host-local version stays as the interpreter-dispatch/image-diffing logic
    /// its own unit test (`test_script_execution`) exercises without needing a live sandbox.
    ///
    /// # Errors
    ///
    /// Returns a [`WebError`] if the script file does not exist or execution fails.
    pub async fn run_script<P: AsRef<Path>>(
        project_root: P,
        rel_path: &str,
        args: &str,
        stdin_input: Option<&str>,
    ) -> WebResult<ScriptRunResult> {
        let root = project_root.as_ref();
        let script_file = root.join(rel_path);

        if !script_file.exists() {
            return Err(WebError::NotFound(format!("Script not found: {rel_path}")));
        }

        let ext = script_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // The subprocess's cwd is set to `root` below, so the script argument must be the plain
        // `rel_path` -- not `script_file` (== root.join(rel_path)) -- or the two `root` prefixes
        // compound into a nonexistent path once the cwd has already moved there. Reproduced live:
        // `project.storage_path` is a *relative* path by default (`./scratch/workspace/...`), so
        // running a script threw "can't open file '.../apich-showcase-x/./scratch/workspace/.../
        // apich-showcase-x/analysis.py'" -- the project root, twice -- with the real error only
        // visible in the script console's stderr pane, never surfaced as a page-level failure.
        //
        // `.rs` files fell through to the `_` catch-all and were run as `bash script.rs`
        // (a Rust source file interpreted as a shell script -- always fails, and confusingly, see
        // below) -- `is_script()` lists "rs" as a runnable extension but this dispatch table never
        // actually handled it. Compile-then-run via a `bash -c` wrapper, passing the source path
        // and any extra CLI args as real argv entries (`$1`, `${@:2}`) rather than interpolating
        // them into the shell string, so user-supplied args can't be misparsed as shell syntax.
        let run_id = uuid::Uuid::new_v4().simple().to_string();
        let rust_bin_path = format!("/home/user/tmp/apich_run_{run_id}");
        let (interpreter, base_args): (&str, Vec<String>) = match ext.as_str() {
            | "py" => ("python3", vec![rel_path.to_string()]),
            | "r" => ("Rscript", vec![rel_path.to_string()]),
            | "rs" => {
                (
                    "bash",
                    vec![
                        "-c".to_string(),
                        format!(
                            r#"rustc -O "$1" -o "{bin}" && "{bin}" "${{@:2}}""#,
                            bin = rust_bin_path
                        ),
                        "bash".to_string(),
                        rel_path.to_string(),
                    ],
                )
            },
            | "js" | "ts" => ("node", vec![rel_path.to_string()]),
            | _ => ("bash", vec![rel_path.to_string()]),
        };

        // Snapshot existing image timestamps before run
        let before_images = Self::scan_images(root);

        let start = Instant::now();
        let mut cmd = tokio::process::Command::new(interpreter);
        cmd.current_dir(root)
            .env("TMPDIR", "/home/user/tmp")
            .env("PYTHONUNBUFFERED", "1")
            .args(&base_args);

        if !args.trim().is_empty() {
            for arg in args.split_whitespace() {
                cmd.arg(arg);
            }
        }

        if stdin_input.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        }
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| WebError::Internal(format!("Failed to spawn script process: {e}")))?;

        if let Some(input) = stdin_input {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(input.as_bytes()).await;
            }
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| WebError::Internal(format!("Script process execution error: {e}")))?;

        if ext == "rs" {
            let _ = std::fs::remove_file(&rust_bin_path);
        }

        let elapsed = start.elapsed().as_millis();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();
        let success = output.status.success();

        // Detect newly generated or updated images
        let after_images = Self::scan_images(root);
        let mut output_images = Vec::new();

        for (img_path, mtime) in &after_images {
            let is_new_or_modified = match before_images.get(img_path) {
                | Some(old_mtime) => mtime > old_mtime,
                | None => true,
            };

            if is_new_or_modified {
                if let Ok(bytes) = tokio::fs::read(img_path).await {
                    let ext = img_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png")
                        .to_lowercase();
                    let mime = match ext.as_str() {
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

                    output_images.push(ScriptOutputImage { name, data_uri });
                }
            }
        }

        Ok(ScriptRunResult {
            success,
            exit_code,
            stdout,
            stderr,
            execution_time_ms: elapsed,
            output_images,
        })
    }

    /// Scan workspace directory for plot / image files with modified times. `pub(crate)` (not
    /// private) so `ProjectManagerService::run_script_in_sandbox` can reuse the exact same
    /// before/after diffing this host-based `run_script` uses, for a script that actually ran
    /// inside the sandbox container instead -- the container's `/workspace` is the same
    /// bind-mounted directory as this host path, so any file it writes is visible here too.
    pub(crate) fn scan_images(
        dir: &Path
    ) -> std::collections::HashMap<PathBuf, std::time::SystemTime> {
        let mut map = std::collections::HashMap::new();
        walk_images(dir, &mut map);
        map
    }

    /// Render Markdown document into HTML with `KaTeX` math rendering, interactive task checkboxes, and [[`WikiLinks`]]
    pub fn render_markdown_interactive(
        content: &str,
        file_path: &str,
        project_id: uuid::Uuid,
    ) -> MarkdownRenderResult {
        let mut html = String::new();
        let mut tasks = Vec::new();

        let Ok(task_regex) = Regex::new(r"^\s*[-*]\s+\[([ xX/])\]\s+(.*)$") else {
            return MarkdownRenderResult { html, tasks };
        };
        let Ok(tag_regex) = Regex::new(r"#([a-zA-Z0-9_\-]+)") else {
            return MarkdownRenderResult { html, tasks };
        };
        let Ok(date_regex) = Regex::new(r"@(\d{4}-\d{2}-\d{2})") else {
            return MarkdownRenderResult { html, tasks };
        };

        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_lines = Vec::new();
        let mut in_list = false;

        for (idx, line) in content.lines().enumerate() {
            let line_num = idx.saturating_add(1);
            let trimmed = line.trim();

            // Code blocks
            if trimmed.starts_with("```") {
                if in_code_block {
                    let _ = writeln!(
                        html,
                        "<pre class=\"code-block\"><code class=\"language-{}\">{}</code></pre>",
                        html_escape(&code_lang),
                        html_escape(&code_lines.join("\n"))
                    );
                    code_lines.clear();
                    in_code_block = false;
                } else {
                    if in_list {
                        html.push_str("</ul>\n");
                        in_list = false;
                    }
                    code_lang = trimmed.trim_start_matches("```").trim().to_string();
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_lines.push(line.to_string());
                continue;
            }

            // Math blocks: $$ ... $$
            if trimmed.starts_with("$$") && trimmed.ends_with("$$") && trimmed.len() > 4 {
                if in_list {
                    html.push_str("</ul>\n");
                    in_list = false;
                }
                let math = &trimmed[2..trimmed.len().saturating_sub(2)];
                let _ = write!(
                    html,
                    "<div class=\"math-block\" data-line=\"{line_num}\" data-math=\"{}\">$$\n{}\n$$</div>\n",
                    html_escape(math.trim()),
                    html_escape(math.trim())
                );
                continue;
            }

            // Task item: - [ ] or - [x] or - [/]
            if let Some(caps) = task_regex.captures(line) {
                if !in_list {
                    html.push_str("<ul class=\"interactive-task-list\">\n");
                    in_list = true;
                }

                let mark = caps.get(1).map_or(" ", |m| m.as_str());
                let raw_body = caps.get(2).map_or("", |m| m.as_str()).trim();
                let completed = mark == "x" || mark == "X";
                let status = if completed {
                    "done"
                } else if mark == "/" {
                    "in_progress"
                } else {
                    "todo"
                };

                let tags: Vec<String> = tag_regex
                    .find_iter(raw_body)
                    .map(|m| m.as_str().trim_start_matches('#').to_string())
                    .collect();
                let due_date = date_regex
                    .captures(raw_body)
                    .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

                let mut clean_title = tag_regex.replace_all(raw_body, "").to_string();
                clean_title = date_regex.replace_all(&clean_title, "").to_string();
                let title = clean_title.trim().to_string();

                let id = format!("{file_path}:{line_num}");
                tasks.push(MarkdownTask {
                    id,
                    file_path: file_path.to_string(),
                    line_number: line_num,
                    raw_line: line.to_string(),
                    title: if title.is_empty() {
                        raw_body.to_string()
                    } else {
                        title
                    },
                    completed,
                    status: status.to_string(),
                    tags,
                    due_date,
                });

                let checked_attr = if completed { "checked" } else { "" };
                let class_extra = if completed {
                    "task-completed"
                } else if mark == "/" {
                    "task-in-progress"
                } else {
                    ""
                };

                let mut badges = String::new();
                for t in tag_regex.find_iter(raw_body) {
                    let _ = write!(
                        badges,
                        "<span class=\"task-tag-badge\">{}</span> ",
                        html_escape(t.as_str())
                    );
                }
                if let Some(d) = date_regex.captures(raw_body).and_then(|c| c.get(0)) {
                    let _ = write!(
                        badges,
                        "<span class=\"task-date-badge\">📅 {}</span> ",
                        html_escape(d.as_str())
                    );
                }

                // Render body with wiki links
                let body_with_wiki = Self::format_inline_markdown(raw_body, project_id);

                let _ = writeln!(
                    html,
                    "<li class=\"interactive-task-row {class_extra}\" data-line=\"{line_num}\" data-file=\"{}\">\
                        <input type=\"checkbox\" {checked_attr} class=\"task-live-checkbox\">\
                        <span class=\"task-text\">{body_with_wiki}</span>\
                        <div class=\"task-badges\">{badges}</div>\
                    </li>",
                    html_escape(file_path)
                );
                continue;
            }

            if in_list && !trimmed.starts_with('-') && !trimmed.starts_with('*') {
                html.push_str("</ul>\n");
                in_list = false;
            }

            // Headings
            if trimmed.starts_with('#') {
                let level = trimmed.chars().take_while(|&c| c == '#').count();
                if level <= 4 {
                    let htext = trimmed[level..].trim();
                    let formatted_text = Self::format_inline_markdown(htext, project_id);
                    let _ = writeln!(
                        html,
                        "<h{level} data-line=\"{line_num}\" class=\"doc-heading\" id=\"sec-{line_num}\">\
                            <span>{formatted_text}</span>\
                            <a href=\"javascript:void(0)\" class=\"line-sync-anchor\" data-line=\"{line_num}\" title=\"Reverse search: jump to line {line_num}\">#L{line_num}</a>\
                        </h{level}>"
                    );
                    continue;
                }
            }

            // Empty lines
            if trimmed.is_empty() {
                html.push_str("<div class=\"doc-spacer\"></div>\n");
                continue;
            }

            // Regular paragraph
            let p_formatted = Self::format_inline_markdown(trimmed, project_id);
            let _ = writeln!(
                html,
                "<p class=\"doc-paragraph\" data-line=\"{line_num}\">{p_formatted}</p>"
            );
        }

        if in_list {
            html.push_str("</ul>\n");
        }
        if in_code_block {
            let _ = writeln!(
                html,
                "<pre class=\"code-block\"><code class=\"language-{}\">{}</code></pre>",
                html_escape(&code_lang),
                html_escape(&code_lines.join("\n"))
            );
        }

        MarkdownRenderResult { html, tasks }
    }

    /// Format inline Markdown elements: `code`, **bold**, *italic*, $inline math$, and [[`WikiLinks`]]
    fn format_inline_markdown(
        input: &str,
        project_id: uuid::Uuid,
    ) -> String {
        let mut out = html_escape(input);

        // Wiki links: [[Target|Display]] or [[Target]] -- one pass, one regex. This used to be two
        // sequential regexes (pipe-form, then simple-form); the simple-form pass ran on the
        // pipe-form's own output and re-matched the literal `[[Display]]` text still sitting
        // inside the anchor it had just produced, wrapping it in a second, nested `<a>` pointed at
        // the display text (not the real target) -- e.g. `<a href=.../cryostat_cooldown.anote>
        // <a href=.../Cryostat%20Cool-Down%20Log.anote>[[Cryostat Cool-Down Log]]</a></a>`, a
        // real link to a nonexistent file inside invalid nested-anchor HTML. Caught live: the
        // demo project's own pipe-form links rendered exactly this way.
        let Ok(wiki_link) = Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]") else {
            return out;
        };
        out = wiki_link
            .replace_all(&out, |caps: &regex::Captures<'_>| {
                let target = caps.get(1).map_or("", |m| m.as_str().trim());
                let display = caps.get(2).map_or(target, |m| m.as_str().trim());
                let note_file = if Path::new(target).extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("anote") || ext.eq_ignore_ascii_case("md")
                }) {
                    target.to_string()
                } else {
                    format!("{target}.anote")
                };
                format!(
                    "<a href=\"/projects/{}/note?file={}\" class=\"wiki-link-pill\">[[{}]]</a>",
                    project_id,
                    urlencoding::encode(&note_file),
                    display
                )
            })
            .to_string();

        // Inline math: $formula$
        if let Ok(inline_math) = Regex::new(r"\$([^\$]+)\$") {
            out = inline_math
                .replace_all(&out, |caps: &regex::Captures<'_>| {
                    let math = caps.get(1).map_or("", |m| m.as_str());
                    format!("<span class=\"math-inline\" data-math=\"{math}\">${math}$</span>")
                })
                .to_string();
        }

        // Bold: **text**
        if let Ok(bold) = Regex::new(r"\*\*([^\*]+)\*\*") {
            out = bold
                .replace_all(&out, |caps: &regex::Captures<'_>| {
                    format!(
                        "<strong>{}</strong>",
                        caps.get(1).map_or("", |m| m.as_str())
                    )
                })
                .to_string();
        }

        // Inline code: `code`
        if let Ok(code) = Regex::new(r"`([^`]+)`") {
            out = code
                .replace_all(&out, |caps: &regex::Captures<'_>| {
                    format!(
                        "<code class=\"inline-code\">{}</code>",
                        caps.get(1).map_or("", |m| m.as_str())
                    )
                })
                .to_string();
        }

        out
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn walk_images(
    p: &Path,
    map: &mut std::collections::HashMap<PathBuf, std::time::SystemTime>,
) {
    if let Ok(entries) = std::fs::read_dir(p) {
        for e in entries.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    walk_images(&path, map);
                }
            } else if path.is_file()
                && path.extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("png")
                        || ext.eq_ignore_ascii_case("svg")
                        || ext.eq_ignore_ascii_case("jpg")
                        || ext.eq_ignore_ascii_case("jpeg")
                })
            {
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        map.insert(path, modified);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_typst_annotation_for_reverse_search() {
        let typst_code = r#"#import "theme.typ": *
#show: slide-theme

= Section 1: Overview
The transmon frequency is 5.0 GHz.

$ H = h bar omega $
"#;
        let (annotated, zero_content_calls) =
            DocumentRenderer::annotate_typst_lines_for_reverse_search(typst_code);
        assert!(annotated.contains("#link(\"sync:line:4\")[= Section 1: Overview]"));
        assert!(annotated.contains("#link(\"sync:line:5\")[The transmon frequency is 5.0 GHz.]"));
        // Directives should not be wrapped
        assert!(annotated.contains("#import \"theme.typ\": *"));
        // No `#slide`/`#title-slide` calls in this document, so nothing should be tracked.
        assert!(zero_content_calls.is_empty());
    }

    #[tokio::test]
    async fn test_typst_annotation_tracks_zero_content_slide_calls() {
        let typst_code = r#"#title-slide(
  title: "My Deck",
  author: "Someone"
)

#slide(title: "Real content")[
  Some real visible text on this slide.
]
"#;
        let (_annotated, zero_content_calls) =
            DocumentRenderer::annotate_typst_lines_for_reverse_search(typst_code);
        // `#title-slide(...)` has only named-argument lines (code mode, never wrapped) -- it
        // should be reported as a zero-content call starting at line 1.
        assert_eq!(zero_content_calls, vec![1]);
    }

    /// Regression test for a real bug: the annotator used to wrap every non-blank line
    /// unconditionally, which corrupted any multi-line Typst construct (`#cols([...], [...])`,
    /// `#chart(...)` argument lists, a bare `[`/`]` continuation, a `#RRGGBB` color string) and
    /// made the dev-mode compile fail -- silently falling back to the un-annotated source, so
    /// reverse-search links never existed on any real (non-trivial) document. This compiles the
    /// actual seeded demo presentation (which uses exactly those constructs) through the real
    /// `typst` binary in dev mode and checks both that it succeeds and that it actually produced
    /// `sync:line:` links, not just that the compile didn't error.
    #[tokio::test]
    async fn test_typst_reverse_search_annotation_compiles_on_real_multiline_document() {
        let dir = tempdir().unwrap();
        crate::services::demo_project::DemoProjectService::seed_demo_files(dir.path())
            .await
            .expect("seed demo files");

        let result = DocumentRenderer::compile_typst(dir.path(), "slides.typ", true).await;
        assert!(
            result.success,
            "typst compile failed: {:?}",
            result.error_message
        );
        assert!(
            !result.pages_svg.is_empty(),
            "expected at least one rendered page"
        );
        let has_sync_link = result
            .pages_svg
            .iter()
            .any(|svg| svg.contains("sync:line:"));
        assert!(
            has_sync_link,
            "expected at least one sync:line: reverse-search link in the compiled SVG output"
        );
    }

    #[tokio::test]
    async fn test_compile_typst_pdf_produces_real_pdf_bytes() {
        let dir = tempdir().unwrap();
        crate::services::demo_project::DemoProjectService::seed_demo_files(dir.path())
            .await
            .expect("seed demo files");

        let pdf_bytes = DocumentRenderer::compile_typst_pdf(dir.path(), "slides.typ")
            .await
            .expect("typst pdf compile failed");
        assert!(
            pdf_bytes.starts_with(b"%PDF-"),
            "output should be a real PDF (starts with %PDF- magic bytes)"
        );
        assert!(
            pdf_bytes.len() > 100,
            "expected a non-trivial PDF, got {} bytes",
            pdf_bytes.len()
        );
    }

    #[tokio::test]
    async fn test_compile_typst_pdf_reports_missing_file() {
        let dir = tempdir().unwrap();
        let err = DocumentRenderer::compile_typst_pdf(dir.path(), "does_not_exist.typ")
            .await
            .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn test_script_execution() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("test_run.py");
        std::fs::write(&script, b"print('APICH Quantum Execution OK')\n").unwrap();

        let res = DocumentRenderer::run_script(dir.path(), "test_run.py", "", None)
            .await
            .unwrap();
        assert!(res.success);
        assert_eq!(res.exit_code, Some(0));
        assert!(res.stdout.contains("APICH Quantum Execution OK"));
    }

    #[test]
    fn test_markdown_interactive_render() {
        let md = r#"# Lab Notebook
- [ ] Calibrate pulse #rf @2026-09-15
- [x] Measure baseline
Discussion on [[Quantum Resonator]].
$$ E = m c^2 $$
"#;
        let proj_id = uuid::Uuid::new_v4();
        let res = DocumentRenderer::render_markdown_interactive(md, "test.anote", proj_id);
        assert_eq!(res.tasks.len(), 2);
        assert!(res.html.contains("interactive-task-row"));
        assert!(res.html.contains("wiki-link-pill"));
        assert!(res.html.contains("math-block"));
    }
}
