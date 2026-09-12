//! High-level user container lifecycle abstraction.

use super::config::SandboxConfig;
use super::driver::ContainerInspectInfo;
use super::driver::ContainerStatus;
use super::driver::PodmanDriver;
use super::error::Result;
use super::error::SandboxError;
use super::exec::ExecOptions;
use super::exec::ExecResult;
use super::exec::ExecStream;
use super::exec::InteractiveExec;
use super::fs::FileEntry;
use super::fs::{
    self,
};
use super::tools::AgentToolchain;
use super::tools::GitToolchain;
use super::tools::LatexToolchain;
use super::tools::PythonToolchain;
use super::tools::RToolchain;
use super::tools::RustToolchain;
use super::tools::TypstToolchain;
use std::path::Path;
use std::sync::Arc;

/// High-level handle to a user's isolated sandbox container
#[derive(Clone)]
pub struct UserContainer {
    config: SandboxConfig,
    driver: Arc<PodmanDriver>,
}

impl UserContainer {
    /// Creates a new `UserContainer` instance wrapping the given configuration and Podman driver.
    #[must_use]
    pub const fn new(
        config: SandboxConfig,
        driver: Arc<PodmanDriver>,
    ) -> Self {
        Self { config, driver }
    }

    /// Returns the user ID associated with this container.
    #[must_use]
    pub fn user_id(&self) -> &str {
        &self.config.user_id
    }

    /// Returns the Podman container name.
    #[must_use]
    pub fn container_name(&self) -> &str {
        &self.config.container_name
    }

    /// Returns a reference to the container configuration.
    #[must_use]
    pub const fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Returns the host directory path mounted into the container.
    #[must_use]
    pub fn host_workspace_dir(&self) -> &Path {
        &self.config.host_workspace_dir
    }

    /// Returns the workspace mount destination path inside the container.
    #[must_use]
    pub fn container_workspace_dir(&self) -> &Path {
        &self.config.container_workspace_dir
    }

    /// Returns a reference to the underlying Podman driver.
    #[must_use]
    pub const fn driver(&self) -> &Arc<PodmanDriver> {
        &self.driver
    }

    // --- Lifecycle Operations ---

    /// Inspect the underlying container
    ///
    /// # Errors
    /// Returns an error if the container inspect command fails.
    pub async fn inspect(&self) -> Result<Option<ContainerInspectInfo>> {
        self.driver.inspect(&self.config.container_name).await
    }

    /// Check if container is running
    ///
    /// # Errors
    /// Returns an error if querying the container status fails.
    pub async fn is_running(&self) -> Result<bool> {
        if let Some(info) = self.inspect().await? {
            Ok(info.is_running)
        } else {
            Ok(false)
        }
    }

    /// Get current container status
    ///
    /// # Errors
    /// Returns an error if the container is not found or querying the driver fails.
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
    ///
    /// # Errors
    /// Returns an error if starting the container fails.
    pub async fn start(&self) -> Result<()> {
        self.driver.start(&self.config.container_name).await
    }

    /// Stop the container gracefully with timeout
    ///
    /// # Errors
    /// Returns an error if stopping the container fails.
    pub async fn stop(
        &self,
        timeout_secs: u32,
    ) -> Result<()> {
        self.driver
            .stop(&self.config.container_name, timeout_secs)
            .await
    }

    /// Kill the container immediately
    ///
    /// # Errors
    /// Returns an error if killing the container process fails.
    pub async fn kill(&self) -> Result<()> {
        self.driver.kill(&self.config.container_name).await
    }

    /// Pause the container
    ///
    /// # Errors
    /// Returns an error if pausing the container fails.
    pub async fn pause(&self) -> Result<()> {
        self.driver.pause(&self.config.container_name).await
    }

    /// Unpause the container
    ///
    /// # Errors
    /// Returns an error if unpausing the container fails.
    pub async fn unpause(&self) -> Result<()> {
        self.driver.unpause(&self.config.container_name).await
    }

    /// Restart the container
    ///
    /// # Errors
    /// Returns an error if stopping or starting the container fails.
    pub async fn restart(
        &self,
        timeout_secs: u32,
    ) -> Result<()> {
        self.stop(timeout_secs).await?;
        self.start().await
    }

    /// Destroy/remove the container
    ///
    /// # Errors
    /// Returns an error if removing the container fails.
    pub async fn destroy(&self) -> Result<()> {
        self.driver.remove(&self.config.container_name, true).await
    }

    // --- Command Execution Operations ---

    /// Execute command in container with default working directory (`container_workspace_dir`)
    ///
    /// # Errors
    /// Returns an error if executing the command fails.
    pub async fn exec<I, S>(
        &self,
        cmd: I,
    ) -> Result<ExecResult>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let opts = ExecOptions::new(cmd).working_dir(&self.config.container_workspace_dir);
        self.exec_with_options(opts).await
    }

    /// Execute command in container with explicit options
    ///
    /// # Errors
    /// Returns an error if command execution fails.
    pub async fn exec_with_options(
        &self,
        mut opts: ExecOptions,
    ) -> Result<ExecResult> {
        if opts.working_dir.is_none() {
            opts.working_dir = Some(self.config.container_workspace_dir.clone());
        }
        self.driver.exec(&self.config.container_name, &opts).await
    }

    /// Execute command in container with streaming stdout/stderr
    ///
    /// # Errors
    /// Returns an error if starting the streaming execution fails.
    pub async fn exec_stream(
        &self,
        mut opts: ExecOptions,
    ) -> Result<ExecStream> {
        if opts.working_dir.is_none() {
            opts.working_dir = Some(self.config.container_workspace_dir.clone());
        }
        self.driver
            .exec_stream(&self.config.container_name, &opts)
            .await
    }

    /// Execute command in container with streaming output AND a writable stdin, for commands
    /// that need real interactive input mid-run (e.g. an agent CLI's account-login flow).
    ///
    /// # Errors
    /// Returns an error if launching interactive execution fails.
    pub async fn exec_interactive(
        &self,
        mut opts: ExecOptions,
    ) -> Result<InteractiveExec> {
        if opts.working_dir.is_none() {
            opts.working_dir = Some(self.config.container_workspace_dir.clone());
        }
        self.driver
            .exec_interactive(&self.config.container_name, &opts)
            .await
    }

    // --- File Operations ---

    /// Write binary file to user's workspace
    ///
    /// # Errors
    /// Returns an error if resolving the path or writing the file fails.
    pub async fn save_file(
        &self,
        rel_path: impl AsRef<Path>,
        content: &[u8],
    ) -> Result<()> {
        let base_dir = self.config.host_workspace_dir.clone();
        let rel_path = rel_path.as_ref().to_path_buf();
        let content = content.to_vec();
        tokio::task::spawn_blocking(move || fs::write_file_safe(&base_dir, &rel_path, &content))
            .await
            .map_err(|e| SandboxError::Internal(e.to_string()))?
    }

    /// Write string file to user's workspace
    ///
    /// # Errors
    /// Returns an error if saving the file fails.
    pub async fn save_file_str(
        &self,
        rel_path: impl AsRef<Path>,
        content: &str,
    ) -> Result<()> {
        self.save_file(rel_path, content.as_bytes()).await
    }

    /// Read file content from user's workspace
    ///
    /// # Errors
    /// Returns an error if resolving the path or reading the file fails.
    pub async fn read_file(
        &self,
        rel_path: impl AsRef<Path>,
    ) -> Result<Vec<u8>> {
        let base_dir = self.config.host_workspace_dir.clone();
        let rel_path = rel_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || fs::read_file_safe(&base_dir, &rel_path))
            .await
            .map_err(|e| SandboxError::Internal(e.to_string()))?
    }

    /// Read file content as UTF-8 string from user's workspace
    ///
    /// # Errors
    /// Returns an error if reading the file fails or the content is not valid UTF-8.
    pub async fn read_file_str(
        &self,
        rel_path: impl AsRef<Path>,
    ) -> Result<String> {
        let bytes = self.read_file(rel_path).await?;
        String::from_utf8(bytes).map_err(|e| SandboxError::Internal(e.to_string()))
    }

    /// Check if file exists in user's workspace
    pub fn file_exists(
        &self,
        rel_path: impl AsRef<Path>,
    ) -> bool {
        fs::file_exists_safe(&self.config.host_workspace_dir, rel_path.as_ref())
    }

    /// Remove a file or directory in user's workspace
    ///
    /// # Errors
    /// Returns an error if path resolution or removing the file fails.
    pub async fn remove_file(
        &self,
        rel_path: impl AsRef<Path>,
    ) -> Result<()> {
        let base_dir = self.config.host_workspace_dir.clone();
        let rel_path = rel_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || fs::remove_file_safe(&base_dir, &rel_path))
            .await
            .map_err(|e| SandboxError::Internal(e.to_string()))?
    }

    /// List files in user's workspace directory
    ///
    /// # Errors
    /// Returns an error if reading the directory entries fails.
    pub async fn list_files(
        &self,
        rel_path: impl AsRef<Path>,
    ) -> Result<Vec<FileEntry>> {
        let base_dir = self.config.host_workspace_dir.clone();
        let rel_path = rel_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || fs::list_dir_safe(&base_dir, &rel_path))
            .await
            .map_err(|e| SandboxError::Internal(e.to_string()))?
    }

    /// Copy a host file into container at an arbitrary path (outside workspace mount)
    ///
    /// # Errors
    /// Returns an error if the container copy command fails.
    pub async fn copy_into(
        &self,
        host_src: &Path,
        container_dest: &Path,
    ) -> Result<()> {
        self.driver
            .copy_to(&self.config.container_name, host_src, container_dest)
            .await
    }

    /// Copy a container file from an arbitrary path out to host
    ///
    /// # Errors
    /// Returns an error if copying from the container fails.
    pub async fn copy_out(
        &self,
        container_src: &Path,
        host_dest: &Path,
    ) -> Result<()> {
        self.driver
            .copy_from(&self.config.container_name, container_src, host_dest)
            .await
    }

    // --- Toolchain Integrations ---

    /// Access the Rust toolchain helper within this container.
    #[must_use]
    pub const fn rust(&self) -> RustToolchain<'_> {
        RustToolchain::new(self)
    }

    /// Access the Python toolchain helper within this container.
    #[must_use]
    pub const fn python(&self) -> PythonToolchain<'_> {
        PythonToolchain::new(self)
    }

    /// Access the Git toolchain helper within this container.
    #[must_use]
    pub const fn git(&self) -> GitToolchain<'_> {
        GitToolchain::new(self)
    }

    /// Access the R toolchain helper within this container.
    #[must_use]
    pub const fn r(&self) -> RToolchain<'_> {
        RToolchain::new(self)
    }

    /// Access the LaTeX toolchain helper within this container.
    #[must_use]
    pub const fn latex(&self) -> LatexToolchain<'_> {
        LatexToolchain::new(self)
    }

    /// Access the Typst toolchain helper within this container.
    #[must_use]
    pub const fn typst(&self) -> TypstToolchain<'_> {
        TypstToolchain::new(self)
    }

    /// Access the AI Agent toolchain helper within this container.
    #[must_use]
    pub const fn agents(&self) -> AgentToolchain<'_> {
        AgentToolchain::new(self)
    }
}
