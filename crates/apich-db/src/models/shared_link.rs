use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Stored managed temporary share link for a project file.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SharedLink {
    /// Unique identifier for the shared link record.
    pub id: Uuid,
    /// Secure random token used in the URL `/s/:token`.
    pub token: String,
    /// Associated project ID.
    pub project_id: Uuid,
    /// Locked relative file path within the project.
    pub file_path: String,
    /// Presentation or viewer target type: "slide" or "pdf".
    pub target_type: String,
    /// Creator user ID if authenticated.
    pub created_by_user_id: Option<Uuid>,
    /// Optional expiration timestamp. None means the link never expires until revoked.
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether the link has been manually revoked.
    pub is_revoked: bool,
    /// Whether viewers can post comments.
    pub allow_comments: bool,
    /// Total number of visits/views.
    pub view_count: i32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl SharedLink {
    /// Check whether this link is currently valid (not revoked and not expired).
    #[must_use]
    pub fn is_active(&self) -> bool {
        if self.is_revoked {
            return false;
        }
        if let Some(exp) = self.expires_at {
            if exp <= Utc::now() {
                return false;
            }
        }
        true
    }
}

/// DTO for creating a new temporary shared link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSharedLinkDto {
    /// Optional custom token. If None, a secure random token will be generated.
    pub token: Option<String>,
    /// Project identifier.
    pub project_id: Uuid,
    /// Relative file path within the project.
    pub file_path: String,
    /// Target presentation type: "slide" or "pdf".
    pub target_type: String,
    /// Creator user ID.
    pub created_by_user_id: Option<Uuid>,
    /// Optional expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether in-browser comments are allowed.
    pub allow_comments: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_shared_link_active_status() {
        let now = Utc::now();
        let mut link = SharedLink {
            id: Uuid::new_v4(),
            token: "test_token_123".to_string(),
            project_id: Uuid::new_v4(),
            file_path: "slides/deck.typ".to_string(),
            target_type: "slide".to_string(),
            created_by_user_id: None,
            expires_at: Some(now + Duration::days(7)),
            is_revoked: false,
            allow_comments: true,
            view_count: 0,
            created_at: now,
            updated_at: now,
        };

        // Active link with future expiration
        assert!(link.is_active());

        // Revoked link
        link.is_revoked = true;
        assert!(!link.is_active());

        // Unrevoked but expired link
        link.is_revoked = false;
        link.expires_at = Some(now - Duration::hours(1));
        assert!(!link.is_active());

        // Permanent link (no expiration)
        link.expires_at = None;
        assert!(link.is_active());
    }

    #[test]
    fn test_shared_link_serialization() {
        let now = Utc::now();
        let link = SharedLink {
            id: Uuid::new_v4(),
            token: "token_abc".to_string(),
            project_id: Uuid::new_v4(),
            file_path: "report.pdf".to_string(),
            target_type: "pdf".to_string(),
            created_by_user_id: None,
            expires_at: None,
            is_revoked: false,
            allow_comments: true,
            view_count: 42,
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&link).expect("serialize");
        let deserialized: SharedLink = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(link.token, deserialized.token);
        assert_eq!(link.file_path, deserialized.file_path);
        assert_eq!(link.view_count, deserialized.view_count);
        assert!(deserialized.is_active());
    }
}

