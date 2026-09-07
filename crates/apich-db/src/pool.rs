use crate::config::PostgresConfig;
use crate::error::Result;
use crate::migrations::{run_migrations, MigrationManager, MigrationResult};
use crate::permissions::PermissionManager;
use crate::repo::Repository;
use crate::session::DbSession;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// High-level Database handle with connection pools, migrations, and repository access
#[derive(Clone)]
pub struct Database {
    admin_pool: PgPool,
    app_pool: Option<PgPool>,
}

impl Database {
    /// Connect to PostgreSQL using administrator credentials
    pub async fn connect_admin(config: &PostgresConfig, host: &str) -> Result<Self> {
        let admin_url = config.admin_connection_url(host);
        info!(host = %host, port = %config.host_port, "Connecting to PostgreSQL with admin role");

        let admin_pool = PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&admin_url)
            .await?;

        Ok(Self {
            admin_pool,
            app_pool: None,
        })
    }

    /// Connect or initialize application role connection pool
    pub async fn init_app_pool(&mut self, config: &PostgresConfig, host: &str) -> Result<()> {
        let app_url = config.app_connection_url(host);
        info!(host = %host, app_user = %config.app_user, "Connecting to PostgreSQL with app role");

        let app_pool = PgPoolOptions::new()
            .max_connections(config.tuning.max_connections.min(50))
            .acquire_timeout(Duration::from_secs(5))
            .connect(&app_url)
            .await?;

        self.app_pool = Some(app_pool);
        Ok(())
    }

    /// Admin connection pool (for DDL, user administration, backup preparation)
    pub fn admin_pool(&self) -> &PgPool {
        &self.admin_pool
    }

    /// Application connection pool (for least-privilege operations)
    pub fn app_pool(&self) -> Option<&PgPool> {
        self.app_pool.as_ref()
    }

    /// Preferred pool for business queries (app_pool if available, else admin_pool)
    pub fn pool(&self) -> &PgPool {
        self.app_pool.as_ref().unwrap_or(&self.admin_pool)
    }

    /// Verify database connectivity
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(self.pool()).await?;
        Ok(())
    }

    /// Run default database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        run_migrations(&self.admin_pool).await
    }

    /// Run migrations with an extensible `MigrationManager`
    pub async fn migrate_with(&self, manager: &MigrationManager) -> Result<Vec<MigrationResult>> {
        manager.migrate(&self.admin_pool).await
    }

    /// Configure least-privilege permissions for the application user
    pub async fn setup_permissions(&self, config: &PostgresConfig) -> Result<()> {
        PermissionManager::setup_least_privilege(&self.admin_pool, config).await
    }

    /// Get typed repository helper
    pub fn repository(&self) -> Repository<'_> {
        Repository::new(self.pool())
    }

    /// Get typed repository helper explicitly bound to admin pool
    pub fn admin_repository(&self) -> Repository<'_> {
        Repository::new(&self.admin_pool)
    }

    /// Begin a user-scoped session enforcing Row-Level Security (RLS)
    pub async fn begin_session(&self, user_id: Option<Uuid>) -> Result<DbSession<'_>> {
        DbSession::begin(self.pool(), user_id).await
    }

    /// Begin a system session with RLS bypassed
    pub async fn begin_system_session(&self) -> Result<DbSession<'_>> {
        DbSession::begin_system(self.pool()).await
    }
}
