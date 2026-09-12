//! Git authentication and remote configuration models.

use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

/// Git authentication mechanism
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitAuth {
    /// Anonymous or public access without credentials.
    Anonymous,
    /// Personal access token authentication.
    Token {
        /// Authentication token string.
        token: String,
    },
    /// SSH private key authentication.
    SshKey {
        /// Filesystem path to the SSH private key.
        private_key_path: PathBuf,
        /// Optional passphrase to decrypt the SSH private key.
        passphrase: Option<String>,
    },
    /// Basic username and password authentication.
    UserPassword {
        /// Account username.
        username: String,
        /// Account password or personal access secret.
        password: String,
    },
}

/// Remote Git repository configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRemoteConfig {
    /// Remote name (e.g., "origin").
    pub name: String,
    /// Remote Git URL.
    pub url: String,
    /// Authentication credentials for accessing the remote.
    pub auth: GitAuth,
}

impl GitRemoteConfig {
    /// Creates a new `GitRemoteConfig` with given name, URL, and credentials.
    pub fn new(
        name: impl Into<String>,
        url: impl Into<String>,
        auth: GitAuth,
    ) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            auth,
        }
    }
}
