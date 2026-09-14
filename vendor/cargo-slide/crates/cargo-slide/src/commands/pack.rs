//! Command to package a presentation into a standalone compressed .slide archive.

use slide_core::compiler::SlideCompiler;
use slide_core::package::pack_deck_with_assets;
use std::path::Path;
use std::path::PathBuf;

/// Execute the `pack` command.
///
/// Compiles the presentation slides and archives them into a standalone `.slide` file
/// using maximum LZMA2 compression.
#[allow(clippy::pedantic, clippy::nursery, clippy::cast_precision_loss)]
pub fn execute(
    file: &Path,
    output: Option<PathBuf>,
    animation: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let out_file = output.unwrap_or_else(|| PathBuf::from(format!("{stem}.slide")));

    slide_core::logger::log_event(
        "info",
        &format!(
            "📦 Packaging presentation into .slide archive: {}",
            file.display()
        ),
        Some(serde_json::json!({
            "stage": "pack_start",
            "source_file": file.display().to_string(),
            "output_file": out_file.display().to_string(),
        })),
    );

    let compiler = SlideCompiler::new()?;
    let mut deck = compiler.compile_file(file)?;

    if !animation.is_empty() {
        deck.default_animation = animation.to_string();
    }

    let base_dir = file.parent();
    pack_deck_with_assets(&deck, base_dir, &out_file)?;

    let size_bytes = std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
    let size_kb = (size_bytes as f64) / 1024.0;

    slide_core::logger::log_event(
        "success",
        &format!(
            "✅ Packaged {} slides into {} ({:.1} KB, LZMA2 extreme)",
            deck.total_slides(),
            out_file.display(),
            size_kb
        ),
        Some(serde_json::json!({
            "stage": "pack_success",
            "total_slides": deck.total_slides(),
            "output_file": out_file.display().to_string(),
            "size_bytes": size_bytes,
        })),
    );

    Ok(())
}
