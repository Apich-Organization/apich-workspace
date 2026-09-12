use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Role of a user within a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    /// Team administrator with management permissions.
    Admin,
    /// Team maintainer with elevated edit access.
    Maintainer,
    /// Regular team member.
    #[default]
    Member,
    /// Read-only team viewer.
    Viewer,
}

impl TeamRole {
    /// Returns the static string representation of the team role.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Admin => "admin",
            | Self::Maintainer => "maintainer",
            | Self::Member => "member",
            | Self::Viewer => "viewer",
        }
    }

    /// Returns true if the role is team administrator.
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl std::str::FromStr for TeamRole {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "admin" => Self::Admin,
            | "maintainer" => Self::Maintainer,
            | "viewer" => Self::Viewer,
            | _ => Self::Member,
        })
    }
}

/// A team within an organization, supporting nested hierarchical structures.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Team {
    /// Unique team identifier (`UUIDv7`).
    pub id: Uuid,
    /// Parent organization ID.
    pub org_id: Uuid,
    /// Optional parent team ID for hierarchical teams.
    pub parent_team_id: Option<Uuid>,
    /// Team display name.
    pub name: String,
    /// URL-friendly slug.
    pub slug: String,
    /// Optional team description.
    pub description: Option<String>,
    /// Team chat URL override.
    pub chat_url: Option<String>,
    /// Team meeting URL override.
    pub meeting_url: Option<String>,
    /// Team storage drive URL override.
    pub drive_url: Option<String>,
    /// Team AI agent URL override.
    pub ai_agent_url: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Data transfer object for creating a new team.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateTeamDto {
    /// Parent organization ID.
    pub org_id: Uuid,
    /// Optional parent team ID.
    pub parent_team_id: Option<Uuid>,
    /// Team display name.
    pub name: String,
    /// Team URL slug.
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// External chat URL.
    pub chat_url: Option<String>,
    /// External meeting URL.
    pub meeting_url: Option<String>,
    /// External storage drive URL.
    pub drive_url: Option<String>,
    /// External AI agent URL.
    pub ai_agent_url: Option<String>,
}

/// Data transfer object for updating a team.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTeamDto {
    /// Updated name.
    pub name: Option<String>,
    /// Updated slug.
    pub slug: Option<String>,
    /// Updated description.
    pub description: Option<String>,
    /// Updated parent team ID (nested option for clearing).
    pub parent_team_id: Option<Option<Uuid>>,
    /// Updated chat URL.
    pub chat_url: Option<String>,
    /// Updated meeting URL.
    pub meeting_url: Option<String>,
    /// Updated drive URL.
    pub drive_url: Option<String>,
    /// Updated AI agent URL.
    pub ai_agent_url: Option<String>,
}

/// Membership record of a user in a team.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMember {
    /// Target team ID.
    pub team_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Assigned role within the team.
    pub role: String,
    /// Timestamp when member joined.
    pub joined_at: DateTime<Utc>,
}

/// Team member record joined with user account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberWithUser {
    /// Target team ID.
    pub team_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Assigned team role.
    pub role: String,
    /// User account username.
    pub username: String,
    /// User email address.
    pub email: String,
    /// User display name.
    pub display_name: String,
    /// User avatar image URL.
    pub avatar_url: Option<String>,
    /// Timestamp when member joined.
    pub joined_at: DateTime<Utc>,
}

/// Recursive team node with children for nested hierarchy rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTreeNode {
    /// Team details for this tree node.
    pub team: Team,
    /// Child teams under this team.
    pub children: Vec<Self>,
    /// Count of direct members.
    pub members_count: usize,
}
