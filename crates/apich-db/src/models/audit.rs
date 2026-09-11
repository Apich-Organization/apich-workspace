use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub action: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogDto {
    pub user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub action: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
}
