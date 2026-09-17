//! # Services Module
//!
//! Business logic, background services, and application workflows.

pub(crate) mod agent_login;
pub(crate) mod ai_service;
pub(crate) mod cargo_slide_helpers;
pub(crate) mod command_security;
/// Default template seeding service.
pub mod default_templates;
pub(crate) mod demo_project;
pub(crate) mod document_renderer;
pub(crate) mod git_server;
pub(crate) mod gpg_keys;
pub(crate) mod identity_service;
pub(crate) mod knowledge_sync;
pub(crate) mod project_manager;
pub(crate) mod slide_build;
pub(crate) mod sqlite_table;
pub(crate) mod sso_service;
pub(crate) mod template_library;

pub(crate) use agent_login::AgentLoginStatus;
pub(crate) use command_security::CommandSecurityGuard;
pub(crate) use command_security::SecurityViolation;
pub(crate) use gpg_keys::gpg_key_fingerprint;
pub(crate) use identity_service::IdentityService;
pub(crate) use knowledge_sync::KnowledgeSyncService;
pub(crate) use project_manager::FileShareInfo;
pub(crate) use project_manager::ProjectManager;
pub(crate) use slide_build::SlideBuildStatus;
pub(crate) use sqlite_table::SqliteTableService;
pub(crate) use sso_service::SsoService;
pub(crate) use sso_service::TokenResponse;
