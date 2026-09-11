use slide_core::compiler::SlideCompiler;
use std::fs::create_dir_all;
use std::fs::write;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use tempfile::tempdir;

/// Compile the presentation into a standalone release binary. `target`, when given, cross-compiles
/// for that Rust target triple (e.g. `x86_64-pc-windows-msvc`) instead of the host's own platform;
/// the triple must already be installed via `rustup target add` and have a working linker
/// configured for a non-host target.
#[allow(clippy::too_many_lines)]
pub fn execute(
    file: &Path,
    output: Option<PathBuf>,
    animation: &str,
    target: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // The *target* platform decides the binary's extension, not the host `cargo-slide` itself
    // runs on (`cfg!(windows)` reflects the host, which is always false when cross-compiling
    // from this Linux sandbox to a Windows target) -- a real bug in the original upstream
    // implementation, fixed here as part of adding cross-compilation support at all.
    let targets_windows = target.is_some_and(|t| t.contains("windows"));
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    slide_core::logger::log_event(
        "info",
        &format!(
            "📦 Packaging presentation into single binary: {}",
            file.display()
        ),
        Some(serde_json::json!({
            "stage": "package_start",
            "source_file": file.display().to_string(),
            "percent": 1,
        })),
    );
    let compiler = SlideCompiler::new()?;
    let deck = compiler.compile_file(file)?;

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let default_name = if targets_windows {
        format!("{stem}-presentation.exe")
    } else {
        format!("{stem}-presentation")
    };
    let target_binary = output.unwrap_or_else(|| PathBuf::from(default_name));

    slide_core::logger::log_event(
        "info",
        &format!(
            "⏳ Generating self-contained Rust bundle with {} slides...",
            deck.total_slides()
        ),
        Some(serde_json::json!({
            "stage": "bundle_generate",
            "total_slides": deck.total_slides(),
            "percent": 3,
        })),
    );

    // Discover repository root for workspace dependencies
    let current_exe = std::env::current_exe().unwrap_or_default();
    let repo_root = find_repo_root(&current_exe)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Create temporary build project in target/.build_tmp if possible to avoid tmpfs size exhaustion
    let build_dir_parent = if repo_root.join("target").exists() {
        let p = repo_root.join("target/.build_tmp");
        let _ = create_dir_all(&p);
        p
    } else {
        std::env::temp_dir()
    };
    let build_dir = tempfile::Builder::new()
        .prefix("slide_build_")
        .tempdir_in(&build_dir_parent)
        .or_else(|_| tempdir())?;
    let build_path = build_dir.path();
    let src_dir = build_path.join("src");
    create_dir_all(&src_dir)?;

    // Serialize deck to JSON to be embedded via include_str!
    let deck_json = serde_json::to_string(&deck)?;
    write(build_path.join("deck.json"), deck_json)?;

    let core_path = repo_root.join("crates/slide-core");
    let player_path = repo_root.join("crates/slide-player");

    // Format paths with forward slashes for cross-platform TOML compatibility (avoids backslash escaping on Windows)
    let core_str = core_path.to_string_lossy().replace('\\', "/");
    let player_str = player_path.to_string_lossy().replace('\\', "/");

    let pkg_version = env!("CARGO_PKG_VERSION");
    let (core_dep, player_dep) = if core_path.exists() && player_path.exists() {
        (
            format!(r#"slide-core = {{ path = "{core_str}", version = "{pkg_version}" }}"#),
            format!(r#"slide-player = {{ path = "{player_str}", version = "{pkg_version}" }}"#),
        )
    } else {
        (
            format!(r#"slide-core = "{pkg_version}""#),
            format!(r#"slide-player = "{pkg_version}""#),
        )
    };

    let cargo_toml = format!(
        r#"[package]
name = "slide-standalone-runner"
version = "{pkg_version}"
edition = "2024"

[workspace]

[dependencies]
{core_dep}
{player_dep}
serde_json = "1.0"
"#
    );
    write(build_path.join("Cargo.toml"), cargo_toml)?;

    let runner_main = format!(
        r#"use slide_player::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let deck_json = include_str!("../deck.json");
    let deck: SlideDeck = serde_json::from_str(deck_json)?;

    SlideApp::from_deck(deck)
        .default_animation("{animation}")
        .run()?;

    Ok(())
}}
"#
    );
    write(src_dir.join("main.rs"), runner_main)?;

    slide_core::logger::log_event(
        "info",
        "🔨 Compiling release binary with Cargo...",
        Some(serde_json::json!({
            "stage": "cargo_build_release",
            "percent": 5,
        })),
    );
    let target_dir = repo_root.join("target/standalone_target");
    run_cargo_build_with_progress(build_path, &target_dir, target)?;

    let bin_filename = if targets_windows {
        "slide-standalone-runner.exe"
    } else {
        "slide-standalone-runner"
    };
    // `cargo build --target <triple>` (even for the host's own triple) nests output under
    // `<target-dir>/<triple>/release/`, not `<target-dir>/release/` -- only the no-`--target`
    // invocation uses the flat layout.
    let release_dir = match target {
        Some(triple) => target_dir.join(triple).join("release"),
        None => target_dir.join("release"),
    };
    let built_bin = release_dir.join(bin_filename);
    if !built_bin.exists() {
        return Err(format!("Compiled binary not found at {}", built_bin.display()).into());
    }

    // Copy to target location
    std::fs::copy(&built_bin, &target_binary)?;

    // Ensure executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&target_binary) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&target_binary, perms);
        }
    }

    let size_bytes = std::fs::metadata(&target_binary).map_or(0, |m| m.len());
    #[allow(clippy::cast_precision_loss)]
    let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

    slide_core::logger::log_event(
        "success",
        &format!(
            "✅ Single binary generated successfully!\n\
             📍 Output: {} ({:.2} MB)\n\
             💡 You can now distribute and run this standalone presentation on any machine!",
            target_binary.display(),
            size_mb
        ),
        Some(serde_json::json!({
            "stage": "package_complete",
            "status": "success",
            "output": target_binary.display().to_string(),
            "size_mb": size_mb,
            "total_slides": deck.total_slides(),
            "percent": 100,
        })),
    );

    Ok(())
}

fn find_repo_root(exe: &Path) -> Option<PathBuf> {
    // 1. Explicit environment variable override
    if let Ok(env_root) = std::env::var("CARGO_SLIDE_REPO_ROOT") {
        let p = PathBuf::from(env_root);
        if p.join("crates").join("slide-core").exists() {
            return Some(p);
        }
    }

    // 2. Search upwards from current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = Some(cwd.as_path());
        while let Some(dir) = cur {
            if dir.join("crates").join("slide-core").exists() {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }

    // 3. Search upwards from compile-time manifest dir (for local dev installs)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut cur = Some(manifest_dir);
    while let Some(dir) = cur {
        if dir.join("crates").join("slide-core").exists() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }

    // 4. Search upwards from executable location
    let mut cur = exe.parent();
    while let Some(dir) = cur {
        if dir.join("crates").join("slide-core").exists() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }

    None
}

/// Resolved package-graph size for `build_path`'s generated standalone-runner project, used as a
/// rough denominator for build progress (see `run_cargo_build_with_progress`) -- not an exact
/// rustc-invocation count (a build-script or proc-macro crate can compile more than once), just
/// close enough that a caller polling `--log-format json`'s progress events sees a percentage
/// that moves at roughly the right pace instead of standing still for minutes then jumping to
/// 100%. Returns 0 (caller then treats every artifact as 1% until 100 are seen) if `cargo
/// metadata` itself fails for any reason, rather than aborting the whole build over a
/// progress-estimate step that was never essential to begin with.
fn estimate_total_units(build_path: &Path, target: Option<&str>) -> usize {
    let mut cmd = Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(build_path);
    if let Some(triple) = target {
        cmd.arg("--filter-platform").arg(triple);
    }
    let Ok(output) = cmd.output() else {
        return 0;
    };
    if !output.status.success() {
        return 0;
    }
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return 0;
    };
    json.get("resolve")
        .and_then(|r| r.get("nodes"))
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

/// Runs the actual `cargo build --release` for the generated standalone-runner project, emitting
/// a real (not simulated) progress percentage as compilation proceeds -- each unit of work cargo
/// itself reports finishing (`--message-format=json`'s `"reason":"compiler-artifact"` events)
/// advances the count, scaled against `estimate_total_units`'s rough total. Reserves 0-5% for
/// work already done before this function is called (Typst compilation, bundle generation) and
/// 95-100% for copying the finished binary out afterward, so a caller watching the percentage
/// climb sees it start above 0 and top out just under 100 while this function itself is running.
///
/// Real compiler diagnostics (errors and warnings) are forwarded too, via `"reason":
/// "compiler-message"` events -- `--message-format=json` alone would otherwise swallow them
/// entirely compared to the plain (non-JSON) invocation this replaces, since cargo routes
/// per-file diagnostics through the JSON message stream rather than stderr in that mode.
fn run_cargo_build_with_progress(
    build_path: &Path,
    target_dir: &Path,
    target: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let total = estimate_total_units(build_path, target);

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("build")
        .arg("--release")
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--message-format=json")
        .current_dir(build_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if let Some(triple) = target {
        cargo_cmd.arg("--target").arg(triple);
    }

    let mut child = cargo_cmd.spawn()?;
    let Some(stdout) = child.stdout.take() else {
        return Err("Failed to capture cargo build output".into());
    };
    let reader = BufReader::new(stdout);

    let mut compiled: usize = 0;
    let mut last_reported_percent: u8 = 0;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(reason) = value.get("reason").and_then(|r| r.as_str()) else {
            continue;
        };
        match reason {
            "compiler-artifact" => {
                compiled = compiled.saturating_add(1);
                let denom = if total == 0 { compiled.max(100) } else { total };
                #[allow(clippy::cast_precision_loss)]
                let fraction = (compiled as f64 / denom as f64).min(1.0);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let percent = (5.0 + fraction * 90.0) as u8;
                if percent != last_reported_percent {
                    last_reported_percent = percent;
                    let pkg_name = value
                        .get("target")
                        .and_then(|t| t.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("dependency");
                    slide_core::logger::log_event(
                        "info",
                        &format!("🔨 Compiling {pkg_name} ({compiled}/{}, cross-target: {})", if total == 0 { "?".to_string() } else { total.to_string() }, target.unwrap_or("host")),
                        Some(serde_json::json!({
                            "stage": "cargo_build_release",
                            "compiled": compiled,
                            "total": total,
                            "percent": percent,
                        })),
                    );
                }
            },
            "compiler-message" => {
                let Some(message) = value.get("message") else { continue };
                let level = message.get("level").and_then(|l| l.as_str()).unwrap_or("");
                let Some(rendered) = message.get("rendered").and_then(|r| r.as_str()) else {
                    continue;
                };
                if level == "error" {
                    slide_core::logger::log_event("error", rendered, None);
                }
            },
            _ => {},
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err("Cargo compilation failed for single binary output".into());
    }
    Ok(())
}
