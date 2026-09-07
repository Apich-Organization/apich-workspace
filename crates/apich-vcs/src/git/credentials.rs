use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Git authentication mechanism
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitAuth {
    Anonymous,
    Token {
        token: String,
    },
    SshKey {
        private_key_path: PathBuf,
        passphrase: Option<String>,
    },
    UserPassword {
        username: String,
        password: String,
    },
}

/// Remote Git repository configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRemoteConfig {
    pub name: String,
    pub url: String,
    pub auth: GitAuth,
}

impl GitRemoteConfig {
    pub fn new(name: impl Into<String>, url: impl Into<String>, auth: GitAuth) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            auth,
        }
    }
}
