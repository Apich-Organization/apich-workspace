//! Error types and Result alias for APICH VCS.

use thiserror::Error;

/// Specialized Result type for VCS operations.
pub type Result<T> = std::result::Result<T, VcsError>;

/// Error variants representing failures during VCS operations.
#[derive(Error, Debug)]
pub enum VcsError {
    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Git operation or repository error.
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// Content-addressable chunk not found.
    #[error("Chunk not found in CAS: {0}")]
    ChunkNotFound(String),

    /// Snapshot referenced by hash or UUID was not found.
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    /// Milestone tag not found.
    #[error("Milestone not found: {0}")]
    MilestoneNotFound(String),

    /// Named branch not found.
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Unresolved merge or reconcile conflict.
    #[error("Unresolved merge conflict: {0}")]
    Conflict(String),

    /// Invalid or unauthorized path traversal.
    #[error("Invalid path or directory traversal attempt: {0}")]
    InvalidPath(String),

    /// VCS repository has not been initialized.
    #[error("VCS repository not initialized at: {0}")]
    NotInitialized(String),

    /// Internal or unexpected VCS error.
    #[error("Internal VCS error: {0}")]
    Internal(String),
}
