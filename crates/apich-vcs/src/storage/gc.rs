//! Mark-and-sweep garbage collector for unreferenced CAS chunks.

use super::cas::ContentAddressableStorage;
use crate::error::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Statistics returned after running garbage collection
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GcStats {
    /// Number of chunk files examined during the sweep.
    pub scanned_chunks: usize,
    /// Number of unreachable chunk files deleted.
    pub pruned_chunks: usize,
    /// Total disk space in bytes reclaimed by the sweep.
    pub reclaimed_bytes: u64,
}

/// Mark-and-sweep garbage collector operating across content-addressable chunks.
pub struct GarbageCollector<'a> {
    cas: &'a ContentAddressableStorage,
}

impl<'a> GarbageCollector<'a> {
    /// Creates a new `GarbageCollector` referencing the provided CAS.
    #[must_use]
    pub const fn new(cas: &'a ContentAddressableStorage) -> Self {
        Self { cas }
    }

    /// Mark-and-sweep physical unreferenced chunks
    ///
    /// # Errors
    /// Returns an error if traversing chunks or deleting unreferenced objects fails.
    pub fn sweep_unreferenced(
        &self,
        referenced_hashes: &HashSet<String>,
    ) -> Result<GcStats> {
        let mut stats = GcStats::default();
        let chunks_dir = self.cas.root_dir().join("chunks");
        if !chunks_dir.exists() {
            return Ok(stats);
        }

        Self::visit_dir(&chunks_dir, referenced_hashes, &mut stats)?;
        Ok(stats)
    }

    fn visit_dir(
        dir: &Path,
        referenced: &HashSet<String>,
        stats: &mut GcStats,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::visit_dir(&path, referenced, stats)?;
                // Remove empty subdirectories
                let _ = fs::remove_dir(&path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("chunk") {
                stats.scanned_chunks = stats.scanned_chunks.saturating_add(1);
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !referenced.contains(stem) {
                        if let Ok(metadata) = path.metadata() {
                            stats.reclaimed_bytes =
                                stats.reclaimed_bytes.saturating_add(metadata.len());
                        }
                        fs::remove_file(&path)?;
                        stats.pruned_chunks = stats.pruned_chunks.saturating_add(1);
                    }
                }
            }
        }
        Ok(())
    }
}
