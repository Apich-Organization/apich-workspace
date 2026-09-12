use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Primary platform role for a user account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    /// Administrator with platform-wide management access.
    Admin,
    /// Standard user account with project and workspace privileges.
    #[default]
    Member,
    /// Read-only guest account.
    Guest,
}

impl UserRole {
    /// Returns the static string representation of the user role.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
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

/// A registered user account in the system.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    /// Unique user identifier (`UUIDv7`).
    pub id: Uuid,
    /// Unique username.
    pub username: String,
    /// Verified email address.
    pub email: String,
    /// Argon2 password hash.
    pub password_hash: String,
    /// Friendly display name.
    pub display_name: String,
    /// Optional avatar image URL.
    pub avatar_url: Option<String>,
    /// Primary platform role.
    pub role: UserRole,
    /// Whether the user has global platform administrator permissions.
    pub is_platform_admin: bool,
    /// Storage quota limit in bytes.
    pub storage_quota_bytes: i64,
    /// Whether the user account is active.
    pub is_active: bool,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last account update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Data transfer object for provisioning a new user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto {
    /// Username for the account.
    pub username: String,
    /// Account email address.
    pub email: String,
    /// Pre-hashed password.
    pub password_hash: String,
    /// User display name.
    pub display_name: String,
    /// Optional assigned platform role.
    pub role: Option<UserRole>,
    /// Optional platform administrator flag.
    pub is_platform_admin: Option<bool>,
    /// Optional custom storage quota in bytes.
    pub storage_quota_bytes: Option<i64>,
}

/// Data transfer object for updating a user's personal profile information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateUserProfileDto {
    /// Updated display name.
    pub display_name: Option<String>,
    /// Updated email address.
    pub email: Option<String>,
    /// Updated avatar image URL.
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
