//! First-class non-blocking conflict models.

use serde::Deserialize;
use serde::Serialize;

/// First-class non-blocking conflict metadata.
///
/// Conflicts do not halt or lock the repository. They are recorded directly
/// as state entries allowing normal editing to proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    /// Relative path of the conflicted file
    pub path: String,
    /// BLAKE3 hash of our version
    pub ours_hash: Option<String>,
    /// BLAKE3 hash of incoming version
    pub theirs_hash: Option<String>,
    /// BLAKE3 hash of common ancestor version
    pub base_hash: Option<String>,
    /// Whether the conflict has been resolved by the user
    pub is_resolved: bool,
}

impl Conflict {
    /// Creates a new `Conflict` instance for a path with the given version hashes.
    pub fn new(
        path: impl Into<String>,
        ours_hash: Option<String>,
        theirs_hash: Option<String>,
        base_hash: Option<String>,
    ) -> Self {
        Self {
            path: path.into(),
            ours_hash,
            theirs_hash,
            base_hash,
            is_resolved: false,
        }
    }

    /// Render standard inline conflict markers into a single text buffer
    #[must_use]
    pub fn format_text_conflict(
        ours: &str,
        theirs: &str,
        ours_label: &str,
        theirs_label: &str,
    ) -> String {
        format!("<<<<<<< {ours_label}\n{ours}\n=======\n{theirs}\n>>>>>>> {theirs_label}\n")
    }
}
