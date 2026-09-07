use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Underlying sandbox error: {0}")]
    Sandbox(#[from] apich_sandbox::SandboxError),

    #[error("SQL error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database container '{0}' failed to start or become healthy within {1:?}")]
    HealthCheckTimeout(String, Duration),

    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    #[error("Backup file not found: {0}")]
    BackupNotFound(PathBuf),

    #[error("Permission configuration error: {0}")]
    PermissionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, DbError>;
