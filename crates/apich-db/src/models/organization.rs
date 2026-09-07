use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OrgRole {
    Owner,
    Admin,
    #[default]
    Member,
    Guest,
}

impl OrgRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Guest => "guest",
        }
    }

    pub fn is_admin_or_owner(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

impl std::str::FromStr for OrgRole {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "owner" => Self::Owner,
            "admin" => Self::Admin,
            "guest" => Self::Guest,
            _ => Self::Member,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationDto {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateOrganizationDto {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrgMember {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMemberWithUser {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub joined_at: DateTime<Utc>,
}
