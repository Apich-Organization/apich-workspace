use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Lifecycle status of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    /// Active project accepting modifications.
    #[default]
    Active,
    /// Paused project temporarily suspended.
    Paused,
    /// Archived read-only project.
    Archived,
    /// Soft-deleted project.
    Deleted,
}

impl ProjectStatus {
    /// Returns the static string representation of the project status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Active => "active",
            | Self::Paused => "paused",
            | Self::Archived => "archived",
            | Self::Deleted => "deleted",
        }
    }
}

impl std::str::FromStr for ProjectStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "paused" => Self::Paused,
            | "archived" => Self::Archived,
            | "deleted" => Self::Deleted,
            | _ => Self::Active,
        })
    }
}

/// Execution status of an associated project sandbox container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    /// Container is actively running.
    Running,
    /// Container is stopped due to inactivity or command.
    #[default]
    Stopped,
    /// Container has been destroyed or terminated.
    Terminated,
}

impl SandboxStatus {
    /// Returns the static string representation of the sandbox status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Running => "running",
            | Self::Stopped => "stopped",
            | Self::Terminated => "terminated",
        }
    }
}

impl std::str::FromStr for SandboxStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "running" => Self::Running,
            | "terminated" => Self::Terminated,
            | _ => Self::Stopped,
        })
    }
}

/// A project containing files, version history, documents, and sandboxes.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    /// Unique project identifier (`UUIDv7`).
    pub id: Uuid,
    /// Associated organization ID.
    pub org_id: Uuid,
    /// Optional assigned team ID.
    pub team_id: Option<Uuid>,
    /// User ID of the project owner.
    pub owner_id: Uuid,
    /// Project display name.
    pub name: String,
    /// Project URL-friendly slug.
    pub slug: String,
    /// Optional project description.
    pub description: Option<String>,
    /// Path on host storage for repository files.
    pub storage_path: String,
    /// Current project lifecycle status.
    pub status: String,
    /// Whether the apich-vcs repository has been initialized.
    pub vcs_initialized: bool,
    /// JSON configuration and tool settings.
    pub settings: serde_json::Value,
    /// When true, the VCS timeline flags unsigned snapshots as unverified rather than showing
    /// them the same as signed ones ("vigilant mode" -- bugs.md's own term).
    pub vigilant_mode: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Data transfer object for creating a new project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectDto {
    /// Parent organization ID.
    pub org_id: Uuid,
    /// Optional team ID.
    pub team_id: Option<Uuid>,
    /// Owner user ID.
    pub owner_id: Uuid,
    /// Display name.
    pub name: String,
    /// URL slug.
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Host storage path.
    pub storage_path: String,
    /// Optional initial JSON settings.
    pub settings: Option<serde_json::Value>,
}

/// A container sandbox allocated for executing user code within a project.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSandbox {
    /// Sandbox record unique identifier.
    pub id: Uuid,
    /// Associated project ID.
    pub project_id: Uuid,
    /// Owner user ID.
    pub user_id: Uuid,
    /// Podman container name.
    pub container_name: String,
    /// Sandbox status string.
    pub status: String,
    /// Timestamp when sandbox was last started.
    pub last_started_at: Option<DateTime<Utc>>,
    /// Timestamp when sandbox was stopped.
    pub last_stopped_at: Option<DateTime<Utc>>,
    /// Timestamp of most recent user interaction or command execution.
    pub last_activity_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Access role of a member on a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectRole {
    /// Project owner with full privileges.
    Owner,
    /// Project administrator.
    Admin,
    /// Editor with write and execution access.
    #[default]
    Editor,
    /// Viewer with read-only access.
    Viewer,
}

impl ProjectRole {
    /// Returns the static string representation of the project role.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Owner => "owner",
            | Self::Admin => "admin",
            | Self::Editor => "editor",
            | Self::Viewer => "viewer",
        }
    }
}

impl std::str::FromStr for ProjectRole {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "owner" => Self::Owner,
            | "admin" => Self::Admin,
            | "viewer" => Self::Viewer,
            | _ => Self::Editor,
        })
    }
}

/// Membership record of a user in a project.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectMember {
    /// Target project ID.
    pub project_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Role within the project.
    pub role: String,
    /// Timestamp when member was added.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Project membership record joined with user account information.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectMemberWithUser {
    /// Target project ID.
    pub project_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Assigned role in the project.
    pub role: String,
    /// Member username.
    pub username: String,
    /// Member email address.
    pub email: String,
    /// Member display name.
    pub display_name: String,
    /// Timestamp when member joined.
    pub created_at: DateTime<Utc>,
}

/// Data transfer object for adding a member to a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProjectMemberDto {
    /// User ID to add.
    pub user_id: Uuid,
    /// Optional role to assign (defaults to Editor).
    pub role: Option<String>,
}
