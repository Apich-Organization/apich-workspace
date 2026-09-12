//! Error types and Result alias for the sandbox subsystem.

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Errors produced by sandbox container and process management operations.
#[derive(Error, Debug)]
pub enum SandboxError {
    /// Podman CLI executable could not be located in system PATH.
    #[error("Podman binary not found in PATH or not executable")]
    PodmanNotFound,

    /// Container was not found by name or ID.
    #[error("Container '{0}' not found")]
    ContainerNotFound(String),

    /// A container with the specified name already exists.
    #[error("Container '{0}' already exists")]
    ContainerAlreadyExists(String),

    /// Command executed in the container exited with non-zero exit code.
    #[error("Command execution failed in container '{container}': exit code {exit_code}, stderr: {stderr}")]
    CommandFailed {
        /// Name of the container where command executed.
        container: String,
        /// Command exit code.
        exit_code: i32,
        /// Captured standard error stream.
        stderr: String,
    },

    /// Command execution exceeded the allotted timeout duration.
    #[error("Execution timed out after {0:?}")]
    ExecutionTimeout(Duration),

    /// Container or mount configuration was invalid.
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Path traversal detected or path falls outside allowed workspace boundary.
    #[error("Path traversal detected or invalid path: {0}")]
    InvalidPath(PathBuf),

    /// Low-level I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Asynchronous communication channel closed unexpectedly.
    #[error("Channel error: {0}")]
    ChannelError(String),

    /// Unexpected internal sandbox failure.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Specialized Result type for sandbox operations.
pub type Result<T> = std::result::Result<T, SandboxError>;
