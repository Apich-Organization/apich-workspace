use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    #[default]
    Member,
    Guest,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            | Self::Admin => "admin",
            | Self::Member => "member",
            | Self::Guest => "guest",
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "admin" => Self::Admin,
            | "guest" => Self::Guest,
            | _ => Self::Member,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_platform_admin: bool,
    pub storage_quota_bytes: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub role: Option<UserRole>,
    pub is_platform_admin: Option<bool>,
    pub storage_quota_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateUserProfileDto {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_serialization() {
        let role = UserRole::Admin;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"admin\"");

        let deserialized: UserRole = serde_json::from_str("\"guest\"").unwrap();
        assert_eq!(deserialized, UserRole::Guest);
    }
}
