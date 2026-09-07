//! # APICH DB
//!
//! PostgreSQL 18 container management, volume persistence, automated backups,
//! least-privilege security configurations, extensible domain models,
//! and modern database features (UUIDv7, GIN Full-Text Search, JSONB path-ops,
//! Row-Level Security) for the APICH Technical & Academic Workspace.
//!
//! ## Core Features:
//! - **PostgreSQL 18 Containerization**: Spawns and manages Postgres 18 via rootless Podman without host installation.
//! - **Native PostgreSQL 18 Features**: Native `uuidv7()`, generated tsvector FTS with GIN indexes, `JSON_TABLE` relational projection, and `MERGE` upserts.
//! - **Row-Level Security (RLS)**: Fine-grained multi-tenant workspace isolation at the database engine level.
//! - **Extensible Architecture**: Open migration registry (`MigrationManager`) allowing other modules to register schemas dynamically, and typed extension metadata (`ExtensibleMetadata`) for domain models.
//! - **Data Persistence & Backups**: Host directory mounting with SELinux `:Z`, automated `pg_dump` / `pg_restore` disaster recovery.
//! - **Security & Least Privilege**: Role separation between superuser (`postgres`) and application user (`apich_app`) with `NOBYPASSRLS`.

pub mod backup;
pub mod config;
pub mod container;
pub mod error;
pub mod migrations;
pub mod models;
pub mod permissions;
pub mod pool;
pub mod repo;
pub mod session;

pub use backup::{BackupFormat, BackupInfo, BackupManager, BackupOptions};
pub use config::{
    PostgresConfig, PostgresConfigBuilder, PostgresSecurityConfig, PostgresTuningConfig,
};
pub use container::PostgresContainer;
pub use error::{DbError, Result};
pub use migrations::{run_migrations, Migration, MigrationManager, MigrationResult};
pub use models::*;
pub use permissions::{IdentityPermissionResolver, PermissionManager};
pub use pool::Database;
pub use repo::Repository;
pub use session::DbSession;
