use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Named branch pointer to a specific snapshot in the project timeline
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    /// Branch name (e.g. "main", "draft", "peer-review")
    pub name: String,
    /// Target snapshot ID
    pub head_snapshot_id: Uuid,
    /// When the branch was created or updated
    pub updated_at: DateTime<Utc>,
    /// Optional tracked upstream Git branch (e.g. "origin/main")
    pub upstream: Option<String>,
}

impl Branch {
    pub fn new(name: impl Into<String>, head_snapshot_id: Uuid) -> Self {
        Self {
            name: name.into(),
            head_snapshot_id,
            updated_at: Utc::now(),
            upstream: None,
        }
    }
}
