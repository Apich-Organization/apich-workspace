use crate::config::PostgresConfig;
use crate::error::{DbError, Result};
use apich_sandbox::{ContainerStatus, ExecOptions, ExecResult, PodmanDriver};
use std::fs;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Manages the containerized PostgreSQL instance via Podman
#[derive(Clone)]
pub struct PostgresContainer {
    config: PostgresConfig,
    driver: Arc<PodmanDriver>,
}

impl PostgresContainer {
    pub fn new(config: PostgresConfig) -> Self {
        Self {
            config,
            driver: Arc::new(PodmanDriver::default()),
        }
    }

    pub fn with_driver(mut self, driver: PodmanDriver) -> Self {
        self.driver = Arc::new(driver);
        self
    }

    pub fn config(&self) -> &PostgresConfig {
        &self.config
    }

    pub fn driver(&self) -> &Arc<PodmanDriver> {
        &self.driver
    }

    /// Check if container is running
    pub async fn is_running(&self) -> Result<bool> {
        if let Some(info) = self.driver.inspect(&self.config.container_name).await? {
            Ok(info.is_running)
        } else {
            Ok(false)
        }
    }

    /// Get current container status
    pub async fn status(&self) -> Result<Option<ContainerStatus>> {
        if let Some(info) = self.driver.inspect(&self.config.container_name).await? {
            Ok(Some(info.status))
        } else {
            Ok(None)
        }
    }

    /// Start or create the PostgreSQL container if not already running
    pub async fn ensure_running(&self) -> Result<()> {
        // Ensure host storage directories exist
        if !self.config.host_data_dir.exists() {
            fs::create_dir_all(&self.config.host_data_dir)?;
        }
        if !self.config.host_backup_dir.exists() {
            fs::create_dir_all(&self.config.host_backup_dir)?;
        }

        let container_name = &self.config.container_name;
        match self.driver.inspect(container_name).await? {
            Some(info) => {
                if info.is_running {
                    info!(container = %container_name, "Postgres container is already running");
                    return Ok(());
                }
                info!(container = %container_name, "Starting existing stopped Postgres container");
                if let Err(e) = self.driver.start(container_name).await {
                    warn!(container = %container_name, error = %e, "Failed to start existing container (likely host directory changed); removing and recreating");
                    let _ = self.driver.remove(container_name, true).await;
                    self.spawn_container().await?;
                }
            }
            None => {
                info!(container = %container_name, "Spawning new Postgres container via Podman");
                self.spawn_container().await?;
            }
        }

        Ok(())
    }

    /// Internal method to spawn the postgres container with ports, volumes, and postgres options
    async fn spawn_container(&self) -> Result<String> {
        let mut cmd = Command::new("podman");
        cmd.arg("run").arg("-d");
        cmd.arg("--name").arg(&self.config.container_name);
        cmd.arg("--init");

        // Port mapping
        cmd.arg("-p").arg(format!("{}:5432", self.config.host_port));

        // Environment variables
        cmd.arg("-e")
            .arg(format!("POSTGRES_DB={}", self.config.database));
        cmd.arg("-e")
            .arg(format!("POSTGRES_USER={}", self.config.admin_user));
        cmd.arg("-e")
            .arg(format!("POSTGRES_PASSWORD={}", self.config.admin_password));
        cmd.arg("-e")
            .arg("POSTGRES_INITDB_ARGS=--auth-host=scram-sha-256 --auth-local=scram-sha-256");

        // Volume mounts with SELinux relabeling
        let selinux_suffix = if self.config.selinux_relabel {
            ":Z"
        } else {
            ""
        };
        cmd.arg("-v").arg(format!(
            "{}:/var/lib/postgresql{}",
            self.config.host_data_dir.display(),
            selinux_suffix
        ));
        cmd.arg("-v").arg(format!(
            "{}:/backups{}",
            self.config.host_backup_dir.display(),
            selinux_suffix
        ));

        // Managed labels
        cmd.arg("-l").arg("apich.managed=true");
        cmd.arg("-l").arg("apich.role=postgres");

        // Image
        cmd.arg(&self.config.image);

        // Append PostgreSQL server CLI tuning arguments
        cmd.arg("postgres");
        for arg in self.config.tuning.to_postgres_args() {
            cmd.arg(arg);
        }

        debug!("Spawning postgres container with cmd: {:?}", cmd);

        let output = cmd.output().map_err(DbError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(DbError::Internal(format!(
                "Failed to spawn Postgres container: {}",
                stderr
            )));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(container_id = %container_id, "Postgres container created successfully");
        Ok(container_id)
    }

    /// Wait for PostgreSQL inside container to be fully initialized and accepting connections
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let check_cmd = vec![
            "pg_isready",
            "-h",
            "localhost",
            "-p",
            "5432",
            "-U",
            &self.config.admin_user,
            "-d",
            &self.config.database,
        ];

        while start.elapsed() < timeout {
            let opts = ExecOptions::new(check_cmd.clone()).timeout(Duration::from_secs(2));
            if let Ok(res) = self.driver.exec(&self.config.container_name, &opts).await {
                if res.success() {
                    info!(
                        container = %self.config.container_name,
                        elapsed = ?start.elapsed(),
                        "PostgreSQL is healthy and ready to accept connections"
                    );
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(DbError::HealthCheckTimeout(
            self.config.container_name.clone(),
            timeout,
        ))
    }

    /// Gracefully stop the database container
    pub async fn stop(&self, timeout_secs: u32) -> Result<()> {
        self.driver
            .stop(&self.config.container_name, timeout_secs)
            .await?;
        Ok(())
    }

    /// Restart the database container
    pub async fn restart(&self, timeout_secs: u32) -> Result<()> {
        self.stop(timeout_secs).await?;
        self.driver.start(&self.config.container_name).await?;
        Ok(())
    }

    /// Destroy/remove the database container
    pub async fn destroy(&self) -> Result<()> {
        self.driver
            .remove(&self.config.container_name, true)
            .await?;
        Ok(())
    }

    /// Execute command inside the postgres container
    pub async fn exec<I, S>(&self, cmd: I) -> Result<ExecResult>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let opts = ExecOptions::new(cmd);
        self.exec_with_options(&opts).await
    }

    /// Execute command inside the postgres container with explicit ExecOptions
    pub async fn exec_with_options(&self, opts: &ExecOptions) -> Result<ExecResult> {
        let res = self.driver.exec(&self.config.container_name, opts).await?;
        Ok(res)
    }
}
