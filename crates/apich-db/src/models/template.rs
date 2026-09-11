use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateKind {
    #[default]
    Note,
    Latex,
    Typst,
    Slides,
    Kanban,
}

impl TemplateKind {
    pub fn as_str(&self) -> &'static str {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateVisibility {
    #[default]
    Private,
    Shared,
    Public,
}

impl TemplateVisibility {
    pub fn as_str(&self) -> &'static str {
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Template {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateDto {
    pub kind: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TemplateVersion {
    pub id: Uuid,
    pub template_id: Uuid,
    pub version_label: String,
    pub changelog: Option<String>,
    pub content: serde_json::Value,
    pub published_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishTemplateVersionDto {
    pub template_id: Uuid,
    pub version_label: String,
    pub changelog: Option<String>,
    pub content: serde_json::Value,
    pub published_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TemplateShare {
    pub id: Uuid,
    pub template_id: Uuid,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// A gallery-listing row: the template plus a couple of cheap aggregates worth showing without a
/// separate round trip per template (how many versions exist, and its most recent one's label).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TemplateWithLatestVersion {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner_username: String,
    pub version_count: i64,
    pub latest_version_label: Option<String>,
    pub latest_version_id: Option<Uuid>,
}
