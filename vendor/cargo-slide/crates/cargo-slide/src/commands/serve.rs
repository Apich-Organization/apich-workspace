//! Built-in static web server for hosting Leptos CSR presentation bundles.

use slide_core::compiler::SlideCompiler;
use std::fs::File;
use std::fs::create_dir_all;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server;
use tiny_http::StatusCode;

// Embedded prebuilt Leptos CSR web application assets bundled in cargo-slide's own pkg/ directory
pub const INDEX_HTML: &str = include_str!("../../pkg/index.html");
pub const BOOTSTRAP_JS: &str = include_str!("../../pkg/bootstrap.js");
pub const STYLE_CSS: &str = include_str!("../../pkg/style.css");
pub const SLIDE_WEB_JS: &str = include_str!("../../pkg/slide_web.js");
pub const SLIDE_WEB_WASM: &[u8] = include_bytes!("../../pkg/slide_web_bg.wasm");

/// Prepare a standalone CSR distribution directory with all web player assets and `deck.json`.
#[allow(clippy::pedantic, clippy::nursery)]
pub fn prepare_csr_bundle(
    file: &Path,
    dest_dir: &Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    create_dir_all(dest_dir)?;

    // Write Leptos CSR static player files
    std::fs::write(dest_dir.join("index.html"), INDEX_HTML)?;
    std::fs::write(dest_dir.join("bootstrap.js"), BOOTSTRAP_JS)?;
    std::fs::write(dest_dir.join("style.css"), STYLE_CSS)?;
    std::fs::write(dest_dir.join("slide_web.js"), SLIDE_WEB_JS)?;
    std::fs::write(dest_dir.join("slide_web_bg.wasm"), SLIDE_WEB_WASM)?;

    // Generate deck.json from input file and copy assets
    let deck_json_path = dest_dir.join("deck.json");
    let assets_dest = dest_dir.join("assets");

    if file.extension().and_then(|e| e.to_str()) == Some("slide") {
        let (deck, cache_dir) = slide_core::package::unpack_deck_and_assets(file)?;
        let json = serde_json::to_string_pretty(&deck)?;
        std::fs::write(deck_json_path, json)?;
        copy_dir_all(&cache_dir, dest_dir);
    } else if file.extension().and_then(|e| e.to_str()) == Some("json") {
        std::fs::copy(file, deck_json_path)?;
        if let Some(parent) = file.parent() {
            let src_assets = parent.join("assets");
            if src_assets.is_dir() {
                copy_dir_all(&src_assets, &assets_dest);
            }
        }
    } else {
        let compiler = SlideCompiler::new()?;
        let deck = compiler.compile_file(file)?;
        let json = serde_json::to_string_pretty(&deck)?;
        std::fs::write(deck_json_path, json)?;
        if let Some(parent) = file.parent() {
            let src_assets = parent.join("assets");
            if src_assets.is_dir() {
                copy_dir_all(&src_assets, &assets_dest);
            }

            // Copy any linked local relative files or media into dest_dir
            for slide in &deck.slides {
                for hotspot in &slide.hotspots {
                    let candidate = match hotspot {
                        | slide_core::model::Hotspot::Link { target, .. } => {
                            let clean = target.strip_prefix('#').unwrap_or(target);
                            if clean.starts_with("chart:")
                                || clean.starts_with("video:")
                                || clean.starts_with("audio:")
                                || clean.starts_with("step:")
                                || clean.starts_with("transition:")
                                || clean.starts_with("mailto:")
                                || clean.contains("://")
                                || target.starts_with('#')
                            {
                                None
                            } else {
                                Some(target.strip_prefix("file://").unwrap_or(target))
                            }
                        },
                        | slide_core::model::Hotspot::Video { source, .. }
                        | slide_core::model::Hotspot::Audio { source, .. } => {
                            if source.contains("://") || source.starts_with('#') {
                                None
                            } else {
                                Some(source.strip_prefix("file://").unwrap_or(source))
                            }
                        },
                        | _ => None,
                    };

                    if let Some(raw_rel) = candidate {
                        let rel_clean = raw_rel.replace('\\', "/");
                        let src_path = parent.join(&rel_clean);
                        if src_path.is_file() {
                            let dst_path = dest_dir.join(&rel_clean);
                            if let Some(p) = dst_path.parent() {
                                let _ = create_dir_all(p);
                            }
                            let _ = std::fs::copy(&src_path, dst_path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn copy_dir_all(
    src: &Path,
    dst: &Path,
) {
    let _ = create_dir_all(dst);
    for entry in walkdir::WalkDir::new(src).into_iter().flatten() {
        if entry.file_type().is_file()
            && let Ok(rel) = entry.path().strip_prefix(src)
        {
            let target = dst.join(rel);
            if let Some(p) = target.parent() {
                let _ = create_dir_all(p);
            }
            let _ = std::fs::copy(entry.path(), target);
        }
    }
}

/// Execute the `serve` command to launch an HTTP static web server.
#[allow(clippy::pedantic, clippy::nursery, clippy::too_many_lines)]
pub fn execute(
    file: &Path,
    port: u16,
    ip: &str,
    dir: Option<PathBuf>,
    open_browser: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let serve_dir = if let Some(custom_dir) = dir {
        if !custom_dir.exists() {
            return Err(
                format!("Custom directory does not exist: {}", custom_dir.display()).into(),
            );
        }
        custom_dir
    } else if file.is_dir() {
        file.to_path_buf()
    } else {
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("presentation");
        let dist = PathBuf::from("target").join(".web_serve").join(stem);
        prepare_csr_bundle(file, &dist)?;
        dist
    };

    let bind_addr = format!("{ip}:{port}");
    let server = Server::http(&bind_addr)
        .map_err(|e| format!("Failed to bind static server to {bind_addr}: {e}"))?;

    let base_url = format!("http://{bind_addr}/");
    slide_core::logger::log_event(
        "info",
        &format!("🚀 Cargo Slide Web Server running at {base_url}"),
        Some(serde_json::json!({
            "stage": "server_started",
            "url": base_url,
            "serve_dir": serve_dir.display().to_string(),
        })),
    );

    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  🚀 Cargo Slide Static Web Server (Leptos CSR)        │");
    println!("  │                                                        │");
    println!("  │  📡 URL:         http://{:<30}│", bind_addr);
    println!("  │  📁 Directory:   {:<38}│", serve_dir.display());
    println!("  │  ⚡ Mode:        Pure Rust WASM + CSR (Zero Inline JS) │");
    println!("  │                                                        │");
    println!("  │  Press Ctrl+C to terminate the server                  │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();

    if open_browser {
        let _ = open::that(&base_url);
    }

    for request in server.incoming_requests() {
        handle_http_request(request, &serve_dir);
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn handle_http_request(
    request: tiny_http::Request,
    root_dir: &Path,
) {
    let raw_url = request.url();
    let url_path = raw_url.split('?').next().unwrap_or("/");
    let trimmed_path = url_path.trim_start_matches('/');

    let relative_path = if trimmed_path.is_empty() {
        "index.html"
    } else {
        trimmed_path
    };

    let target_path = root_dir.join(relative_path);

    // Prevent directory traversal
    if target_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        let response = Response::from_string("403 Forbidden").with_status_code(StatusCode(403));
        let _ = request.respond(response);
        return;
    }

    if target_path.exists() && target_path.is_file() {
        let Ok(mut file) = File::open(&target_path) else {
            let resp = Response::from_string("500 Internal Server Error")
                .with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let mut data = Vec::new();
        if file.read_to_end(&mut data).is_err() {
            let resp = Response::from_string("500 Internal Server Error")
                .with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        }

        let mime = get_mime_type(&target_path);
        let Ok(content_type_header) = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
        else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let Ok(cors_header) = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
        else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let Ok(accept_ranges_header) = Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..])
        else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let Ok(cache_header) = Header::from_bytes(
            &b"Cache-Control"[..],
            &b"no-cache, no-store, must-revalidate"[..],
        ) else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        // Support HTTP Range requests (206 Partial Content) for streaming media (audio/video)
        let range_opt = request
            .headers()
            .iter()
            .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("range"))
            .map(|h| h.value.as_str().to_string());

        let total_len = data.len();

        if let Some(range_val) = range_opt
            && let Some(spec) = range_val.strip_prefix("bytes=")
        {
            let parts: Vec<&str> = spec.split('-').collect();
            let start = parts
                .first()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let end = parts
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or_else(|| total_len.saturating_sub(1))
                .min(total_len.saturating_sub(1));

            if start <= end && start < total_len {
                let sliced = &data[start..=end];
                let content_range = format!("bytes {start}-{end}/{total_len}");
                if let Ok(cr_header) =
                    Header::from_bytes(&b"Content-Range"[..], content_range.as_bytes())
                {
                    let response = Response::from_data(sliced.to_vec())
                        .with_status_code(StatusCode(206))
                        .with_header(content_type_header)
                        .with_header(cors_header)
                        .with_header(accept_ranges_header)
                        .with_header(cr_header)
                        .with_header(cache_header);
                    let _ = request.respond(response);
                    return;
                }
            }
        }

        let response = Response::from_data(data)
            .with_header(content_type_header)
            .with_header(cors_header)
            .with_header(accept_ranges_header)
            .with_header(cache_header);

        let _ = request.respond(response);
    } else {
        let not_found = Response::from_string("404 Not Found").with_status_code(StatusCode(404));
        let _ = request.respond(not_found);
    }
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        | Some("html" | "htm") => "text/html; charset=utf-8",
        | Some("js" | "mjs") => "application/javascript; charset=utf-8",
        | Some("wasm") => "application/wasm",
        | Some("css") => "text/css; charset=utf-8",
        | Some("json") => "application/json; charset=utf-8",
        | Some("svg") => "image/svg+xml",
        | Some("png") => "image/png",
        | Some("jpg" | "jpeg") => "image/jpeg",
        | Some("gif") => "image/gif",
        | Some("ico") => "image/x-icon",
        | Some("wav") => "audio/wav",
        | Some("mp3") => "audio/mpeg",
        | Some("ogg") => "audio/ogg",
        | Some("flac") => "audio/flac",
        | Some("mp4" | "m4v") => "video/mp4",
        | Some("webm") => "video/webm",
        | Some("csv") => "text/csv; charset=utf-8",
        | Some("md" | "markdown") => "text/markdown; charset=utf-8",
        | Some("txt" | "rs" | "typ" | "toml" | "yaml" | "yml" | "sql" | "py") => {
            "text/plain; charset=utf-8"
        },
        | Some("db" | "sqlite") => "application/x-sqlite3",
        | _ => "application/octet-stream",
    }
}
