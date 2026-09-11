use std::path::Path;
use std::process::Command;
use std::process::Stdio;

/// Media player interface for video placeholders
pub struct MediaPlayer;

impl MediaPlayer {
    /// Play a video file using FFmpeg (ffplay), mpv, or system default opener in a background thread
    /// Fully compatible across Linux, macOS, and Windows without blocking the main event loop.
    pub fn play_video(source: &str) -> std::io::Result<()> {
        let source_str = source.to_string();
        let _ = std::thread::spawn(move || {
            let video_path = Path::new(&source_str);
            if !source_str.contains("://") && !video_path.exists() {
                eprintln!("⚠️ Video file not found: {}", source_str);
            }

            // 1. Try ffplay in a native window with titlebar, controls, and auto-exit
            let mut cmd = Command::new("ffplay");
            cmd.arg("-autoexit")
                .arg("-window_title")
                .arg("Cargo Slide Video Player")
                .arg(video_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            if let Ok(mut child) = cmd.spawn()
                && let Ok(status) = child.wait()
                && status.success()
            {
                return;
            }

            // 2. Fallback to mpv if ffplay is not installed or failed
            let mut mpv_cmd = Command::new("mpv");
            mpv_cmd
                .arg(video_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            if let Ok(mut child) = mpv_cmd.spawn()
                && let Ok(status) = child.wait()
                && status.success()
            {
                return;
            }

            // 3. Fallback to system default video opener
            crate::window::open_external_target_detached(&source_str);
        });

        Ok(())
    }
}
