use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary directory within the workspace `target/tmp` directory
/// to avoid exhausting small in-memory `/tmp` tmpfs mounts.
pub fn test_temp_dir() -> TempDir {
    let base = std::env::var("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(manifest).join("../../target/tmp")
        });
    let _ = std::fs::create_dir_all(&base);
    tempfile::tempdir_in(&base).unwrap_or_else(|_| tempfile::tempdir().unwrap())
}
