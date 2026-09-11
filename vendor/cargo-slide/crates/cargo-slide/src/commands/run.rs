use slide_core::compiler::SlideCompiler;
use slide_player::PlayerConfig;
use slide_player::SlidePlayer;
use std::path::Path;

/// Launch the presentation viewer
pub fn execute(
    file: &Path,
    animation: &str,
    watch: bool,
    fullscreen: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    slide_core::logger::log_event(
        "info",
        &format!("🎬 Compiling {}...", file.display()),
        Some(serde_json::json!({
            "stage": "compile_start",
            "file": file.display().to_string(),
        })),
    );
    let compiler = SlideCompiler::new()?;
    let deck = compiler.compile_file(file)?;

    slide_core::logger::log_event(
        "success",
        &format!(
            "✨ Presentation compiled successfully! {} slides loaded.",
            deck.total_slides()
        ),
        Some(serde_json::json!({
            "stage": "compile_success",
            "total_slides": deck.total_slides(),
            "title": &deck.title,
        })),
    );

    let config = PlayerConfig {
        title: deck.title.clone(),
        default_animation: animation.to_string(),
        fullscreen,
        watch,
        ..Default::default()
    };

    slide_core::logger::log_event(
        "info",
        "🚀 Launching interactive GUI presentation player...",
        Some(serde_json::json!({
            "stage": "player_launch",
            "fullscreen": fullscreen,
            "animation": animation,
            "watch": watch,
        })),
    );
    let canonical_file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());

    SlidePlayer::new(deck)
        .with_config(config)
        .with_source_file(Some(canonical_file))
        .with_watch(watch)
        .run()?;

    Ok(())
}
