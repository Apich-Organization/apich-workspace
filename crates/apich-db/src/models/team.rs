use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    Admin,
    Maintainer,
    #[default]
    Member,
    Viewer,
}

impl TeamRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            | Self::Admin => "admin",
            | Self::Maintainer => "maintainer",
            | Self::Member => "member",
            | Self::Viewer => "viewer",
        }
    }

    pub fn is_admin(&self) -> bool {
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Team {
    pub id: Uuid,
    pub org_id: Uuid,
    pub parent_team_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateTeamDto {
    pub org_id: Uuid,
    pub parent_team_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTeamDto {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_team_id: Option<Option<Uuid>>,
    pub chat_url: Option<String>,
    pub meeting_url: Option<String>,
    pub drive_url: Option<String>,
    pub ai_agent_url: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberWithUser {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub joined_at: DateTime<Utc>,
}

/// Recursive team node with children for nested hierarchy rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTreeNode {
    pub team: Team,
    pub children: Vec<TeamTreeNode>,
    pub members_count: usize,
}
