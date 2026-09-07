use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Podman binary not found in PATH or not executable")]
    PodmanNotFound,

    #[error("Container '{0}' not found")]
    ContainerNotFound(String),

    #[error("Container '{0}' already exists")]
    ContainerAlreadyExists(String),

    #[error("Command execution failed in container '{container}': exit code {exit_code}, stderr: {stderr}")]
    CommandFailed {
        container: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("Execution timed out after {0:?}")]
    ExecutionTimeout(Duration),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Path traversal detected or invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SandboxError>;
