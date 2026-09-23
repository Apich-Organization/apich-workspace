use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Stored external Git credential for a user (e.g. GitHub, GitLab).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserGitCredential {
    /// Unique credential record identifier.
    pub id: Uuid,
    /// User identifier owning this credential.
    pub user_id: Uuid,
    /// Provider name, e.g. "github" or "gitlab".
    pub provider: String,
    /// Account username on the provider (e.g. "octocat").
    pub account_username: String,
    /// Personal access token for authentication.
    pub access_token: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating or updating a user's Git credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertGitCredentialDto {
    /// Provider identifier (e.g. "github").
    pub provider: String,
    /// Provider account username.
    pub account_username: String,
    /// Personal access token.
    pub access_token: String,
}
