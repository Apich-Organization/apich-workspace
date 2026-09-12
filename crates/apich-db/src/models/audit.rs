use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Record of an auditable action performed within the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    /// Unique identifier of the audit log entry (`UUIDv7`).
    pub id: Uuid,
    /// ID of the user who performed the action, if authenticated.
    pub user_id: Option<Uuid>,
    /// Associated workspace ID, if applicable.
    pub workspace_id: Option<Uuid>,
    /// Action name or verb (e.g. "document.create", "user.login").
    pub action: String,
    /// Structured JSON metadata detailing the action context.
    pub details: serde_json::Value,
    /// Originating IP address of the client request.
    pub ip_address: Option<String>,
    /// Timestamp when the audit event was logged.
    pub created_at: DateTime<Utc>,
}

impl ExtensibleMetadata for AuditLog {
    fn metadata(&self) -> &serde_json::Value {
        &self.details
    }

    fn metadata_mut(&mut self) -> &mut serde_json::Value {
        &mut self.details
    }
}

/// Data transfer object for recording a new audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogDto {
    /// ID of the user performing the action, if known.
    pub user_id: Option<Uuid>,
    /// Associated workspace ID, if applicable.
    pub workspace_id: Option<Uuid>,
    /// Action name or verb.
    pub action: String,
    /// Structured JSON payload with event details.
    pub details: serde_json::Value,
    /// Client IP address.
    pub ip_address: Option<String>,
}
