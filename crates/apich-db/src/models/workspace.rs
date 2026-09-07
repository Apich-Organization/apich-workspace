use super::metadata::ExtensibleMetadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "workspace_visibility", rename_all = "lowercase")]
pub enum WorkspaceVisibility {
    #[default]
    Private,
    Internal,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "member_role", rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Maintainer,
    Contributor,
    #[default]
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workspace {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: WorkspaceVisibility,
    pub container_name: Option<String>,
    pub storage_path: Option<String>,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExtensibleMetadata for Workspace {
    fn metadata(&self) -> &serde_json::Value {
        &self.settings
    }

    fn metadata_mut(&mut self) -> &mut serde_json::Value {
        &mut self.settings
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkspaceMember {
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceDto {
    pub id: Option<Uuid>,
    pub owner_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Option<WorkspaceVisibility>,
    pub settings: Option<serde_json::Value>,
}
