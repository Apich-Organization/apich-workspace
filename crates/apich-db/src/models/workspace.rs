use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Visibility level governing access to a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "workspace_visibility", rename_all = "lowercase")]
pub enum WorkspaceVisibility {
    /// Restricted to explicit workspace members.
    #[default]
    Private,
    /// Visible to all authenticated users in the organization/instance.
    Internal,
    /// Publicly readable across the instance.
    Public,
}

/// Membership role of a user inside a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "member_role", rename_all = "lowercase")]
pub enum MemberRole {
    /// Workspace owner with administrative control.
    Owner,
    /// Maintainer with write and configuration rights.
    Maintainer,
    /// Contributor with write and edit access.
    Contributor,
    /// Viewer with read-only permissions.
    #[default]
    Viewer,
}

/// A workspace grouping documents, knowledge nodes, and collaborative state.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workspace {
    /// Unique workspace identifier (`UUIDv7`).
    pub id: Uuid,
    /// User ID of the owner.
    pub owner_id: Uuid,
    /// Workspace URL slug.
    pub slug: String,
    /// Workspace display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Visibility level.
    pub visibility: WorkspaceVisibility,
    /// Associated container name if containerized.
    pub container_name: Option<String>,
    /// Storage path on host.
    pub storage_path: Option<String>,
    /// Extensible JSON configuration settings.
    pub settings: serde_json::Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
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

/// Membership record connecting a user to a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkspaceMember {
    /// Associated workspace ID.
    pub workspace_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Assigned workspace role.
    pub role: MemberRole,
    /// Timestamp when member joined.
    pub joined_at: DateTime<Utc>,
}

/// Data transfer object for creating a new workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceDto {
    /// Optional specific UUID (defaults to auto-generated `UUIDv7`).
    pub id: Option<Uuid>,
    /// Owner user ID.
    pub owner_id: Uuid,
    /// URL slug.
    pub slug: String,
    /// Display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Optional visibility setting.
    pub visibility: Option<WorkspaceVisibility>,
    /// Optional structured settings.
    pub settings: Option<serde_json::Value>,
}
