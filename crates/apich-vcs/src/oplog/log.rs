use super::op::{OpAction, VcsOperation};
use crate::error::Result;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Append-only operation ledger facilitating 100% reversible history and undo-tree
pub struct OpLog {
    log_file: PathBuf,
}

impl OpLog {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let path = project_root.as_ref().join(".apich").join("oplog.jsonl");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { log_file: path })
    }

    /// Append a new operation to the log
    pub fn append(&self, op: &VcsOperation) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)?;

        let line = serde_json::to_string(op)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// List all operations in chronological order
    pub fn list(&self) -> Result<Vec<VcsOperation>> {
        if !self.log_file.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.log_file)?;
        let reader = BufReader::new(file);
        let mut ops = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                let op: VcsOperation = serde_json::from_str(&line)?;
                ops.push(op);
            }
        }

        Ok(ops)
    }

    /// Undo the last mutating operation: find the target snapshot before the operation,
    /// and record an Undo entry. Returns target snapshot ID to revert to.
    pub fn undo(&self, current_snapshot: Option<Uuid>) -> Result<Option<Uuid>> {
        let ops = self.list()?;
        let mut net_undo_depth: usize = 0;

        for op in ops.iter().rev() {
            match op.action {
                OpAction::Redo => {
                    net_undo_depth = net_undo_depth.saturating_sub(1);
                }
                OpAction::Undo => {
                    net_undo_depth += 1;
                }
                _ => {
                    if net_undo_depth > 0 {
                        net_undo_depth -= 1;
                    } else {
                        // Found the mutating operation to undo!
                        if let Some(target_id) = op.snapshot_before {
                            let undo_op = VcsOperation::new(
                                OpAction::Undo,
                                current_snapshot,
                                Some(target_id),
                                format!("Undo operation: {}", op.description),
                            );
                            self.append(&undo_op)?;
                            return Ok(Some(target_id));
                        } else {
                            // If snapshot_before was None (initial snapshot), there is no prior snapshot
                            return Ok(None);
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Redo the last undone operation: find the matching Undo operation to reverse,
    /// and record a Redo entry. Returns target snapshot ID to restore.
    pub fn redo(&self, current_snapshot: Option<Uuid>) -> Result<Option<Uuid>> {
        let ops = self.list()?;
        let mut net_redo_depth: usize = 0;

        for op in ops.iter().rev() {
            match op.action {
                OpAction::Redo => {
                    net_redo_depth += 1;
                }
                OpAction::Undo => {
                    if net_redo_depth > 0 {
                        net_redo_depth -= 1;
                    } else if let Some(target_id) = op.snapshot_before {
                        let redo_op = VcsOperation::new(
                            OpAction::Redo,
                            current_snapshot,
                            Some(target_id),
                            format!("Redo operation: restored state to {}", target_id),
                        );
                        self.append(&redo_op)?;
                        return Ok(Some(target_id));
                    }
                }
                _ => {
                    // New mutation encountered; cannot redo past a fresh mutation
                    break;
                }
            }
        }

        Ok(None)
    }
}
