use super::metadata::ExtensibleMetadata;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// A node in the workspace knowledge graph (e.g. wiki, task, calendar, whiteboard, note).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeNode {
    /// Unique node identifier (`UUIDv7`).
    pub id: Uuid,
    /// Associated workspace identifier.
    pub workspace_id: Uuid,
    /// Type of node (e.g. "wiki", "task", "calendar", "whiteboard", "note").
    pub node_type: String,
    /// Node title or label.
    pub title: String,
    /// Content hash for change detection and deduplication.
    pub content_hash: Option<String>,
    /// Extensible JSON metadata.
    pub metadata: serde_json::Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
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

/// A directed edge connecting two knowledge nodes in the workspace graph.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeEdge {
    /// Unique edge identifier (`UUIDv7`).
    pub id: Uuid,
    /// Associated workspace identifier.
    pub workspace_id: Uuid,
    /// Source node identifier.
    pub source_id: Uuid,
    /// Target node identifier.
    pub target_id: Uuid,
    /// Relationship type (e.g. "references", "`depends_on`", "`subtask_of`", "embeds").
    pub relation_type: String,
    /// Edge weight or relevance score.
    pub weight: f32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Data transfer object for creating a knowledge graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeNodeDto {
    /// Optional specific UUID (defaults to auto-generated `UUIDv7`).
    pub id: Option<Uuid>,
    /// Containing workspace ID.
    pub workspace_id: Uuid,
    /// Node category string.
    pub node_type: String,
    /// Node title.
    pub title: String,
    /// Optional content hash.
    pub content_hash: Option<String>,
    /// Optional structured metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Data transfer object for creating a directed edge between two knowledge nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeEdgeDto {
    /// Optional specific UUID (defaults to auto-generated `UUIDv7`).
    pub id: Option<Uuid>,
    /// Containing workspace ID.
    pub workspace_id: Uuid,
    /// Source node ID.
    pub source_id: Uuid,
    /// Target node ID.
    pub target_id: Uuid,
    /// Relationship type name.
    pub relation_type: String,
    /// Optional relationship weight (defaults to 1.0).
    pub weight: Option<f32>,
}
