use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Role of a user within an organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OrgRole {
    /// Organization owner with complete authority.
    Owner,
    /// Organization administrator with management access.
    Admin,
    /// Regular member of the organization.
    #[default]
    Member,
    /// Guest with restricted read-only permissions.
    Guest,
}

impl OrgRole {
    /// Returns the static string representation of the role.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Owner => "owner",
            | Self::Admin => "admin",
            | Self::Member => "member",
            | Self::Guest => "guest",
        }
    }

    /// Returns true if the role has administrative or ownership privileges.
    #[must_use]
    pub const fn is_admin_or_owner(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

impl std::str::FromStr for OrgRole {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "owner" => Self::Owner,
            | "admin" => Self::Admin,
            | "guest" => Self::Guest,
            | _ => Self::Member,
        })
    }
}

/// An organization representing a company, university department, or lab.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    /// Unique organization identifier (`UUIDv7`).
    pub id: Uuid,
    /// URL-friendly organization slug.
    pub slug: String,
    /// Organization display name.
    pub name: String,
    /// Optional organization description.
    pub description: Option<String>,
    /// External chat service URL (e.g. Zulip, Slack).
    pub chat_url: Option<String>,
    /// External video meeting URL (e.g. Jitsi, Google Meet).
    pub meeting_url: Option<String>,
    /// External storage / drive URL (e.g. Nextcloud).
    pub drive_url: Option<String>,
    /// External AI agent service URL.
    pub ai_agent_url: Option<String>,
    /// Whether teams within this org can override hub integration links.
    pub allow_team_override: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Data transfer object for creating an organization.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateOrganizationDto {
    /// URL slug for the organization.
    pub slug: String,
    /// Organization display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// External chat service URL.
    pub chat_url: Option<String>,
    /// External meeting URL.
    pub meeting_url: Option<String>,
    /// External drive URL.
    pub drive_url: Option<String>,
    /// External AI agent URL.
    pub ai_agent_url: Option<String>,
    /// Whether teams can override hub integration links.
    pub allow_team_override: Option<bool>,
}

/// Data transfer object for updating an organization.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateOrganizationDto {
    /// Updated organization display name.
    pub name: Option<String>,
    /// Updated URL slug.
    pub slug: Option<String>,
    /// Updated description.
    pub description: Option<String>,
    /// Updated external chat service URL.
    pub chat_url: Option<String>,
    /// Updated external meeting URL.
    pub meeting_url: Option<String>,
    /// Updated external drive URL.
    pub drive_url: Option<String>,
    /// Updated external AI agent URL.
    pub ai_agent_url: Option<String>,
    /// Updated team override permission flag.
    pub allow_team_override: Option<bool>,
}

/// Resolved effective hub links combining organization defaults and team-specific overrides
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EffectiveHubLinks {
    /// Effective chat service URL.
    pub chat_url: Option<String>,
    /// Effective meeting URL.
    pub meeting_url: Option<String>,
    /// Effective storage drive URL.
    pub drive_url: Option<String>,
    /// Effective AI agent service URL.
    pub ai_agent_url: Option<String>,
    /// Whether values were overridden at team level.
    pub is_team_override: bool,
    /// Whether teams are permitted to override these links.
    pub allow_team_override: bool,
}

/// Membership record of a user in an organization.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrgMember {
    /// Target organization ID.
    pub org_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Assigned role in the organization.
    pub role: String,
    /// Timestamp when the user joined.
    pub joined_at: DateTime<Utc>,
}

/// Organization member record enriched with user profile details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMemberWithUser {
    /// Target organization ID.
    pub org_id: Uuid,
    /// Member user ID.
    pub user_id: Uuid,
    /// Assigned role in the organization.
    pub role: String,
    /// User account username.
    pub username: String,
    /// User email address.
    pub email: String,
    /// User display name.
    pub display_name: String,
    /// User avatar image URL.
    pub avatar_url: Option<String>,
    /// Timestamp when the user joined.
    pub joined_at: DateTime<Utc>,
}
