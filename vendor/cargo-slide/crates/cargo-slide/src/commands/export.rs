use slide_core::compiler::SlideCompiler;
use std::fs::create_dir_all;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;

/// Export presentation to PDF or SVG vector files
pub fn execute(
    file: &Path,
    format: &str,
    output: Option<PathBuf>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let compiler = SlideCompiler::new()?;

    match format.to_lowercase().as_str() {
        | "pdf" => {
            let out_pdf = output.unwrap_or_else(|| PathBuf::from(format!("{stem}.pdf")));
            slide_core::logger::log_event(
                "info",
                &format!("📄 Exporting to PDF: {}", out_pdf.display()),
                Some(serde_json::json!({
                    "stage": "export_pdf_start",
                    "output": out_pdf.display().to_string(),
                })),
            );
            compiler.compile_to_pdf(file, &out_pdf)?;
            slide_core::logger::log_event(
                "success",
                &format!("✅ PDF exported successfully: {}", out_pdf.display()),
                Some(serde_json::json!({
                    "stage": "export_pdf_success",
                    "output": out_pdf.display().to_string(),
                })),
            );
        },
        | "svg" => {
            let out_dir = output.unwrap_or_else(|| PathBuf::from(format!("{stem}-svgs")));
            create_dir_all(&out_dir)?;
            slide_core::logger::log_event(
                "info",
                &format!("🎨 Exporting SVGs to directory: {}", out_dir.display()),
                Some(serde_json::json!({
                    "stage": "export_svg_start",
                    "output_dir": out_dir.display().to_string(),
                })),
            );
            let deck = compiler.compile_file(file)?;
            for slide in &deck.slides {
                let page_num = slide.page_number;
                let svg_file = out_dir.join(format!("page-{page_num}.svg"));
                write(&svg_file, &slide.svg_data)?;
            }
            slide_core::logger::log_event(
                "success",
                &format!(
                    "✅ {} SVG slides exported successfully to {}",
                    deck.total_slides(),
                    out_dir.display()
                ),
                Some(serde_json::json!({
                    "stage": "export_svg_success",
                    "total_slides": deck.total_slides(),
                    "output_dir": out_dir.display().to_string(),
                })),
            );
        },
        | other => {
            return Err(
                format!("Unsupported export format: {other}. Supported formats: pdf, svg").into(),
            );
        },
    }

    Ok(())
}
