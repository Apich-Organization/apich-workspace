use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Fido2Credential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_id: String,
    pub public_key: Vec<u8>,
    pub counter: i64,
    pub device_name: String,
    pub aaguid: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationMode {
    Open,
    #[default]
    InviteOnly,
    AdminOnly,
}

impl RegistrationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InviteOnly => "invite_only",
            Self::AdminOnly => "admin_only",
        }
    }
}

impl std::str::FromStr for RegistrationMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "open" => Self::Open,
            "admin_only" => Self::AdminOnly,
            _ => Self::InviteOnly,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemSettings {
    pub id: i32,
    pub registration_mode: String,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from_email: Option<String>,
    pub smtp_from_name: Option<String>,
    pub smtp_use_tls: bool,
    pub smtp_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            id: 1,
            registration_mode: "invite_only".to_string(),
            smtp_host: None,
            smtp_port: None,
            smtp_username: None,
            smtp_password: None,
            smtp_from_email: None,
            smtp_from_name: None,
            smtp_use_tls: false,
            smtp_enabled: false,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSystemSettingsDto {
    pub registration_mode: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from_email: Option<String>,
    pub smtp_from_name: Option<String>,
    pub smtp_use_tls: Option<bool>,
    pub smtp_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invitation {
    pub id: Uuid,
    pub token: String,
    pub email: String,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub role: String,
    pub inviter_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvitationDto {
    pub email: String,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub role: Option<String>,
    pub inviter_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
}
