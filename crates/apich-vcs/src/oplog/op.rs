use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpAction {
    Snapshot,
    Revert,
    Merge,
    CreateBranch,
    CreateMilestone,
    Undo,
    Redo,
}

/// Linear, append-only operation entry in the OpLog
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VcsOperation {
    pub op_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: OpAction,
    pub snapshot_before: Option<Uuid>,
    pub snapshot_after: Option<Uuid>,
    pub description: String,
}

impl VcsOperation {
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
