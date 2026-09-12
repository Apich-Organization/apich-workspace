//! Operation log records and action types.

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Actions recorded in the linear operation log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpAction {
    /// Workspace snapshot created.
    Snapshot,
    /// Reversion to a prior snapshot.
    Revert,
    /// Reconcile or branch merge operation.
    Merge,
    /// New branch creation.
    CreateBranch,
    /// Named milestone creation.
    CreateMilestone,
    /// Undo operation performed.
    Undo,
    /// Redo operation performed.
    Redo,
}

/// Linear, append-only operation entry in the `OpLog`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VcsOperation {
    /// Unique identifier for this operation.
    pub op_id: Uuid,
    /// Timestamp when this operation was logged.
    pub timestamp: DateTime<Utc>,
    /// Type of action performed.
    pub action: OpAction,
    /// Snapshot ID before this operation took effect.
    pub snapshot_before: Option<Uuid>,
    /// Snapshot ID after this operation took effect.
    pub snapshot_after: Option<Uuid>,
    /// Human-readable description or commit message for the operation.
    pub description: String,
}

impl VcsOperation {
    /// Creates a new `VcsOperation` record.
    pub fn new(
        action: OpAction,
        snapshot_before: Option<Uuid>,
        snapshot_after: Option<Uuid>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            op_id: Uuid::now_v7(),
            timestamp: Utc::now(),
            action,
            snapshot_before,
            snapshot_after,
            description: description.into(),
        }
    }
}
