use super::cas::ContentAddressableStorage;
use crate::error::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Statistics returned after running garbage collection
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GcStats {
    pub scanned_chunks: usize,
    pub pruned_chunks: usize,
    pub reclaimed_bytes: u64,
}

pub struct GarbageCollector<'a> {
    cas: &'a ContentAddressableStorage,
}

impl<'a> GarbageCollector<'a> {
    pub fn new(cas: &'a ContentAddressableStorage) -> Self {
        Self { cas }
    }

    /// Mark-and-sweep physical unreferenced chunks
    pub fn sweep_unreferenced(
        &self,
        referenced_hashes: &HashSet<String>,
    ) -> Result<GcStats> {
        let mut stats = GcStats::default();
        let chunks_dir = self.cas.root_dir().join("chunks");
        if !chunks_dir.exists() {
            return Ok(stats);
        }

        self.visit_dir(&chunks_dir, referenced_hashes, &mut stats)?;
        Ok(stats)
    }

    fn visit_dir(
        &self,
        dir: &Path,
        referenced: &HashSet<String>,
        stats: &mut GcStats,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.visit_dir(&path, referenced, stats)?;
                // Remove empty subdirectories
                let _ = fs::remove_dir(&path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("chunk") {
                stats.scanned_chunks += 1;
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !referenced.contains(stem) {
                        if let Ok(metadata) = path.metadata() {
                            stats.reclaimed_bytes += metadata.len();
                        }
                        fs::remove_file(&path)?;
                        stats.pruned_chunks += 1;
                    }
                }
            }
        }
        Ok(())
    }
}
