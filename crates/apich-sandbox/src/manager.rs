use crate::config::{ResourceLimits, SandboxConfig};
use crate::container::UserContainer;
use crate::driver::{ContainerStatus, PodmanDriver};
use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// High-level manager orchestrating user sandbox containers and user storage
#[derive(Clone)]
pub struct SandboxManager {
    base_storage_dir: PathBuf,
    default_image: String,
    driver: Arc<PodmanDriver>,
    selinux_relabel: bool,
    default_resources: ResourceLimits,
    container_workspace_dir: PathBuf,
    keep_id: bool,
}

impl SandboxManager {
    /// Create a new SandboxManager with a base storage root and default image
    pub fn new(base_storage_dir: impl AsRef<Path>, default_image: impl Into<String>) -> Self {
        Self {
            base_storage_dir: base_storage_dir.as_ref().to_path_buf(),
            default_image: default_image.into(),
            driver: Arc::new(PodmanDriver::default()),
            selinux_relabel: true,
            default_resources: ResourceLimits::default(),
            container_workspace_dir: PathBuf::from("/workspace"),
            keep_id: false,
        }
    }

    pub fn with_driver(mut self, driver: PodmanDriver) -> Self {
        self.driver = Arc::new(driver);
        self
    }

    pub fn with_selinux(mut self, enable: bool) -> Self {
        self.selinux_relabel = enable;
        self
    }

    pub fn with_resources(mut self, limits: ResourceLimits) -> Self {
        self.default_resources = limits;
        self
    }

    pub fn with_container_workspace_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.container_workspace_dir = dir.as_ref().to_path_buf();
        self
    }

    pub fn with_keep_id(mut self, keep_id: bool) -> Self {
        self.keep_id = keep_id;
        self
    }

    pub fn base_storage_dir(&self) -> &Path {
        &self.base_storage_dir
    }

    pub fn default_image(&self) -> &str {
        &self.default_image
    }

    pub fn driver(&self) -> &Arc<PodmanDriver> {
        &self.driver
    }

    /// Stop an arbitrary named container
    pub async fn stop_container(&self, container_name: &str, timeout_secs: u32) -> Result<()> {
        self.driver.stop(container_name, timeout_secs).await
    }

    /// Resolve host directory for a specific user
    pub fn user_storage_path(&self, user_id: &str) -> PathBuf {
        self.base_storage_dir.join(user_id)
    }

    /// Generate default config for a user sandbox
    pub fn make_user_config(&self, user_id: &str) -> SandboxConfig {
        let host_dir = self.user_storage_path(user_id);
        let mut builder = SandboxConfig::builder(user_id, host_dir)
            .image(&self.default_image)
            .container_workspace_dir(&self.container_workspace_dir)
            .selinux_relabel(self.selinux_relabel)
            .keep_id(self.keep_id);

        if let Some(ref mem) = self.default_resources.memory {
            builder = builder.memory_limit(mem.clone());
        }
        if let Some(cpus) = self.default_resources.cpus {
            builder = builder.cpu_limit(cpus);
        }
        if let Some(pids) = self.default_resources.pids_limit {
            builder = builder.pids_limit(pids);
        }

        builder.build()
    }

    /// Ensure the user's container is created and running.
    /// If container does not exist, it will create storage directory, mount it, and start container.
    /// If container is stopped or exited, it will start it.
    /// If container is running, it returns the handle directly.
    pub async fn ensure_running(&self, user_id: &str) -> Result<UserContainer> {
        let config = self.make_user_config(user_id);
        self.ensure_running_with_config(config).await
    }

    /// Ensure container is running with custom configuration
    pub async fn ensure_running_with_config(&self, config: SandboxConfig) -> Result<UserContainer> {
        // Ensure host storage directory exists
        if !config.host_workspace_dir.exists() {
            fs::create_dir_all(&config.host_workspace_dir)?;
        }

        let container_name = &config.container_name;
        match self.driver.inspect(container_name).await? {
            Some(info) => {
                // Detect a container stuck on a stale image: it was created once via `podman
                // run` and pinned to whatever image ID that tag resolved to *then*, so rebuilding
                // e.g. docker/Containerfile.sandbox (adding a missing Python package, say) and
                // retagging `apich-sandbox:latest` never touches containers already created from
                // the old build -- they'd otherwise keep running the stale image forever, since
                // every other branch below only looks at run/stop state, never image identity.
                // `image_id` resolving to `None` (image not built/pulled yet) is left alone here;
                // that failure surfaces naturally from `run_detached` instead.
                let current_image_id = self.driver.image_id(&config.image).await?;
                let is_stale = match &current_image_id {
                    Some(current) => !info.image_id.is_empty() && &info.image_id != current,
                    None => false,
                };
                if is_stale {
                    info!(
                        container_name = %container_name,
                        old_image = %info.image_id,
                        new_image = current_image_id.as_deref().unwrap_or(""),
                        "Recreating container: image was rebuilt since this container was created"
                    );
                    let _ = self.driver.remove(container_name, true).await;
                    self.driver.run_detached(&config).await?;
                    return Ok(UserContainer::new(config, self.driver.clone()));
                }

                match info.status {
                    ContainerStatus::Running => {
                        info!(container_name = %container_name, "Container already running");
                    }
                    ContainerStatus::Paused => {
                        info!(container_name = %container_name, "Unpausing container");
                        self.driver.unpause(container_name).await?;
                    }
                    ContainerStatus::Stopped
                    | ContainerStatus::Exited
                    | ContainerStatus::Created => {
                        info!(container_name = %container_name, "Starting stopped container");
                        self.driver.start(container_name).await?;
                    }
                    ContainerStatus::Unknown => {
                        // Recreate if in unknown state
                        let _ = self.driver.remove(container_name, true).await;
                        self.driver.run_detached(&config).await?;
                    }
                }
            }
            None => {
                info!(container_name = %container_name, "Spawning new container");
                self.driver.run_detached(&config).await?;
            }
        }

        Ok(UserContainer::new(config, self.driver.clone()))
    }

    /// Retrieve user container handle if it exists
    pub async fn get_user_sandbox(&self, user_id: &str) -> Result<Option<UserContainer>> {
        let config = self.make_user_config(user_id);
        if let Some(_info) = self.driver.inspect(&config.container_name).await? {
            Ok(Some(UserContainer::new(config, self.driver.clone())))
        } else {
            Ok(None)
        }
    }

    /// Gracefully stop user sandbox
    pub async fn stop_user_sandbox(&self, user_id: &str, timeout_secs: u32) -> Result<()> {
        let container_name = SandboxConfig::default_container_name(user_id);
        self.driver.stop(&container_name, timeout_secs).await
    }

    /// Destroy / remove user sandbox container
    pub async fn destroy_user_sandbox(&self, user_id: &str) -> Result<()> {
        let container_name = SandboxConfig::default_container_name(user_id);
        self.driver.remove(&container_name, true).await
    }

    /// List all managed *sandbox* containers -- i.e. per-user/per-project containers
    /// (`SandboxConfig::builder()` stamps every one of these with `apich.user_id`), NOT every
    /// container this app manages. `apich.managed=true` alone is too broad: the platform's own
    /// Postgres container (`apich-db`'s `PostgresContainer::spawn_container`) carries that same
    /// label (plus `apich.role=postgres`, no `apich.user_id`) since it's also podman-managed by
    /// this app -- filtering on `apich.managed=true` here previously made this function, and the
    /// orphan reaper built on it, treat that infra container as an abandoned sandbox and delete
    /// it (confirmed live: it took down the running database mid-session). `apich.user_id`
    /// existence is what actually distinguishes a sandbox from infra.
    pub async fn list_managed_sandboxes(&self) -> Result<Vec<String>> {
        self.driver.list_containers(Some("apich.user_id")).await
    }
}
