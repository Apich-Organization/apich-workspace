use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "doc_type", rename_all = "lowercase")]
pub enum DocType {
    #[default]
    Markdown,
    Typst,
    Latex,
    SqliteTable,
    Slide,
    Code,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub rel_path: String,
    pub title: String,
    pub content: String,
    pub doc_type: DocType,
    pub version: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
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
    /// If the document ID was generated using UUIDv7 (PostgreSQL 18 default),
    /// this extracts the embedded creation timestamp directly from the UUID.
    pub fn timestamp_from_uuid(&self) -> Option<DateTime<Utc>> {
        self.id.get_timestamp().map(|ts| {
            let (secs, nanos) = ts.to_unix();
            DateTime::from_timestamp(secs as i64, nanos).unwrap_or(self.created_at)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentDto {
    pub id: Option<Uuid>,
    pub workspace_id: Uuid,
    pub rel_path: String,
    pub title: String,
    pub content: Option<String>,
    pub doc_type: Option<DocType>,
    pub metadata: Option<serde_json::Value>,
}

/// Search result returned by PostgreSQL Full-Text Search
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentSearchResult {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub rel_path: String,
    pub title: String,
    pub content: String,
    pub doc_type: DocType,
    pub version: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Relevance rank calculated by ts_rank
    pub rank: f32,
}
