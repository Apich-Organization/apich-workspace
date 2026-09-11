use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeNode {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub node_type: String, // "wiki", "task", "calendar", "whiteboard", "note"
    pub title: String,
    pub content_hash: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExtensibleMetadata for KnowledgeNode {
    fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut serde_json::Value {
        &mut self.metadata
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeEdge {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relation_type: String, // "references", "depends_on", "subtask_of", "embeds"
    pub weight: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeNodeDto {
    pub id: Option<Uuid>,
    pub workspace_id: Uuid,
    pub node_type: String,
    pub title: String,
    pub content_hash: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeEdgeDto {
    pub id: Option<Uuid>,
    pub workspace_id: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relation_type: String,
    pub weight: Option<f32>,
}
