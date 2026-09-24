use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Stored user or reviewer comment on a specific file, slide, or page.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileComment {
    /// Unique comment ID.
    pub id: Uuid,
    /// Project identifier.
    pub project_id: Uuid,
    /// Relative file path within the project.
    pub file_path: String,
    /// Associated user ID if signed in.
    pub user_id: Option<Uuid>,
    /// Display name or reviewer nickname.
    pub author_name: String,
    /// Comment content.
    pub content: String,
    /// Slide index (1-based) or document page number.
    pub slide_or_page: Option<i32>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a new file comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileCommentDto {
    /// Relative file path within the project.
    pub file_path: String,
    /// Author nickname or display name.
    pub author_name: String,
    /// Comment text body.
    pub content: String,
    /// Slide number or PDF page number (optional).
    pub slide_or_page: Option<i32>,
}
