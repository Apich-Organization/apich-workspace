use crate::config::SandboxConfig;
use crate::driver::{ContainerInspectInfo, ContainerStatus, PodmanDriver};
use crate::error::{Result, SandboxError};
use crate::exec::{ExecOptions, ExecResult, ExecStream, InteractiveExec};
use crate::fs::{self, FileEntry};
use crate::tools::{
    agent::AgentToolchain, git::GitToolchain, latex::LatexToolchain, python::PythonToolchain,
    r::RToolchain, rust::RustToolchain, typst::TypstToolchain,
};
use std::path::Path;
use std::sync::Arc;

/// High-level handle to a user's isolated sandbox container
#[derive(Clone)]
pub struct UserContainer {
    config: SandboxConfig,
    driver: Arc<PodmanDriver>,
}

impl UserContainer {
    pub fn new(config: SandboxConfig, driver: Arc<PodmanDriver>) -> Self {
        Self { config, driver }
    }

    pub fn user_id(&self) -> &str {
        &self.config.user_id
    }

    pub fn container_name(&self) -> &str {
        &self.config.container_name
    }

    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    pub fn host_workspace_dir(&self) -> &Path {
        &self.config.host_workspace_dir
    }

    pub fn container_workspace_dir(&self) -> &Path {
        &self.config.container_workspace_dir
    }

    pub fn driver(&self) -> &Arc<PodmanDriver> {
        &self.driver
    }

    // --- Lifecycle Operations ---

    /// Inspect the underlying container
    pub async fn inspect(&self) -> Result<Option<ContainerInspectInfo>> {
        self.driver.inspect(&self.config.container_name).await
    }

    /// Check if container is running
    pub async fn is_running(&self) -> Result<bool> {
        if let Some(info) = self.inspect().await? {
            Ok(info.is_running)
        } else {
            Ok(false)
        }
    }

    /// Get current container status
    pub async fn status(&self) -> Result<ContainerStatus> {
        if let Some(info) = self.inspect().await? {
            Ok(info.status)
        } else {
            Err(SandboxError::ContainerNotFound(
                self.config.container_name.clone(),
            ))
        }
    }

    /// Start the container if stopped
    pub async fn start(&self) -> Result<()> {
        self.driver.start(&self.config.container_name).await
    }

    /// Stop the container gracefully with timeout
    pub async fn stop(&self, timeout_secs: u32) -> Result<()> {
        self.driver
            .stop(&self.config.container_name, timeout_secs)
            .await
    }

    /// Kill the container immediately
    pub async fn kill(&self) -> Result<()> {
        self.driver.kill(&self.config.container_name).await
    }

    /// Pause the container
    pub async fn pause(&self) -> Result<()> {
        self.driver.pause(&self.config.container_name).await
    }

    /// Unpause the container
    pub async fn unpause(&self) -> Result<()> {
        self.driver.unpause(&self.config.container_name).await
    }

    /// Restart the container
    pub async fn restart(&self, timeout_secs: u32) -> Result<()> {
        self.stop(timeout_secs).await?;
        self.start().await
    }

    /// Destroy/remove the container
    pub async fn destroy(&self) -> Result<()> {
        self.driver.remove(&self.config.container_name, true).await
    }

    // --- Command Execution Operations ---

    /// Execute command in container with default working directory (`container_workspace_dir`)
    pub async fn exec<I, S>(&self, cmd: I) -> Result<ExecResult>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let opts = ExecOptions::new(cmd).working_dir(&self.config.container_workspace_dir);
        self.exec_with_options(opts).await
    }

    /// Execute command in container with explicit options
    pub async fn exec_with_options(&self, mut opts: ExecOptions) -> Result<ExecResult> {
        if opts.working_dir.is_none() {
            opts.working_dir = Some(self.config.container_workspace_dir.clone());
        }
        self.driver.exec(&self.config.container_name, &opts).await
    }

    /// Execute command in container with streaming stdout/stderr
    pub async fn exec_stream(&self, mut opts: ExecOptions) -> Result<ExecStream> {
        if opts.working_dir.is_none() {
            opts.working_dir = Some(self.config.container_workspace_dir.clone());
        }
        self.driver
            .exec_stream(&self.config.container_name, &opts)
            .await
    }

    /// Execute command in container with streaming output AND a writable stdin, for commands
    /// that need real interactive input mid-run (e.g. an agent CLI's account-login flow).
    pub async fn exec_interactive(&self, mut opts: ExecOptions) -> Result<InteractiveExec> {
        if opts.working_dir.is_none() {
            opts.working_dir = Some(self.config.container_workspace_dir.clone());
        }
        self.driver
            .exec_interactive(&self.config.container_name, &opts)
            .await
    }

    // --- File Operations ---

    /// Write binary file to user's workspace
    pub async fn save_file(&self, rel_path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
        fs::write_file_safe(&self.config.host_workspace_dir, rel_path.as_ref(), content)
    }

    /// Write string file to user's workspace
    pub async fn save_file_str(&self, rel_path: impl AsRef<Path>, content: &str) -> Result<()> {
        self.save_file(rel_path, content.as_bytes()).await
    }

    /// Read file content from user's workspace
    pub async fn read_file(&self, rel_path: impl AsRef<Path>) -> Result<Vec<u8>> {
        fs::read_file_safe(&self.config.host_workspace_dir, rel_path.as_ref())
    }

    /// Read file content as UTF-8 string from user's workspace
    pub async fn read_file_str(&self, rel_path: impl AsRef<Path>) -> Result<String> {
        let bytes = self.read_file(rel_path).await?;
        String::from_utf8(bytes).map_err(|e| SandboxError::Internal(e.to_string()))
    }

    /// Check if file exists in user's workspace
    pub fn file_exists(&self, rel_path: impl AsRef<Path>) -> bool {
        fs::file_exists_safe(&self.config.host_workspace_dir, rel_path.as_ref())
    }

    /// Remove a file or directory in user's workspace
    pub async fn remove_file(&self, rel_path: impl AsRef<Path>) -> Result<()> {
        fs::remove_file_safe(&self.config.host_workspace_dir, rel_path.as_ref())
    }

    /// List files in user's workspace directory
    pub async fn list_files(&self, rel_path: impl AsRef<Path>) -> Result<Vec<FileEntry>> {
        fs::list_dir_safe(&self.config.host_workspace_dir, rel_path.as_ref())
    }

    /// Copy a host file into container at an arbitrary path (outside workspace mount)
    pub async fn copy_into(&self, host_src: &Path, container_dest: &Path) -> Result<()> {
        self.driver
            .copy_to(&self.config.container_name, host_src, container_dest)
            .await
    }

    /// Copy a container file from an arbitrary path out to host
    pub async fn copy_out(&self, container_src: &Path, host_dest: &Path) -> Result<()> {
        self.driver
            .copy_from(&self.config.container_name, container_src, host_dest)
            .await
    }

    // --- Toolchain Integrations ---

    pub fn rust(&self) -> RustToolchain<'_> {
        RustToolchain::new(self)
    }

    pub fn python(&self) -> PythonToolchain<'_> {
        PythonToolchain::new(self)
    }

    pub fn git(&self) -> GitToolchain<'_> {
        GitToolchain::new(self)
    }

    pub fn r(&self) -> RToolchain<'_> {
        RToolchain::new(self)
    }

    pub fn latex(&self) -> LatexToolchain<'_> {
        LatexToolchain::new(self)
    }

    pub fn typst(&self) -> TypstToolchain<'_> {
        TypstToolchain::new(self)
    }

    pub fn agents(&self) -> AgentToolchain<'_> {
        AgentToolchain::new(self)
    }
}
