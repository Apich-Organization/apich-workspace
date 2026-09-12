//! Three-way structural reconciliation engine.

use crate::chunking::FastCdcConfig;
use crate::error::Result;
use crate::model::Conflict;
use crate::model::FileEntry;
use crate::model::VcsTree;
use crate::storage::ContentAddressableStorage;
use std::collections::BTreeSet;

/// Result of a 3-way structural reconcile without pointer weaving
#[derive(Debug, Clone)]
pub struct ReconcileResult {
    /// Resulting unified tree
    pub merged_tree: VcsTree,
    /// List of non-blocking conflicts (if any)
    pub conflicts: Vec<Conflict>,
    /// Summary of files auto-merged or conflicts generated
    pub auto_merged_count: usize,
}

/// Weave-free 3-Way Reconcile Engine
pub struct Reconciler<'a> {
    cas: &'a ContentAddressableStorage,
    cdc_config: FastCdcConfig,
}

impl<'a> Reconciler<'a> {
    /// Creates a new `Reconciler` using the provided CAS and chunking configuration.
    #[must_use]
    pub const fn new(
        cas: &'a ContentAddressableStorage,
        cdc_config: FastCdcConfig,
    ) -> Self {
        Self { cas, cdc_config }
    }

    /// Perform a 3-way merge between base, ours, and theirs
    ///
    /// # Errors
    /// Returns an error if the operation fails.
    pub fn reconcile(
        &self,
        base: Option<&VcsTree>,
        ours: &VcsTree,
        theirs: &VcsTree,
    ) -> Result<ReconcileResult> {
        let mut merged_tree = VcsTree::new();
        let mut conflicts = Vec::new();
        let mut auto_merged_count: usize = 0;

        // Collect all unique file paths across all trees
        let mut all_paths = BTreeSet::new();
        for p in ours.entries.keys() {
            all_paths.insert(p.as_str());
        }
        for p in theirs.entries.keys() {
            all_paths.insert(p.as_str());
        }
        if let Some(b) = base {
            for p in b.entries.keys() {
                all_paths.insert(p.as_str());
            }
        }

        for path in all_paths {
            let ours_entry = ours.get(path);
            let theirs_entry = theirs.get(path);
            let base_entry = base.and_then(|b| b.get(path));

            if self.reconcile_path(
                path,
                ours_entry,
                theirs_entry,
                base_entry,
                &mut merged_tree,
                &mut conflicts,
            )? {
                auto_merged_count = auto_merged_count.saturating_add(1);
            }
        }

        self.cas.put_tree(&merged_tree)?;

        Ok(ReconcileResult {
            merged_tree,
            conflicts,
            auto_merged_count,
        })
    }

    fn reconcile_path(
        &self,
        path: &str,
        ours_entry: Option<&FileEntry>,
        theirs_entry: Option<&FileEntry>,
        base_entry: Option<&FileEntry>,
        merged_tree: &mut VcsTree,
        conflicts: &mut Vec<Conflict>,
    ) -> Result<bool> {
        match (ours_entry, theirs_entry, base_entry) {
            // Case 1: Exists in both ours and theirs
            | (Some(o), Some(t), b_opt) => {
                if o.blake3_hash == t.blake3_hash {
                    // Identical on both sides
                    merged_tree.insert(o.clone());
                    Ok(false)
                } else if let Some(b) = b_opt {
                    if o.blake3_hash == b.blake3_hash {
                        // Unchanged on ours, modified on theirs -> accept theirs
                        merged_tree.insert(t.clone());
                        Ok(true)
                    } else if t.blake3_hash == b.blake3_hash {
                        // Unchanged on theirs, modified on ours -> keep ours
                        merged_tree.insert(o.clone());
                        Ok(false)
                    } else {
                        self.merge_both_modified(path, o, t, b, merged_tree, conflicts)
                    }
                } else {
                    // Added on both sides independently with different content
                    merged_tree.insert(o.clone());
                    conflicts.push(Conflict::new(
                        path,
                        Some(o.blake3_hash.clone()),
                        Some(t.blake3_hash.clone()),
                        None,
                    ));
                    Ok(false)
                }
            },

            // Case 2: Added on ours only
            | (Some(o), None, None) => {
                merged_tree.insert(o.clone());
                Ok(false)
            },

            // Case 3: Added on theirs only
            | (None, Some(t), None) => {
                merged_tree.insert(t.clone());
                Ok(true)
            },

            // Case 4: Removed on theirs, unchanged on ours
            | (Some(o), None, Some(b)) => {
                if o.blake3_hash == b.blake3_hash {
                    // Accept deletion from theirs
                    Ok(true)
                } else {
                    // Modified on ours but deleted on theirs -> conflict, keep ours
                    merged_tree.insert(o.clone());
                    conflicts.push(Conflict::new(
                        path,
                        Some(o.blake3_hash.clone()),
                        None,
                        Some(b.blake3_hash.clone()),
                    ));
                    Ok(false)
                }
            },

            // Case 5: Removed on ours, unchanged on theirs
            | (None, Some(t), Some(b)) => {
                if t.blake3_hash != b.blake3_hash {
                    // Modified on theirs but deleted on ours -> conflict, keep theirs
                    merged_tree.insert(t.clone());
                    conflicts.push(Conflict::new(
                        path,
                        None,
                        Some(t.blake3_hash.clone()),
                        Some(b.blake3_hash.clone()),
                    ));
                }
                Ok(false)
            },

            // Removed on both sides
            | (None, None, Some(_) | None) => Ok(false),
        }
    }

    fn merge_both_modified(
        &self,
        path: &str,
        o: &FileEntry,
        t: &FileEntry,
        b: &FileEntry,
        merged_tree: &mut VcsTree,
        conflicts: &mut Vec<Conflict>,
    ) -> Result<bool> {
        let o_bytes = self.cas.read_file_data(&o.chunks)?;
        let t_bytes = self.cas.read_file_data(&t.chunks)?;
        let b_bytes = self.cas.read_file_data(&b.chunks)?;

        if let (Ok(o_str), Ok(t_str), Ok(b_str)) = (
            std::str::from_utf8(&o_bytes),
            std::str::from_utf8(&t_bytes),
            std::str::from_utf8(&b_bytes),
        ) {
            let (merged_text, has_conflict) = Self::merge_text_3way(o_str, t_str, b_str);

            let (new_chunks, blake3_hash, git_sha1) = self
                .cas
                .put_file_data(merged_text.as_bytes(), self.cdc_config)?;

            let merged_entry = FileEntry {
                path: path.to_string(),
                size: u64::try_from(merged_text.len()).unwrap_or(0),
                is_executable: o.is_executable || t.is_executable,
                chunks: new_chunks,
                blake3_hash,
                git_sha1,
                modified_at: chrono::Utc::now(),
            };
            merged_tree.insert(merged_entry);

            if has_conflict {
                conflicts.push(Conflict::new(
                    path,
                    Some(o.blake3_hash.clone()),
                    Some(t.blake3_hash.clone()),
                    Some(b.blake3_hash.clone()),
                ));
                Ok(false)
            } else {
                Ok(true)
            }
        } else {
            // Binary conflict
            merged_tree.insert(o.clone());
            conflicts.push(Conflict::new(
                path,
                Some(o.blake3_hash.clone()),
                Some(t.blake3_hash.clone()),
                Some(b.blake3_hash.clone()),
            ));
            Ok(false)
        }
    }

    /// Perform a line-by-line 3-way text merge
    fn merge_text_3way(
        ours: &str,
        theirs: &str,
        base: &str,
    ) -> (String, bool) {
        if ours == theirs {
            return (ours.to_string(), false);
        }
        if ours == base {
            return (theirs.to_string(), false);
        }
        if theirs == base {
            return (ours.to_string(), false);
        }

        let ours_lines: Vec<&str> = ours.lines().collect();
        let theirs_lines: Vec<&str> = theirs.lines().collect();
        let base_lines: Vec<&str> = base.lines().collect();

        let mut output = Vec::new();
        let mut has_conflict = false;

        let max_len = ours_lines
            .len()
            .max(theirs_lines.len())
            .max(base_lines.len());
        let mut i = 0;

        while i < max_len {
            let o_line = ours_lines.get(i).copied();
            let t_line = theirs_lines.get(i).copied();
            let b_line = base_lines.get(i).copied();

            match (o_line, t_line, b_line) {
                | (Some(o), Some(t), Some(b)) => {
                    if o == t {
                        output.push(o.to_string());
                    } else if o == b {
                        output.push(t.to_string());
                    } else if t == b {
                        output.push(o.to_string());
                    } else {
                        // Direct line conflict
                        has_conflict = true;
                        output.push(format!("<<<<<<< ours\n{o}\n=======\n{t}\n>>>>>>> theirs"));
                    }
                },
                | (Some(o), Some(t), None) => {
                    if o == t {
                        output.push(o.to_string());
                    } else {
                        has_conflict = true;
                        output.push(format!("<<<<<<< ours\n{o}\n=======\n{t}\n>>>>>>> theirs"));
                    }
                },
                | (Some(o), None, Some(b)) => {
                    if o != b {
                        has_conflict = true;
                        output.push(format!(
                            "<<<<<<< ours\n{o}\n=======\n[deleted in theirs]\n>>>>>>> theirs"
                        ));
                    }
                },
                | (None, Some(t), Some(b)) => {
                    if t != b {
                        has_conflict = true;
                        output.push(format!(
                            "<<<<<<< ours\n[deleted in ours]\n=======\n{t}\n>>>>>>> theirs"
                        ));
                    }
                },
                | (Some(o), None, None) => output.push(o.to_string()),
                | (None, Some(t), None) => output.push(t.to_string()),
                | (None, None, _) => {},
            }
            i = i.saturating_add(1);
        }

        (output.join("\n"), has_conflict)
    }
}
