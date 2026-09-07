use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Performance and resource tuning configuration for PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostgresTuningConfig {
    pub max_connections: u32,
    pub shared_buffers: String,
    pub effective_cache_size: String,
    pub maintenance_work_mem: String,
    pub work_mem: String,
    pub wal_buffers: String,
    pub random_page_cost: f32,
    pub track_io_timing: bool,
    pub checkpoint_completion_target: f32,
}

impl Default for PostgresTuningConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            shared_buffers: "128MB".to_string(),
            effective_cache_size: "512MB".to_string(),
            maintenance_work_mem: "64MB".to_string(),
            work_mem: "4MB".to_string(),
            wal_buffers: "16MB".to_string(),
            random_page_cost: 1.1,
            track_io_timing: true,
            checkpoint_completion_target: 0.9,
        }
    }
}

impl PostgresTuningConfig {
    /// Convert configuration into PostgreSQL command line arguments (`-c param=value`)
    pub fn to_postgres_args(&self) -> Vec<String> {
        let args = vec![
            "-c".to_string(),
            format!("max_connections={}", self.max_connections),
            "-c".to_string(),
            format!("shared_buffers={}", self.shared_buffers),
            "-c".to_string(),
            format!("effective_cache_size={}", self.effective_cache_size),
            "-c".to_string(),
            format!("maintenance_work_mem={}", self.maintenance_work_mem),
            "-c".to_string(),
            format!("work_mem={}", self.work_mem),
            "-c".to_string(),
            format!("wal_buffers={}", self.wal_buffers),
            "-c".to_string(),
            format!("random_page_cost={}", self.random_page_cost),
            "-c".to_string(),
            format!("track_io_timing={}", if self.track_io_timing { "on" } else { "off" }),
            "-c".to_string(),
            format!("checkpoint_completion_target={}", self.checkpoint_completion_target),
        ];
        args
    }
}

/// Security & Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostgresSecurityConfig {
    /// Password encryption algorithm (default: scram-sha-256)
    pub password_encryption: String,
    /// Enforce least privilege separation between admin and application users
    pub enforce_least_privilege: bool,
    /// SSL mode: disable, require, verify-ca, verify-full
    pub ssl_mode: String,
}

impl Default for PostgresSecurityConfig {
    fn default() -> Self {
        Self {
            password_encryption: "scram-sha-256".to_string(),
            enforce_least_privilege: true,
            ssl_mode: "prefer".to_string(),
        }
    }
}

/// Main configuration for the containerized PostgreSQL service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostgresConfig {
    /// Container name
    pub container_name: String,
    /// Container image (defaults to "docker.io/library/postgres:18-alpine")
    pub image: String,
    /// Host port to expose (e.g. 5432)
    pub host_port: u16,
    /// Target database name
    pub database: String,
    /// Superuser username (default "postgres")
    pub admin_user: String,
    /// Superuser password
    pub admin_password: String,
    /// Application user with restricted permissions
    pub app_user: String,
    /// Application user password
    pub app_password: String,
    /// Host directory for database files (mounted to /var/lib/postgresql/data)
    pub host_data_dir: PathBuf,
    /// Host directory for backup files (mounted to /backups)
    pub host_backup_dir: PathBuf,
    /// SELinux relabeling flag (:Z)
    pub selinux_relabel: bool,
    /// Performance tuning options
    pub tuning: PostgresTuningConfig,
    /// Security options
    pub security: PostgresSecurityConfig,
}

impl PostgresConfig {
    pub fn builder(
        host_data_dir: impl AsRef<Path>,
        host_backup_dir: impl AsRef<Path>,
    ) -> PostgresConfigBuilder {
        PostgresConfigBuilder::new(host_data_dir, host_backup_dir)
    }

    /// Admin connection string (used for administrative tasks, user creation, migrations)
    pub fn admin_connection_url(&self, host: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.admin_user,
            self.admin_password,
            host,
            self.host_port,
            self.database,
            self.security.ssl_mode
        )
    }

    /// Application connection string (used for normal workspace queries with restricted privileges)
    pub fn app_connection_url(&self, host: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.app_user,
            self.app_password,
            host,
            self.host_port,
            self.database,
            self.security.ssl_mode
        )
    }
}

pub struct PostgresConfigBuilder {
    container_name: String,
    image: String,
    host_port: u16,
    database: String,
    admin_user: String,
    admin_password: Option<String>,
    app_user: String,
    app_password: Option<String>,
    host_data_dir: PathBuf,
    host_backup_dir: PathBuf,
    selinux_relabel: bool,
    tuning: PostgresTuningConfig,
    security: PostgresSecurityConfig,
}

impl PostgresConfigBuilder {
    pub fn new(host_data_dir: impl AsRef<Path>, host_backup_dir: impl AsRef<Path>) -> Self {
        Self {
            container_name: "apich-postgres".to_string(),
            image: "docker.io/library/postgres:18-alpine".to_string(),
            host_port: 5432,
            database: "apich_workspace".to_string(),
            admin_user: "postgres".to_string(),
            admin_password: None,
            app_user: "apich_app".to_string(),
            app_password: None,
            host_data_dir: host_data_dir.as_ref().to_path_buf(),
            host_backup_dir: host_backup_dir.as_ref().to_path_buf(),
            selinux_relabel: true,
            tuning: PostgresTuningConfig::default(),
            security: PostgresSecurityConfig::default(),
        }
    }

    pub fn container_name(mut self, name: impl Into<String>) -> Self {
        self.container_name = name.into();
        self
    }

    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = image.into();
        self
    }

    pub fn host_port(mut self, port: u16) -> Self {
        self.host_port = port;
        self
    }

    pub fn database(mut self, db: impl Into<String>) -> Self {
        self.database = db.into();
        self
    }

    pub fn admin_user(mut self, user: impl Into<String>) -> Self {
        self.admin_user = user.into();
        self
    }

    pub fn admin_password(mut self, password: impl Into<String>) -> Self {
        self.admin_password = Some(password.into());
        self
    }

    pub fn app_user(mut self, user: impl Into<String>) -> Self {
        self.app_user = user.into();
        self
    }

    pub fn app_password(mut self, password: impl Into<String>) -> Self {
        self.app_password = Some(password.into());
        self
    }

    pub fn selinux_relabel(mut self, enable: bool) -> Self {
        self.selinux_relabel = enable;
        self
    }

    pub fn tuning(mut self, tuning: PostgresTuningConfig) -> Self {
        self.tuning = tuning;
        self
    }

    pub fn security(mut self, security: PostgresSecurityConfig) -> Self {
        self.security = security;
        self
    }

    pub fn build(self) -> PostgresConfig {
        let admin_password = self
            .admin_password
            .unwrap_or_else(|| "apich_admin_secret_pwd".to_string());
        let app_password = self
            .app_password
            .unwrap_or_else(|| "apich_app_secure_pwd".to_string());

        PostgresConfig {
            container_name: self.container_name,
            image: self.image,
            host_port: self.host_port,
            database: self.database,
            admin_user: self.admin_user,
            admin_password,
            app_user: self.app_user,
            app_password,
            host_data_dir: self.host_data_dir,
            host_backup_dir: self.host_backup_dir,
            selinux_relabel: self.selinux_relabel,
            tuning: self.tuning,
            security: self.security,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_postgres_config_builder() {
        let data_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let config = PostgresConfig::builder(data_dir.path(), backup_dir.path())
            .container_name("test-postgres")
            .host_port(5439)
            .database("test_db")
            .admin_user("admin")
            .admin_password("admin_pass")
            .app_user("app_user")
            .app_password("app_pass")
            .selinux_relabel(true)
            .tuning(PostgresTuningConfig {
                max_connections: 50,
                shared_buffers: "64MB".to_string(),
                ..Default::default()
            })
            .build();

        assert_eq!(config.image, "docker.io/library/postgres:18-alpine");
        assert_eq!(config.container_name, "test-postgres");
        assert_eq!(config.host_port, 5439);
        assert_eq!(config.database, "test_db");
        assert_eq!(config.admin_user, "admin");
        assert_eq!(config.admin_password, "admin_pass");
        assert_eq!(config.app_user, "app_user");
        assert_eq!(config.app_password, "app_pass");
        assert!(config.selinux_relabel);

        let admin_url = config.admin_connection_url("localhost");
        assert_eq!(
            admin_url,
            "postgres://admin:admin_pass@localhost:5439/test_db?sslmode=prefer"
        );

        let app_url = config.app_connection_url("localhost");
        assert_eq!(
            app_url,
            "postgres://app_user:app_pass@localhost:5439/test_db?sslmode=prefer"
        );

        let args = config.tuning.to_postgres_args();
        assert!(args.contains(&"max_connections=50".to_string()));
        assert!(args.contains(&"shared_buffers=64MB".to_string()));
        assert!(args.contains(&"track_io_timing=on".to_string()));
    }
}
