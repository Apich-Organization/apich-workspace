use slide_core::logger::log_event;
use std::fs::create_dir_all;
use std::fs::write;
use std::path::Path;

/// Initialize a new presentation project workspace
#[allow(clippy::too_many_lines)]
pub fn execute(
    target_path: &Path,
    include_rust: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let project_dir = if target_path == Path::new(".") {
        std::env::current_dir()?
    } else {
        if !target_path.exists() {
            create_dir_all(target_path)?;
        }
        target_path.to_path_buf()
    };

    let dir_name = project_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");

    log_event(
        "info",
        &format!(
            "✨ Initializing cargo-slide presentation workspace in: {}",
            project_dir.display()
        ),
        Some(serde_json::json!({
            "event": "init_start",
            "path": project_dir.display().to_string(),
            "rust_extensions": include_rust,
        })),
    );

    let assets_dir = project_dir.join("assets");
    create_dir_all(&assets_dir)?;

    // 1. Write slides.typ
    let slides_path = project_dir.join("slides.typ");
    if !slides_path.exists() {
        write(&slides_path, slide_theme::DEFAULT_PRESENTATION)?;
    }

    // 2. Write theme.typ & slide.typ
    let theme_path = project_dir.join("theme.typ");
    if !theme_path.exists() {
        write(&theme_path, slide_theme::DEFAULT_THEME)?;
    }

    let slide_macro_path = project_dir.join("slide.typ");
    if !slide_macro_path.exists() {
        write(&slide_macro_path, slide_theme::SLIDE_MACROS)?;
    }

    // 3. Write assets/data.csv starter dataset
    let sample_csv_path = assets_dir.join("data.csv");
    if !sample_csv_path.exists() {
        let sample_csv = "Framework,FPS,Memory_MB,Startup_ms\ncargo-slide,60,18,42\nNative App,60,95,320\nMarp,30,110,680\nElectron,45,340,1250\n";
        write(&sample_csv_path, sample_csv)?;
    }

    // 4. Write .gitignore
    let gitignore_path = project_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let gitignore = "/target\n*.pdf\n*-svgs/\n*-presentation\n*.cache.csv\n";
        write(&gitignore_path, gitignore)?;
    }

    // 5. Optional Rust extension scaffolding
    if include_rust {
        let src_dir = project_dir.join("src");
        create_dir_all(&src_dir)?;

        let cargo_toml_path = project_dir.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            let cargo_toml = format!(
                r#"[package]
name = "{dir_name}"
version = "0.1.0"
edition = "2024"

[dependencies]
slide-core = {{ version = "0.1" }}
slide-player = {{ version = "0.1" }}
serde_json = "1.0"
"#
            );
            write(&cargo_toml_path, cargo_toml)?;
        }

        let main_rs_path = src_dir.join("main.rs");
        if !main_rs_path.exists() {
            let main_rs = r#"use slide_player::prelude::*;
use std::time::Duration;

/// Custom page transition example implementing SlideAnimation
pub struct CustomSpiralAnimation;

impl SlideAnimation for CustomSpiralAnimation {
    fn name(&self) -> &str {
        "spiral"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let width = ctx.width;
        let height = ctx.height;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;

        for y in 0..height {
            let dy = y as f32 - cy;
            let row_start = y * width;
            for x in 0..width {
                let dx = x as f32 - cx;
                let angle = (dy.atan2(dx) + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
                let radius = (dx * dx + dy * dy).sqrt() / cx.max(cy);
                let spiral_val = (angle + radius * 2.0).fract();

                let pixel = if t >= spiral_val {
                    to.get_pixel(x, y)
                } else if let Some(from_surf) = from {
                    from_surf.get_pixel(x, y)
                } else {
                    0
                };
                ctx.buffer[row_start + x] = pixel;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Running presentation with custom Rust animations...");
    SlideApp::new("slides.typ")
        .default_animation("spiral")
        .register_animation(CustomSpiralAnimation)
        .run()?;
    Ok(())
}
"#;
            write(&main_rs_path, main_rs)?;
        }
    }

    let mut created_files = vec![
        "slides.typ",
        "theme.typ",
        "slide.typ",
        "assets/data.csv",
        ".gitignore",
    ];
    if include_rust {
        created_files.push("Cargo.toml");
        created_files.push("src/main.rs");
    }

    log_event(
        "success",
        &format!(
            "🎉 Presentation workspace initialized successfully in {}!\n\n\
             Workspace Files:\n  \
             📄 slides.typ      - Main presentation deck (Pure Typst, no Rust required!)\n  \
             🎨 theme.typ       - Visual theme & styling configuration\n  \
             📦 slide.typ       - Component & layout macros (#slide, #step, #chart, #video, #callout)\n  \
             📊 assets/data.csv - Starter dataset for interactive charts\n  \
             🛡️  .gitignore      - Build output exclusions\n\n\
             Quick Commands:\n  \
             cargo slide run    - Run interactive presentation player (60 FPS)\n  \
             cargo slide dev    - Run with live hot-reloading on file edit\n  \
             cargo slide build  - Compile into standalone single executable binary\n  \
             cargo slide export - Export slides to PDF or SVG vector files\n",
            project_dir.display()
        ),
        Some(serde_json::json!({
            "event": "init_success",
            "path": project_dir.display().to_string(),
            "files": created_files,
        })),
    );

    Ok(())
}
