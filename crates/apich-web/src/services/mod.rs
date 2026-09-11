pub mod agent_login;
pub mod ai_service;
pub mod cargo_slide_helpers;
pub mod default_templates;
pub mod demo_project;
pub mod document_renderer;
pub mod git_server;
pub mod gpg_keys;
pub mod identity_service;
pub mod knowledge_sync;
pub mod project_manager;
pub mod slide_build;
pub mod sqlite_table;
pub mod sso_service;
pub mod template_library;

pub use agent_login::{AgentLoginRegistry, AgentLoginStatus};
pub use ai_service::{AiAssistantService, AiChatRequest, AiChatResponse};
pub use demo_project::DemoProjectService;
pub use document_renderer::{DocumentRenderer, MarkdownRenderResult, ScriptRunResult, TypstRenderResult};
pub use gpg_keys::gpg_key_fingerprint;
pub use identity_service::IdentityService;
pub use knowledge_sync::KnowledgeSyncService;
pub use project_manager::{FileShareInfo, ProjectManager};
pub use slide_build::{SlideBuildRegistry, SlideBuildStatus};
pub use sqlite_table::SqliteTableService;
pub use sso_service::{JwksResponse, SsoService, TokenResponse};

