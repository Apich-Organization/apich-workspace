//! # APICH DB
//!
//! PostgreSQL 18 container management, volume persistence, automated backups,
//! least-privilege security configurations, extensible domain models,
//! and modern database features (`UUIDv7`, GIN Full-Text Search, JSONB path-ops,
//! Row-Level Security) for the APICH Technical & Academic Workspace.
//!
//! ## Core Features:
//! - **PostgreSQL 18 Containerization**: Spawns and manages Postgres 18 via rootless Podman without host installation.
//! - **Native PostgreSQL 18 Features**: Native `uuidv7()`, generated tsvector FTS with GIN indexes, `JSON_TABLE` relational projection, and `MERGE` upserts.
//! - **Row-Level Security (RLS)**: Fine-grained multi-tenant workspace isolation at the database engine level.
//! - **Extensible Architecture**: Open migration registry (`MigrationManager`) allowing other modules to register schemas dynamically, and typed extension metadata (`ExtensibleMetadata`) for domain models.
//! - **Data Persistence & Backups**: Host directory mounting with `SELinux` `:Z`, automated `pg_dump` / `pg_restore` disaster recovery.
//! - **Security & Least Privilege**: Role separation between superuser (`postgres`) and application user (`apich_app`) with `NOBYPASSRLS`.

/// Database backup and restore operations via `pg_dump` and `pg_restore`.
pub mod backup;
/// Database configuration structures and builders.
pub mod config;
/// Container lifecycle management for PostgreSQL 18.
pub mod container;
/// Database error types and result aliases.
pub mod error;
/// Database schema migrations and migration registry.
pub mod migrations;
/// Domain models and data transfer objects.
pub mod models;
/// Row-level security and permission management.
pub mod permissions;
/// Database connection pool and high-level client handle.
pub mod pool;
/// Data access layer and repository operations.
pub mod repo;
/// Scoped session handles with transaction and RLS context.
pub mod session;

pub use backup::BackupFormat;
pub use backup::BackupInfo;
pub use backup::BackupManager;
pub use backup::BackupOptions;
pub use config::PostgresConfig;
pub use config::PostgresConfigBuilder;
pub use config::PostgresSecurityConfig;
pub use config::PostgresTuningConfig;
pub use container::PostgresContainer;
pub use error::DbError;
pub use error::Result;
pub use migrations::run_migrations;
pub use migrations::Migration;
pub use migrations::MigrationManager;
pub use migrations::MigrationResult;
pub use models::*;
pub use permissions::IdentityPermissionResolver;
pub use permissions::PermissionManager;
pub use pool::Database;
pub use repo::Repository;
pub use session::DbSession;
