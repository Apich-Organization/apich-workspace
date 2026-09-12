use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Document content type format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "doc_type", rename_all = "lowercase")]
pub enum DocType {
    /// Markdown format (.md).
    #[default]
    Markdown,
    /// Typst technical document format (.typ).
    Typst,
    /// LaTeX document format (.tex).
    Latex,
    /// SQLite relational table snapshot.
    SqliteTable,
    /// Presentation slide format.
    Slide,
    /// Source code file.
    Code,
    /// Other unclassified file format.
    Other,
}

/// Persistent workspace document record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    /// Unique document identifier (`UUIDv7`).
    pub id: Uuid,
    /// Containing workspace identifier.
    pub workspace_id: Uuid,
    /// Relative path within the workspace root.
    pub rel_path: String,
    /// Document display title.
    pub title: String,
    /// Raw textual content of the document.
    pub content: String,
    /// Type/format of the document.
    pub doc_type: DocType,
    /// Incrementing version counter.
    pub version: i32,
    /// Extensible JSON metadata.
    pub metadata: serde_json::Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
}

impl ExtensibleMetadata for Document {
    fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut serde_json::Value {
        &mut self.metadata
    }
}

impl Document {
    /// If the document ID was generated using `UUIDv7` (PostgreSQL 18 default),
    /// this extracts the embedded creation timestamp directly from the UUID.
    #[must_use]
    pub fn timestamp_from_uuid(&self) -> Option<DateTime<Utc>> {
        self.id.get_timestamp().map(|ts| {
            let (secs, nanos) = ts.to_unix();
            DateTime::from_timestamp(secs as i64, nanos).unwrap_or(self.created_at)
        })
    }
}

/// Data transfer object for creating a new workspace document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentDto {
    /// Optional specific UUID (defaults to auto-generated `UUIDv7`).
    pub id: Option<Uuid>,
    /// Workspace to contain the document.
    pub workspace_id: Uuid,
    /// Relative filesystem path.
    pub rel_path: String,
    /// Display title.
    pub title: String,
    /// Initial text content.
    pub content: Option<String>,
    /// Document format.
    pub doc_type: Option<DocType>,
    /// Optional custom metadata object.
    pub metadata: Option<serde_json::Value>,
}

/// Search result returned by PostgreSQL Full-Text Search
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentSearchResult {
    /// Document unique identifier.
    pub id: Uuid,
    /// Containing workspace ID.
    pub workspace_id: Uuid,
    /// Relative file path.
    pub rel_path: String,
    /// Document title.
    pub title: String,
    /// Content snippet or body.
    pub content: String,
    /// Document format.
    pub doc_type: DocType,
    /// Document version number.
    pub version: i32,
    /// Document metadata.
    pub metadata: serde_json::Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Relevance rank calculated by `ts_rank`
    pub rank: f32,
}
