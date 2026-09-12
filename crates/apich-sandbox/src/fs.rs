//! Secure workspace filesystem utilities and traversal defenses.

use crate::error::Result;
use crate::error::SandboxError;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

/// Metadata for a file or directory entry inside a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// File or directory base name.
    pub name: String,
    /// Path relative to the workspace root.
    pub rel_path: PathBuf,
    /// True if entry is a directory.
    pub is_dir: bool,
    /// True if entry is a regular file.
    pub is_file: bool,
    /// True if entry is a symbolic link.
    pub is_symlink: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modification timestamp in UTC.
    pub modified: Option<DateTime<Utc>>,
}

/// Safely resolve a relative path against a base host workspace directory,
/// preventing path traversal attacks (e.g. `../../etc/passwd`).
///
/// # Errors
/// Returns `SandboxError::InvalidPath` if the user path attempts path traversal.
pub fn resolve_safe_path(
    base_dir: &Path,
    user_path: &Path,
) -> Result<PathBuf> {
    // Disallow absolute user paths or paths with Prefix/RootDir/ParentDir escapes
    for comp in user_path.components() {
        match comp {
            | Component::ParentDir => {
                return Err(SandboxError::InvalidPath(user_path.to_path_buf()));
            },
            | Component::Prefix(_)
            | Component::RootDir
            | Component::Normal(_)
            | Component::CurDir => {},
        }
    }

    // Strip leading `/` or `/workspace/` if user provided container path
    let clean_rel = user_path
        .strip_prefix("/workspace")
        .or_else(|_| user_path.strip_prefix("/"))
        .unwrap_or(user_path);

    let mut full_path = base_dir.to_path_buf();
    for comp in clean_rel.components() {
        if let Component::Normal(c) = comp {
            full_path.push(c);
        }
    }

    // Double check base_dir prefix
    if !full_path.starts_with(base_dir) {
        return Err(SandboxError::InvalidPath(user_path.to_path_buf()));
    }

    Ok(full_path)
}

/// Save file content to workspace
///
/// # Errors
/// Returns an error if path resolution or writing file to disk fails.
pub fn write_file_safe(
    base_dir: &Path,
    rel_path: &Path,
    content: &[u8],
) -> Result<()> {
    let target = resolve_safe_path(base_dir, rel_path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, content)?;
    Ok(())
}

/// Read file content from workspace
///
/// # Errors
/// Returns an error if path resolution or reading file from disk fails.
pub fn read_file_safe(
    base_dir: &Path,
    rel_path: &Path,
) -> Result<Vec<u8>> {
    let target = resolve_safe_path(base_dir, rel_path)?;
    let bytes = fs::read(target)?;
    Ok(bytes)
}

/// Check if file exists in workspace
#[must_use]
pub fn file_exists_safe(
    base_dir: &Path,
    rel_path: &Path,
) -> bool {
    resolve_safe_path(base_dir, rel_path).is_ok_and(|target| target.exists())
}

/// Delete file in workspace
///
/// # Errors
/// Returns an error if path resolution or removing file from disk fails.
pub fn remove_file_safe(
    base_dir: &Path,
    rel_path: &Path,
) -> Result<()> {
    let target = resolve_safe_path(base_dir, rel_path)?;
    if target.is_dir() {
        fs::remove_dir_all(target)?;
    } else if target.exists() {
        fs::remove_file(target)?;
    }
    Ok(())
}

/// Create directory hierarchy in workspace
///
/// # Errors
/// Returns an error if path resolution or creating directory hierarchy fails.
pub fn create_dir_all_safe(
    base_dir: &Path,
    rel_path: &Path,
) -> Result<()> {
    let target = resolve_safe_path(base_dir, rel_path)?;
    fs::create_dir_all(target)?;
    Ok(())
}

/// List files in workspace directory
///
/// # Errors
/// Returns an error if reading directory contents fails.
pub fn list_dir_safe(
    base_dir: &Path,
    rel_path: &Path,
) -> Result<Vec<FileEntry>> {
    let target = resolve_safe_path(base_dir, rel_path)?;
    if !target.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        let modified: Option<DateTime<Utc>> = metadata.modified().ok().map(|t| {
            let sys_time: SystemTime = t;
            DateTime::<Utc>::from(sys_time)
        });

        let entry_path = entry.path();
        let rel = entry_path
            .strip_prefix(base_dir)
            .unwrap_or(&entry_path)
            .to_path_buf();

        entries.push(FileEntry {
            name: file_name,
            rel_path: rel,
            is_dir: file_type.is_dir(),
            is_file: file_type.is_file(),
            is_symlink: file_type.is_symlink(),
            size: metadata.len(),
            modified,
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_safe_path_valid() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        let resolved = resolve_safe_path(base, Path::new("sub/file.txt")).unwrap();
        assert_eq!(resolved, base.join("sub/file.txt"));

        let resolved2 = resolve_safe_path(base, Path::new("/workspace/sub/file.txt")).unwrap();
        assert_eq!(resolved2, base.join("sub/file.txt"));

        let resolved3 = resolve_safe_path(base, Path::new("/sub/file.txt")).unwrap();
        assert_eq!(resolved3, base.join("sub/file.txt"));
    }

    #[test]
    fn test_resolve_safe_path_directory_traversal() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        assert!(resolve_safe_path(base, Path::new("../etc/passwd")).is_err());
        assert!(resolve_safe_path(base, Path::new("foo/../../etc/passwd")).is_err());
    }

    #[test]
    fn test_file_operations() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        let rel = Path::new("docs/notes.md");
        assert!(!file_exists_safe(base, rel));

        write_file_safe(base, rel, b"# Notes").unwrap();
        assert!(file_exists_safe(base, rel));

        let content = read_file_safe(base, rel).unwrap();
        assert_eq!(content, b"# Notes");

        let entries = list_dir_safe(base, Path::new("docs")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "notes.md");
        assert!(entries[0].is_file);

        remove_file_safe(base, rel).unwrap();
        assert!(!file_exists_safe(base, rel));
    }
}
