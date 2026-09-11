use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

/// Resource constraints for the container
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ResourceLimits {
    /// Memory limit, e.g. "2g", "512m"
    pub memory: Option<String>,
    /// CPU limit in number of cores, e.g. 2.0
    pub cpus: Option<f64>,
    /// Maximum number of PIDs
    pub pids_limit: Option<u64>,
}

/// Extra volume/bind mount configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MountSpec {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
    pub selinux_label: Option<String>, // e.g. "Z" or "z"
}

/// Configuration for creating and running a user sandbox container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxConfig {
    /// Unique user ID associated with this sandbox
    pub user_id: String,
    /// Container name (defaults to "apich-sandbox-{user_id}")
    pub container_name: String,
    /// OCI image to run
    pub image: String,
    /// Root directory on the host holding the user's workspace files
    pub host_workspace_dir: PathBuf,
    /// Target directory inside the container where workspace is mounted (default: /workspace)
    pub container_workspace_dir: PathBuf,
    /// Resource limits (CPU, memory, PIDs)
    pub resources: ResourceLimits,
    /// Environment variables to inject
    pub env: HashMap<String, String>,
    /// Additional bind mounts
    pub extra_mounts: Vec<MountSpec>,
    /// Labels attached to the container
    pub labels: HashMap<String, String>,
    /// Network mode (e.g. "none", "bridge", "slirp4netns", "pasta", "host")
    pub network: Option<String>,
    /// Whether to apply SELinux ':Z' flag on workspace mount
    pub selinux_relabel: bool,
    /// Run with --userns=keep-id in rootless mode to preserve host UID
    pub keep_id: bool,
    /// Run with --init to provide a proper init process for signal handling
    pub init_process: bool,
}

impl SandboxConfig {
    pub fn builder(
        user_id: impl Into<String>,
        host_workspace_dir: impl AsRef<Path>,
    ) -> SandboxConfigBuilder {
        SandboxConfigBuilder::new(user_id, host_workspace_dir)
    }

    /// Default container name generation
    pub fn default_container_name(user_id: &str) -> String {
        format!("apich-sandbox-{}", user_id)
    }
}

pub struct SandboxConfigBuilder {
    user_id: String,
    container_name: Option<String>,
    image: String,
    host_workspace_dir: PathBuf,
    container_workspace_dir: PathBuf,
    resources: ResourceLimits,
    env: HashMap<String, String>,
    extra_mounts: Vec<MountSpec>,
    labels: HashMap<String, String>,
    network: Option<String>,
    selinux_relabel: bool,
    keep_id: bool,
    init_process: bool,
}

impl SandboxConfigBuilder {
    pub fn new(
        user_id: impl Into<String>,
        host_workspace_dir: impl AsRef<Path>,
    ) -> Self {
        let uid = user_id.into();
        let default_name = SandboxConfig::default_container_name(&uid);
        let mut labels = HashMap::new();
        labels.insert("apich.managed".to_string(), "true".to_string());
        labels.insert("apich.user_id".to_string(), uid.clone());

        Self {
            user_id: uid,
            container_name: Some(default_name),
            image: "localhost/apich-sandbox:latest".to_string(),
            host_workspace_dir: host_workspace_dir.as_ref().to_path_buf(),
            container_workspace_dir: PathBuf::from("/workspace"),
            resources: ResourceLimits::default(),
            env: HashMap::new(),
            extra_mounts: Vec::new(),
            labels,
            network: None,
            selinux_relabel: true,
            keep_id: false, // by default off unless specified or tested
            init_process: true,
        }
    }

    pub fn container_name(
        mut self,
        name: impl Into<String>,
    ) -> Self {
        self.container_name = Some(name.into());
        self
    }

    pub fn image(
        mut self,
        image: impl Into<String>,
    ) -> Self {
        self.image = image.into();
        self
    }

    pub fn container_workspace_dir(
        mut self,
        dir: impl AsRef<Path>,
    ) -> Self {
        self.container_workspace_dir = dir.as_ref().to_path_buf();
        self
    }

    pub fn memory_limit(
        mut self,
        memory: impl Into<String>,
    ) -> Self {
        self.resources.memory = Some(memory.into());
        self
    }

    pub fn cpu_limit(
        mut self,
        cpus: f64,
    ) -> Self {
        self.resources.cpus = Some(cpus);
        self
    }

    pub fn pids_limit(
        mut self,
        pids: u64,
    ) -> Self {
        self.resources.pids_limit = Some(pids);
        self
    }

    pub fn env(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn envs(
        mut self,
        envs: HashMap<String, String>,
    ) -> Self {
        self.env.extend(envs);
        self
    }

    pub fn add_mount(
        mut self,
        mount: MountSpec,
    ) -> Self {
        self.extra_mounts.push(mount);
        self
    }

    pub fn add_label(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn network(
        mut self,
        network: impl Into<String>,
    ) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn selinux_relabel(
        mut self,
        enable: bool,
    ) -> Self {
        self.selinux_relabel = enable;
        self
    }

    pub fn keep_id(
        mut self,
        enable: bool,
    ) -> Self {
        self.keep_id = enable;
        self
    }

    pub fn init_process(
        mut self,
        enable: bool,
    ) -> Self {
        self.init_process = enable;
        self
    }

    pub fn build(self) -> SandboxConfig {
        let container_name = self
            .container_name
            .unwrap_or_else(|| SandboxConfig::default_container_name(&self.user_id));

        SandboxConfig {
            user_id: self.user_id,
            container_name,
            image: self.image,
            host_workspace_dir: self.host_workspace_dir,
            container_workspace_dir: self.container_workspace_dir,
            resources: self.resources,
            env: self.env,
            extra_mounts: self.extra_mounts,
            labels: self.labels,
            network: self.network,
            selinux_relabel: self.selinux_relabel,
            keep_id: self.keep_id,
            init_process: self.init_process,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::builder("alice", "/tmp/alice_workspace")
            .image("custom-image:1.0")
            .memory_limit("2g")
            .cpu_limit(2.5)
            .pids_limit(512)
            .env("ENV_VAR", "TEST")
            .network("pasta")
            .selinux_relabel(true)
            .build();

        assert_eq!(config.user_id, "alice");
        assert_eq!(config.container_name, "apich-sandbox-alice");
        assert_eq!(config.image, "custom-image:1.0");
        assert_eq!(
            config.host_workspace_dir,
            PathBuf::from("/tmp/alice_workspace")
        );
        assert_eq!(config.container_workspace_dir, PathBuf::from("/workspace"));
        assert_eq!(config.resources.memory.as_deref(), Some("2g"));
        assert_eq!(config.resources.cpus, Some(2.5));
        assert_eq!(config.resources.pids_limit, Some(512));
        assert_eq!(config.env.get("ENV_VAR").unwrap(), "TEST");
        assert_eq!(config.labels.get("apich.managed").unwrap(), "true");
        assert_eq!(config.labels.get("apich.user_id").unwrap(), "alice");
        assert_eq!(config.network.as_deref(), Some("pasta"));
        assert!(config.selinux_relabel);
    }
}
