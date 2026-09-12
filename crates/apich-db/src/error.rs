use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during database container, migration, or query operations.
#[derive(Error, Debug)]
pub enum DbError {
    /// Underlying container or sandbox execution error.
    #[error("Underlying sandbox error: {0}")]
    Sandbox(#[from] apich_sandbox::SandboxError),

    /// SQL query or connection error from sqlx.
    #[error("SQL error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Filesystem I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Database container failed to become healthy before the timeout.
    #[error("Database container '{0}' failed to start or become healthy within {1:?}")]
    HealthCheckTimeout(String, Duration),

    /// Database backup process failed.
    #[error("Backup failed: {0}")]
    BackupFailed(String),

    /// Database restore process failed.
    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    /// Specified backup file could not be found.
    #[error("Backup file not found: {0}")]
    BackupNotFound(PathBuf),

    /// Role, user grant, or RLS permission error.
    #[error("Permission configuration error: {0}")]
    PermissionError(String),

    /// Configuration validation or setup error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Requested database record was not found.
    #[error("Entity not found: {0}")]
    NotFound(String),

    /// Unique constraint violation or conflicting state.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Unclassified internal database error.
    #[error("Database error: {0}")]
    Internal(String),
}

/// Convenience result type for database operations returning [`DbError`].
pub type Result<T> = std::result::Result<T, DbError>;
