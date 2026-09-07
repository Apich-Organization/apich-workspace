use thiserror::Error;

pub type Result<T> = std::result::Result<T, VcsError>;

#[derive(Error, Debug)]
pub enum VcsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Chunk not found in CAS: {0}")]
    ChunkNotFound(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("Milestone not found: {0}")]
    MilestoneNotFound(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Unresolved merge conflict: {0}")]
    Conflict(String),

    #[error("Invalid path or directory traversal attempt: {0}")]
    InvalidPath(String),

    #[error("VCS repository not initialized at: {0}")]
    NotInitialized(String),

    #[error("Internal VCS error: {0}")]
    Internal(String),
}
