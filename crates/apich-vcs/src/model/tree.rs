//! VCS Tree model representing files and directories at a point in time.

use super::chunk::ChunkRef;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

/// Metadata and chunk references for a tracked file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path from workspace project root
    pub path: String,
    /// Total file size in bytes
    pub size: u64,
    /// Executable file permission
    pub is_executable: bool,
    /// Content-defined chunk references in order
    pub chunks: Vec<ChunkRef>,
    /// Full-file BLAKE3 hash
    pub blake3_hash: String,
    /// Full-file SHA-1 hash for native Git Tree/Blob mapping
    pub git_sha1: String,
    /// Last modified time
    pub modified_at: DateTime<Utc>,
}

/// Directory tree representing the full project state at a point in time
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VcsTree {
    /// File entries keyed by relative normalized path
    pub entries: BTreeMap<String, FileEntry>,
    /// Canonical BLAKE3 hash of the tree
    pub tree_hash: String,
}

impl VcsTree {
    /// Creates a new empty `VcsTree`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            tree_hash: String::new(),
        }
    }

    /// Add or update a file entry and recalculate the canonical tree hash
    pub fn insert(
        &mut self,
        entry: FileEntry,
    ) {
        self.entries.insert(entry.path.clone(), entry);
        self.recompute_hash();
    }

    /// Remove a file by relative path
    pub fn remove(
        &mut self,
        path: &str,
    ) -> Option<FileEntry> {
        let removed = self.entries.remove(path);
        if removed.is_some() {
            self.recompute_hash();
        }
        removed
    }

    /// Get file entry by relative path
    #[must_use]
    pub fn get(
        &self,
        path: &str,
    ) -> Option<&FileEntry> {
        self.entries.get(path)
    }

    /// Recalculate canonical BLAKE3 tree hash
    pub fn recompute_hash(&mut self) {
        let mut hasher = blake3::Hasher::new();
        for (path, entry) in &self.entries {
            hasher.update(path.as_bytes());
            hasher.update(&entry.size.to_le_bytes());
            hasher.update(entry.blake3_hash.as_bytes());
            hasher.update(entry.git_sha1.as_bytes());
        }
        self.tree_hash = hasher.finalize().to_hex().to_string();
    }

    /// Compute structural diff between this tree and another base tree
    #[must_use]
    pub fn diff<'a>(
        &'a self,
        other: &'a Self,
    ) -> TreeDiff<'a> {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut removed = Vec::new();

        for (path, entry) in &self.entries {
            match other.entries.get(path) {
                | None => added.push(entry),
                | Some(other_entry) => {
                    if entry.blake3_hash != other_entry.blake3_hash {
                        modified.push((entry, other_entry));
                    }
                },
            }
        }

        for (path, other_entry) in &other.entries {
            if !self.entries.contains_key(path) {
                removed.push(other_entry);
            }
        }

        TreeDiff {
            added,
            modified,
            removed,
        }
    }
}

/// Structural differences between two trees
#[derive(Debug, Clone)]
pub struct TreeDiff<'a> {
    /// Files present in the new tree but absent in the base tree.
    pub added: Vec<&'a FileEntry>,
    /// Files present in both trees with altered content hashes (new, old).
    pub modified: Vec<(&'a FileEntry, &'a FileEntry)>,
    /// Files present in the base tree but missing in the new tree.
    pub removed: Vec<&'a FileEntry>,
}

impl TreeDiff<'_> {
    /// Returns true if there are no added, modified, or removed files.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}
