use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Type of template content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateKind {
    /// Note document template.
    #[default]
    Note,
    /// LaTeX document template.
    Latex,
    /// Typst technical document template.
    Typst,
    /// Slide presentation template.
    Slides,
    /// Kanban board template.
    Kanban,
}

impl TemplateKind {
    /// Returns the static string representation of the template kind.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Note => "note",
            | Self::Latex => "latex",
            | Self::Typst => "typst",
            | Self::Slides => "slides",
            | Self::Kanban => "kanban",
        }
    }
}

impl std::str::FromStr for TemplateKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "latex" => Self::Latex,
            | "typst" => Self::Typst,
            | "slides" => Self::Slides,
            | "kanban" => Self::Kanban,
            | _ => Self::Note,
        })
    }
}

/// Sharing and visibility scope of a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateVisibility {
    /// Only visible to template owner.
    #[default]
    Private,
    /// Shared with specific organizations or teams.
    Shared,
    /// Publicly discoverable across the instance.
    Public,
}

impl TemplateVisibility {
    /// Returns the static string representation of the template visibility.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Private => "private",
            | Self::Shared => "shared",
            | Self::Public => "public",
        }
    }
}

impl std::str::FromStr for TemplateVisibility {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            | "shared" => Self::Shared,
            | "public" => Self::Public,
            | _ => Self::Private,
        })
    }
}

/// A publishable document or workspace template record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Template {
    /// Unique template identifier (`UUIDv7`).
    pub id: Uuid,
    /// Template kind ("note", "latex", "typst", "slides", "kanban").
    pub kind: String,
    /// Template display name.
    pub name: String,
    /// Template URL slug.
    pub slug: String,
    /// Optional template description.
    pub description: Option<String>,
    /// User ID of the template owner.
    pub owner_user_id: Uuid,
    /// Visibility scope ("private", "shared", "public").
    pub visibility: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Data transfer object for creating a template record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateDto {
    /// Kind of template.
    pub kind: String,
    /// Display name.
    pub name: String,
    /// URL slug.
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Owner user ID.
    pub owner_user_id: Uuid,
    /// Visibility scope.
    pub visibility: String,
}

/// An immutable published version of a template's content.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TemplateVersion {
    /// Unique version record identifier (`UUIDv7`).
    pub id: Uuid,
    /// Associated template ID.
    pub template_id: Uuid,
    /// Version label (e.g. "v1.0.0").
    pub version_label: String,
    /// Optional release notes or changelog.
    pub changelog: Option<String>,
    /// Structured JSON content payload of the template.
    pub content: serde_json::Value,
    /// User ID of the publisher.
    pub published_by: Uuid,
    /// Publication timestamp.
    pub created_at: DateTime<Utc>,
}

/// Data transfer object for publishing a new template version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishTemplateVersionDto {
    /// Target template ID.
    pub template_id: Uuid,
    /// Version label.
    pub version_label: String,
    /// Optional release changelog notes.
    pub changelog: Option<String>,
    /// Structured JSON content payload.
    pub content: serde_json::Value,
    /// Publisher user ID.
    pub published_by: Uuid,
}

/// Access grant sharing a template with an organization or team.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TemplateShare {
    /// Share record unique identifier (`UUIDv7`).
    pub id: Uuid,
    /// Associated template ID.
    pub template_id: Uuid,
    /// Shared organization ID, if org-wide.
    pub org_id: Option<Uuid>,
    /// Shared team ID, if scoped to team.
    pub team_id: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// A gallery-listing row: the template plus a couple of cheap aggregates worth showing without a
/// separate round trip per template (how many versions exist, and its most recent one's label).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TemplateWithLatestVersion {
    /// Template unique identifier.
    pub id: Uuid,
    /// Template kind.
    pub kind: String,
    /// Template display name.
    pub name: String,
    /// Template slug.
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Owner user ID.
    pub owner_user_id: Uuid,
    /// Visibility scope string.
    pub visibility: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Username of the template owner.
    pub owner_username: String,
    /// Total count of published versions.
    pub version_count: i64,
    /// Version label of the most recent publication.
    pub latest_version_label: Option<String>,
    /// Record ID of the most recent version.
    pub latest_version_id: Option<Uuid>,
}
