use crate::error::Result;
use crate::error::SlideError;
use crate::model::Slide;
use crate::model::SlideDeck;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Compiler for Typst presentation files
pub struct SlideCompiler {
    typst_path: PathBuf,
}

impl Default for SlideCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to locate Typst executable")
    }
}

impl SlideCompiler {
    /// Locate typst compiler on the system
    pub fn new() -> Result<Self> {
        let typst_path = Self::find_typst().ok_or_else(|| {
            SlideError::Compilation(
                "Typst executable not found. Please install Typst or ensure it is on your PATH, or set the TYPST_PATH environment variable."
                    .to_string(),
            )
        })?;
        Ok(Self { typst_path })
    }

    /// Construct with a specific typst executable path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            typst_path: path.into(),
        }
    }

    /// Cross-platform detection of Typst executable (Linux, macOS, Windows)
    fn find_typst() -> Option<PathBuf> {
        // 1. Check explicit environment variable TYPST_PATH
        if let Some(env_path) = std::env::var_os("TYPST_PATH") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return Some(p);
            }
        }

        // 2. Test if `typst` is directly callable on PATH (works on Linux, macOS, Windows)
        if let Ok(output) = Command::new("typst").arg("--version").output()
            && output.status.success()
        {
            return Some(PathBuf::from("typst"));
        }

        // 3. Platform-specific fallbacks
        #[cfg(windows)]
        {
            // Try `where.exe typst` on Windows
            if let Ok(output) = Command::new("where").arg("typst").output() {
                if output.status.success() {
                    let path_str = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = path_str.lines().next() {
                        let trimmed = first_line.trim();
                        if !trimmed.is_empty() {
                            return Some(PathBuf::from(trimmed));
                        }
                    }
                }
            }

            // Check %USERPROFILE%\.cargo\bin\typst.exe
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                let cargo_bin = PathBuf::from(user_profile).join(".cargo\\bin\\typst.exe");
                if cargo_bin.exists() {
                    return Some(cargo_bin);
                }
            }

            // Check common local app data
            if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
                let scoop_bin = PathBuf::from(&local_app_data).join("Programs\\typst\\typst.exe");
                if scoop_bin.exists() {
                    return Some(scoop_bin);
                }
            }
        }

        #[cfg(not(windows))]
        {
            // Try `which typst` on Unix / macOS
            if let Ok(output) = Command::new("which").arg("typst").output()
                && output.status.success()
            {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }

            // Check ~/.cargo/bin/typst
            if let Some(home) = std::env::var_os("HOME") {
                let cargo_bin = PathBuf::from(&home).join(".cargo/bin/typst");
                if cargo_bin.exists() {
                    return Some(cargo_bin);
                }

                // Check macOS Homebrew paths
                let homebrew_arm = PathBuf::from("/opt/homebrew/bin/typst");
                if homebrew_arm.exists() {
                    return Some(homebrew_arm);
                }

                let homebrew_intel = PathBuf::from("/usr/local/bin/typst");
                if homebrew_intel.exists() {
                    return Some(homebrew_intel);
                }
            }
        }

        None
    }

    /// Compile a Typst presentation file into a complete `SlideDeck`
    pub fn compile_file(
        &self,
        typ_file: &Path,
    ) -> Result<SlideDeck> {
        if !typ_file.exists() {
            return Err(SlideError::Compilation(format!(
                "File does not exist: {}",
                typ_file.display()
            )));
        }

        let temp_dir = tempdir()?;
        let output_template = temp_dir.path().join("slide-{p}.svg");
        let output_str = output_template.to_string_lossy().to_string();

        let mut cmd = Command::new(&self.typst_path);
        cmd.arg("compile").arg(typ_file).arg(&output_str);

        // Safe root directory determination: handle "slides.typ" vs "path/to/slides.typ"
        let root_dir = typ_file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(
                || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                std::path::Path::to_path_buf,
            );

        // Preprocess any SQLite database and JSON chart queries so Typst can load cached CSV
        preprocess_charts(typ_file, &root_dir);

        // Auto-detect project font directories and TYPST_FONT_PATHS for cross-platform deterministic rendering
        let fonts_dir = root_dir.join("fonts");
        if fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&fonts_dir);
        }
        let assets_fonts_dir = root_dir.join("assets").join("fonts");
        if assets_fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&assets_fonts_dir);
        }
        if let Ok(extra_fonts) = std::env::var("TYPST_FONT_PATHS") {
            for p in std::env::split_paths(&extra_fonts) {
                cmd.arg("--font-path").arg(p);
            }
        }

        cmd.arg("--root").arg(&root_dir);

        let output = cmd.output().map_err(|e| {
            SlideError::Compilation(format!(
                "Failed to execute typst at {:?}: {}",
                self.typst_path, e
            ))
        })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            let full_err = if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            };
            crate::logger::log_event(
                "error",
                &format!("Typst compilation failed:\n{full_err}"),
                Some(serde_json::json!({
                    "stage": "typst_compile",
                    "status": "failed",
                    "error": full_err,
                    "file": typ_file.display().to_string(),
                })),
            );
            return Err(SlideError::Compilation(format!(
                "Typst compilation failed:\n{full_err}"
            )));
        } else if !stderr.trim().is_empty() {
            let (warn_text, has_font_warning, missing_fonts) = format_typst_warnings(&stderr);
            crate::logger::log_event(
                "warn",
                &warn_text,
                Some(serde_json::json!({
                    "stage": "typst_compile",
                    "status": "warning",
                    "warning": stderr.trim(),
                    "has_font_warning": has_font_warning,
                    "missing_fonts": missing_fonts,
                    "file": typ_file.display().to_string(),
                })),
            );
        }

        // Collect all generated SVGs in numerical page order
        let mut slides = Vec::new();
        let mut page_num = 1;
        loop {
            let svg_file = temp_dir.path().join(format!("slide-{page_num}.svg"));
            if !svg_file.exists() {
                break;
            }

            let raw_svg = std::fs::read_to_string(&svg_file)?;
            let svg_content = crate::svg::sanitize_svg(&raw_svg);
            let svg_info = crate::svg::parse_svg_slide_with_root(&svg_content, Some(&root_dir))?;

            slides.push(Slide {
                page_number: page_num,
                svg_data: svg_content,
                view_box: svg_info.view_box,
                hotspots: svg_info.hotspots,
                animation: svg_info.transition,
                steps: svg_info.steps,
            });

            page_num += 1;
        }

        if slides.is_empty() {
            return Err(SlideError::Compilation(
                "No slides were generated from the Typst document.".to_string(),
            ));
        }

        // Check for Typst slide content overflow
        check_slide_overflow(typ_file, &root_dir, &slides)?;

        let title = typ_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Slide Deck")
            .to_string();

        let mut deck = SlideDeck::new(title);
        deck.slides = slides;

        Ok(deck)
    }

    /// Compile a Typst presentation directly to PDF
    pub fn compile_to_pdf(
        &self,
        typ_file: &Path,
        output_pdf: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.typst_path);
        cmd.arg("compile").arg(typ_file).arg(output_pdf);

        let root_dir = typ_file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(
                || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                std::path::Path::to_path_buf,
            );

        let fonts_dir = root_dir.join("fonts");
        if fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&fonts_dir);
        }
        let assets_fonts_dir = root_dir.join("assets").join("fonts");
        if assets_fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&assets_fonts_dir);
        }
        if let Ok(extra_fonts) = std::env::var("TYPST_FONT_PATHS") {
            for p in std::env::split_paths(&extra_fonts) {
                cmd.arg("--font-path").arg(p);
            }
        }

        cmd.arg("--root").arg(&root_dir);

        let output = cmd
            .output()
            .map_err(|e| SlideError::Compilation(format!("Failed to execute typst: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(SlideError::Compilation(format!(
                "Typst compilation failed:\n{stderr}"
            )));
        } else if !stderr.trim().is_empty() {
            let (warn_text, has_font_warning, missing_fonts) = format_typst_warnings(&stderr);
            crate::logger::log_event(
                "warn",
                &warn_text,
                Some(serde_json::json!({
                    "stage": "typst_compile_pdf",
                    "status": "warning",
                    "warning": stderr.trim(),
                    "has_font_warning": has_font_warning,
                    "missing_fonts": missing_fonts,
                    "file": typ_file.display().to_string(),
                })),
            );
        }

        Ok(())
    }
}

/// Parse and format Typst compiler stderr output, segregating font fallback notices from critical warnings
#[must_use]
pub fn format_typst_warnings(stderr: &str) -> (String, bool, Vec<String>) {
    let mut missing_fonts = Vec::new();
    let mut other_blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut is_font_block = false;
    let mut font_name = String::new();

    for line in stderr.lines() {
        if line.starts_with("warning:") || line.starts_with("error:") {
            if !current_block.is_empty() {
                if is_font_block && !font_name.is_empty() {
                    if !missing_fonts.contains(&font_name) {
                        missing_fonts.push(font_name.clone());
                    }
                } else {
                    other_blocks.push(current_block.join("\n"));
                }
                current_block.clear();
            }
            if let Some(rest) = line.strip_prefix("warning: unknown font family:") {
                is_font_block = true;
                font_name = rest.trim().to_string();
            } else {
                is_font_block = false;
                font_name.clear();
            }
        }
        current_block.push(line);
    }

    if !current_block.is_empty() {
        if is_font_block && !font_name.is_empty() {
            if !missing_fonts.contains(&font_name) {
                missing_fonts.push(font_name);
            }
        } else {
            other_blocks.push(current_block.join("\n"));
        }
    }

    let mut output = String::new();
    if !other_blocks.is_empty() {
        output.push_str(&format!(
            "⚠️ Typst compiler warning:\n{}\n",
            other_blocks.join("\n\n")
        ));
    }

    let has_font_warnings = !missing_fonts.is_empty();
    if has_font_warnings {
        if !other_blocks.is_empty() {
            output.push('\n');
        }
        output.push_str(&format!(
            "ℹ️ Typst font fallback notice:\n   The following font families were not found: {}\n   Typst automatically falls back to available system fonts.\n💡 Tip: Missing fonts will fall back to system defaults. You can bundle custom fonts by placing .ttf or .otf files into the 'fonts/' or 'assets/fonts/' directory of your project.",
            missing_fonts.join(", ")
        ));
    }

    (output.trim().to_string(), has_font_warnings, missing_fonts)
}

#[derive(Debug, Clone)]
pub struct DeclaredSlide {
    pub line_number: usize,
    pub title: String,
}

#[must_use]
pub fn parse_declared_slides(
    typ_file: &Path,
    _root_dir: &Path,
) -> Vec<DeclaredSlide> {
    let content = match std::fs::read_to_string(typ_file) {
        | Ok(c) => c,
        | Err(_) => return Vec::new(),
    };

    let mut declared = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }

        // Title slide declaration
        if trimmed.starts_with("#title-slide") && !trimmed.starts_with("#let title-slide") {
            let mut title = "Title Slide".to_string();
            for l in &lines[idx..lines.len().min(idx + 10)] {
                if let Some(t_idx) = l.find("title:") {
                    let rest = &l[t_idx + 6..];
                    if let Some(q1) = rest.find('"')
                        && let Some(q2) = rest[q1 + 1..].find('"')
                    {
                        title = rest[q1 + 1..q1 + 1 + q2].to_string();
                        break;
                    }
                }
            }
            declared.push(DeclaredSlide {
                line_number: idx + 1,
                title,
            });
        } else if trimmed.starts_with("#slide") && !trimmed.starts_with("#let slide") {
            let mut title = format!("Slide {}", declared.len() + 1);
            for l in &lines[idx..lines.len().min(idx + 10)] {
                if let Some(t_idx) = l.find("title:") {
                    let rest = &l[t_idx + 6..];
                    if let Some(q1) = rest.find('"')
                        && let Some(q2) = rest[q1 + 1..].find('"')
                    {
                        title = rest[q1 + 1..q1 + 1 + q2].to_string();
                        break;
                    }
                }
            }
            declared.push(DeclaredSlide {
                line_number: idx + 1,
                title,
            });
        }
    }

    declared
}

pub fn check_slide_overflow(
    typ_file: &Path,
    root_dir: &Path,
    slides: &[Slide],
) -> Result<()> {
    let declared = parse_declared_slides(typ_file, root_dir);
    if declared.is_empty() {
        return Ok(());
    }

    let compiled_count = slides.len();
    let declared_count = declared.len();

    if compiled_count > declared_count {
        let extra = compiled_count - declared_count;

        let mut problem_idx = declared_count.saturating_sub(1);
        let mut spillover_page = compiled_count;

        // Detect spillover pages using slide-meta markers embedded by slide.typ
        let has_any_meta = slides.iter().any(|s| s.svg_data.contains("slide-meta:"));
        if has_any_meta {
            let mut current_decl_idx: usize = 0;
            for (p_i, s) in slides.iter().enumerate() {
                if s.svg_data.contains("slide-meta:") {
                    current_decl_idx += 1;
                } else {
                    // This compiled page has no slide-meta declaration, so it is a spillover of the preceding slide
                    problem_idx = current_decl_idx.saturating_sub(1);
                    spillover_page = p_i + 1;
                    break;
                }
            }
        }

        let problem_slide = &declared[problem_idx.min(declared.len().saturating_sub(1))];

        let msg = format!(
            "Typst slide content overflow detected!\n\
             The presentation source declares {} slide(s), but Typst compiled {} pages ({} extra spillover page(s)).\n\n\
             Overflow location:\n  \
             Slide {} (\"{}\", line {}) exceeded the vertical 16:9 canvas bounds.\n  \
             Spillover content pushed onto compiled page {}.\n\n\
             Remediation suggestions:\n  \
             1. Reduce component heights (e.g. adjust chart height or image dimensions).\n  \
             2. Reduce vertical spacing (e.g. change #v(...) to a smaller distance) or font size.\n  \
             3. Split the content into an additional slide using #slide(title: \"...\").\n\n\
             (To bypass this error, set CARGO_SLIDE_ALLOW_OVERFLOW=1 in your environment)",
            declared_count,
            compiled_count,
            extra,
            problem_idx + 1,
            problem_slide.title,
            problem_slide.line_number,
            spillover_page
        );

        if let Ok(val) = std::env::var("CARGO_SLIDE_ALLOW_OVERFLOW")
            && (val == "1" || val.eq_ignore_ascii_case("true"))
        {
            crate::logger::log_event(
                "warn",
                &format!("\n⚠️  WARNING: {msg}\n"),
                Some(serde_json::json!({
                    "event": "slide_overflow_warning",
                    "declared_slides": declared_count,
                    "compiled_pages": compiled_count,
                    "overflow_slide": problem_idx + 1,
                    "overflow_title": &problem_slide.title,
                    "line": problem_slide.line_number,
                })),
            );
            return Ok(());
        }

        crate::logger::log_event(
            "error",
            &msg,
            Some(serde_json::json!({
                "event": "slide_overflow_error",
                "declared_slides": declared_count,
                "compiled_pages": compiled_count,
                "overflow_slide": problem_idx + 1,
                "overflow_title": &problem_slide.title,
                "line": problem_slide.line_number,
                "spillover_page": spillover_page,
            })),
        );

        return Err(SlideError::Overflow(msg));
    }

    Ok(())
}

fn preprocess_charts(
    typ_file: &Path,
    root_dir: &Path,
) {
    use crate::chart::ChartData;
    use crate::chart::ChartType;

    let content = match std::fs::read_to_string(typ_file) {
        | Ok(c) => c,
        | Err(_) => return,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(src_start) = trimmed.find("source:") {
            let rest = &trimmed[src_start + 7..];
            if let Some(quote_start) = rest.find('"')
                && let Some(quote_end) = rest[quote_start + 1..].find('"')
            {
                let source_val = &rest[quote_start + 1..quote_start + 1 + quote_end];

                let (db_path_str, query_opt) = match source_val.split_once('?') {
                    | Some((p, q)) => {
                        let query_str = q.strip_prefix("query=").unwrap_or(q);
                        (p, Some(query_str.replace('+', " ")))
                    },
                    | None => (source_val, None),
                };

                let file_path = if Path::new(db_path_str).is_absolute() {
                    PathBuf::from(db_path_str)
                } else {
                    root_dir.join(db_path_str)
                };

                if file_path.exists() {
                    let query = query_opt.unwrap_or_else(|| "SELECT * FROM data".to_string());
                    let chart_data_res = if db_path_str.ends_with(".db")
                        || db_path_str.ends_with(".sqlite")
                    {
                        ChartData::from_sqlite(&file_path, &query, ChartType::Bar, None)
                    } else if db_path_str.ends_with(".json") || db_path_str.ends_with(".jsonl") {
                        ChartData::from_json_file(&file_path, ChartType::Bar, None)
                    } else if db_path_str.ends_with(".csv") {
                        ChartData::from_csv_file(&file_path, ChartType::Bar, None)
                    } else {
                        continue;
                    };

                    if let Ok(data) = chart_data_res {
                        let new_csv = data.to_csv();
                        let cache_csv = format!("{}.cache.csv", file_path.display());
                        let should_write = std::fs::read_to_string(&cache_csv)
                            .map_or(true, |existing| existing != new_csv);
                        if should_write {
                            let _ = std::fs::write(&cache_csv, &new_csv);
                        }
                        if db_path_str.ends_with(".db") || db_path_str.ends_with(".sqlite") {
                            let alt_cache = file_path.with_extension("db.cache.csv");
                            let should_write_alt = std::fs::read_to_string(&alt_cache)
                                .map_or(true, |existing| existing != new_csv);
                            if should_write_alt {
                                let _ = std::fs::write(&alt_cache, &new_csv);
                            }
                        }
                    }
                }
            }
        }
    }
}
