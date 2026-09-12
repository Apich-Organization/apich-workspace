//! Low-level Podman CLI and container driver implementation.

use crate::config::SandboxConfig;
use crate::error::Result;
use crate::error::SandboxError;
use crate::exec::ExecOptions;
use crate::exec::ExecResult;
use crate::exec::ExecStream;
use crate::exec::InteractiveExec;
use crate::exec::OutputChunk;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::info;

/// Lifecycle execution status of a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    /// Container is actively running.
    Running,
    /// Container process is paused.
    Paused,
    /// Container process has exited.
    Exited,
    /// Container has been created but not started.
    Created,
    /// Container is stopped.
    Stopped,
    /// Status could not be determined.
    Unknown,
}

impl From<&str> for ContainerStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            | "running" => Self::Running,
            | "paused" => Self::Paused,
            | "exited" => Self::Exited,
            | "created" => Self::Created,
            | "stopped" => Self::Stopped,
            | _ => Self::Unknown,
        }
    }
}

/// Raw state information returned by `podman inspect`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStateInfo {
    /// Textual status label.
    #[serde(rename = "Status")]
    pub status: String,
    /// Whether container is in running state.
    #[serde(rename = "Running")]
    pub running: bool,
    /// Whether container is paused.
    #[serde(rename = "Paused", default)]
    pub paused: bool,
    /// Host PID of the main container process.
    #[serde(rename = "Pid", default)]
    pub pid: Option<u32>,
    /// Exit code if container has terminated.
    #[serde(rename = "ExitCode", default)]
    pub exit_code: Option<i32>,
    /// ISO-8601 timestamp when container started.
    #[serde(rename = "StartedAt", default)]
    pub started_at: Option<String>,
    /// ISO-8601 timestamp when container finished.
    #[serde(rename = "FinishedAt", default)]
    pub finished_at: Option<String>,
}

/// Deserialized raw output from `podman inspect`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInspectOutput {
    /// Container ID.
    #[serde(rename = "Id")]
    pub id: String,
    /// Container name.
    #[serde(rename = "Name")]
    pub name: String,
    /// Container state metadata.
    #[serde(rename = "State")]
    pub state: ContainerStateInfo,
    /// Timestamp when container was created.
    #[serde(rename = "Created", default)]
    pub created: Option<String>,
    /// Resolved image ID the container was created from.
    #[serde(rename = "Image", default)]
    pub image: String,
}

/// Normalized container inspection details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInspectInfo {
    /// Container unique ID.
    pub id: String,
    /// Container name.
    pub name: String,
    /// High-level status enum.
    pub status: ContainerStatus,
    /// True if currently running.
    pub is_running: bool,
    /// True if currently paused.
    pub is_paused: bool,
    /// Main process ID on host.
    pub pid: Option<u32>,
    /// Termination exit code.
    pub exit_code: Option<i32>,
    /// Start timestamp string.
    pub started_at: Option<String>,
    /// Completion timestamp string.
    pub finished_at: Option<String>,
    /// Creation timestamp string.
    pub created_at: Option<String>,
    /// Image ID the container is running.
    pub image_id: String,
}

/// Low-level Podman process execution driver.
#[derive(Debug, Clone)]
pub struct PodmanDriver {
    bin_path: PathBuf,
}

impl Default for PodmanDriver {
    fn default() -> Self {
        Self {
            bin_path: PathBuf::from("podman"),
        }
    }
}

impl PodmanDriver {
    /// Creates a new `PodmanDriver`, optionally specifying a custom binary path.
    #[must_use]
    pub fn new(bin_path: Option<PathBuf>) -> Self {
        Self {
            bin_path: bin_path.unwrap_or_else(|| PathBuf::from("podman")),
        }
    }

    /// Check if Podman is installed and available
    ///
    /// # Errors
    /// Returns an error if Podman is not found or fails to execute.
    pub async fn check_available(&self) -> Result<String> {
        let output = Command::new(&self.bin_path)
            .arg("version")
            .output()
            .await
            .map_err(|_| SandboxError::PodmanNotFound)?;

        if !output.status.success() {
            return Err(SandboxError::PodmanNotFound);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Create and run a detached container
    ///
    /// # Errors
    /// Returns an error if the container fails to run or Podman exits with failure.
    pub async fn run_detached(
        &self,
        config: &SandboxConfig,
    ) -> Result<String> {
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("run").arg("-d");

        cmd.arg("--name").arg(&config.container_name);

        if config.init_process {
            cmd.arg("--init");
        }

        if config.keep_id {
            cmd.arg("--userns=keep-id");
        }

        if let Some(net) = &config.network {
            cmd.arg("--network").arg(net);
        }

        if let Some(mem) = &config.resources.memory {
            cmd.arg("--memory").arg(mem);
        }

        if let Some(cpus) = config.resources.cpus {
            cmd.arg("--cpus").arg(cpus.to_string());
        }

        if let Some(pids) = config.resources.pids_limit {
            cmd.arg("--pids-limit").arg(pids.to_string());
        }

        for (k, v) in &config.labels {
            cmd.arg("-l").arg(format!("{k}={v}"));
        }

        for (k, v) in &config.env {
            cmd.arg("-e").arg(format!("{k}={v}"));
        }

        // Workspace volume mount
        let mount_flag = if config.selinux_relabel {
            ":Z"
        } else {
            ""
        };
        let ws_mount = format!(
            "{}:{}{}",
            config.host_workspace_dir.display(),
            config.container_workspace_dir.display(),
            mount_flag
        );
        cmd.arg("-v").arg(ws_mount);

        // Extra mounts
        for extra in &config.extra_mounts {
            let mut opt = String::new();
            if extra.read_only {
                opt.push_str("ro");
            } else {
                opt.push_str("rw");
            }
            if let Some(ref selinux) = extra.selinux_label {
                opt.push(',');
                opt.push_str(selinux);
            }
            cmd.arg("-v").arg(format!(
                "{}:{}:{}",
                extra.host_path.display(),
                extra.container_path.display(),
                opt
            ));
        }

        cmd.arg("-w").arg(&config.container_workspace_dir);

        cmd.arg(&config.image);

        // Default entrypoint / keep alive
        cmd.arg("sleep").arg("infinity");

        debug!("Executing podman run: {:?}", cmd);

        let output = cmd.output().await.map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("already in use") {
                return Err(SandboxError::ContainerAlreadyExists(
                    config.container_name.clone(),
                ));
            }
            return Err(SandboxError::CommandFailed {
                container: config.container_name.clone(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(
            container_name = %config.container_name,
            container_id = %container_id,
            "Detached container started"
        );
        Ok(container_id)
    }

    /// Inspect a container
    ///
    /// # Errors
    /// Returns an error if the inspect command fails or deserialization fails.
    pub async fn inspect(
        &self,
        container_name: &str,
    ) -> Result<Option<ContainerInspectInfo>> {
        let output = Command::new(&self.bin_path)
            .arg("inspect")
            .arg(container_name)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no such object")
                || stderr.contains("no such container")
                || stderr.contains("not found")
                || stderr.contains("no such")
            {
                return Ok(None);
            }
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            });
        }

        let raw_list: Vec<RawInspectOutput> = serde_json::from_slice(&output.stdout)?;
        if let Some(first) = raw_list.into_iter().next() {
            let status = ContainerStatus::from(first.state.status.as_str());
            let is_running = first.state.running;
            let is_paused = first.state.paused;

            Ok(Some(ContainerInspectInfo {
                id: first.id,
                name: first.name,
                status,
                is_running,
                is_paused,
                pid: first.state.pid,
                exit_code: first.state.exit_code,
                started_at: first.state.started_at,
                finished_at: first.state.finished_at,
                created_at: first.created,
                image_id: first.image,
            }))
        } else {
            Ok(None)
        }
    }

    /// Resolve a tag (e.g. "localhost/apich-sandbox:latest") to the concrete image ID it
    /// currently points at, so callers can tell a stale, already-created container (still
    /// pinned to whatever image ID existed at its own `podman run` time) apart from what
    /// rebuilding the image via `podman build` just produced. Returns `Ok(None)` rather than an
    /// error if the image simply hasn't been pulled/built yet -- that's not this call's problem
    /// to report, `run_detached`'s own error on a missing image is the right place for that.
    ///
    /// # Errors
    /// Returns an error if executing the inspect image command fails.
    pub async fn image_id(
        &self,
        image: &str,
    ) -> Result<Option<String>> {
        let output = Command::new(&self.bin_path)
            .arg("image")
            .arg("inspect")
            .arg(image)
            .arg("--format")
            .arg("{{.Id}}")
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            return Ok(None);
        }

        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(id))
        }
    }

    /// Start a stopped container
    ///
    /// # Errors
    /// Returns an error if starting the container fails.
    pub async fn start(
        &self,
        container_name: &str,
    ) -> Result<()> {
        let output = Command::new(&self.bin_path)
            .arg("start")
            .arg(container_name)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Stop a running container gracefully
    ///
    /// # Errors
    /// Returns an error if stopping the container fails.
    pub async fn stop(
        &self,
        container_name: &str,
        timeout_secs: u32,
    ) -> Result<()> {
        let output = Command::new(&self.bin_path)
            .arg("stop")
            .arg("-t")
            .arg(timeout_secs.to_string())
            .arg(container_name)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("no such object")
                || stderr.contains("no such container")
                || stderr.contains("not found")
            {
                return Ok(());
            }
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Kill a container immediately
    ///
    /// # Errors
    /// Returns an error if killing the container fails.
    pub async fn kill(
        &self,
        container_name: &str,
    ) -> Result<()> {
        let output = Command::new(&self.bin_path)
            .arg("kill")
            .arg(container_name)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("no such object")
                || stderr.contains("no such container")
                || stderr.contains("not found")
            {
                return Ok(());
            }
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Pause a running container
    ///
    /// # Errors
    /// Returns an error if pausing the container fails.
    pub async fn pause(
        &self,
        container_name: &str,
    ) -> Result<()> {
        let output = Command::new(&self.bin_path)
            .arg("pause")
            .arg(container_name)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Unpause a paused container
    ///
    /// # Errors
    /// Returns an error if unpausing the container fails.
    pub async fn unpause(
        &self,
        container_name: &str,
    ) -> Result<()> {
        let output = Command::new(&self.bin_path)
            .arg("unpause")
            .arg(container_name)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Remove a container
    ///
    /// # Errors
    /// Returns an error if removing the container fails.
    pub async fn remove(
        &self,
        container_name: &str,
        force: bool,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("rm");
        if force {
            cmd.arg("-f");
        }
        cmd.arg(container_name);

        let output = cmd.output().await.map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("no such object")
                || stderr.contains("no such container")
                || stderr.contains("not found")
            {
                return Ok(());
            }
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Copy a file or directory from host into container
    ///
    /// # Errors
    /// Returns an error if copying into the container fails.
    pub async fn copy_to(
        &self,
        container_name: &str,
        host_src: &Path,
        container_dest: &Path,
    ) -> Result<()> {
        let dest = format!("{}:{}", container_name, container_dest.display());
        let output = Command::new(&self.bin_path)
            .arg("cp")
            .arg(host_src)
            .arg(dest)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Copy a file or directory from container to host
    ///
    /// # Errors
    /// Returns an error if copying from the container fails.
    pub async fn copy_from(
        &self,
        container_name: &str,
        container_src: &Path,
        host_dest: &Path,
    ) -> Result<()> {
        let src = format!("{}:{}", container_name, container_src.display());
        let output = Command::new(&self.bin_path)
            .arg("cp")
            .arg(src)
            .arg(host_dest)
            .output()
            .await
            .map_err(SandboxError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::CommandFailed {
                container: container_name.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }
        Ok(())
    }

    /// Execute a command in a running container and collect all output
    ///
    /// # Errors
    /// Returns an error if command execution fails or the process cannot be spawned.
    pub async fn exec(
        &self,
        container_name: &str,
        opts: &ExecOptions,
    ) -> Result<ExecResult> {
        let start = Instant::now();
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("exec");

        if opts.tty {
            cmd.arg("-t");
        }

        if let Some(ref wd) = opts.working_dir {
            cmd.arg("-w").arg(wd);
        }

        if let Some(ref user) = opts.user {
            cmd.arg("-u").arg(user);
        }

        for (k, v) in &opts.env {
            cmd.arg("-e").arg(format!("{k}={v}"));
        }

        cmd.arg(container_name);

        for arg in &opts.cmd {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(SandboxError::Io)?;
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        let stdout_fut = async {
            let mut buf = Vec::new();
            if let Some(mut r) = stdout_pipe.take() {
                let _ = r.read_to_end(&mut buf).await;
            }
            buf
        };

        let stderr_fut = async {
            let mut buf = Vec::new();
            if let Some(mut r) = stderr_pipe.take() {
                let _ = r.read_to_end(&mut buf).await;
            }
            buf
        };

        let wait_fut = child.wait();

        if let Some(timeout) = opts.timeout {
            let combined = async { tokio::join!(wait_fut, stdout_fut, stderr_fut) };
            match tokio::time::timeout(timeout, combined).await {
                | Ok((status_res, stdout, stderr)) => {
                    let status = status_res.map_err(SandboxError::Io)?;
                    Ok(ExecResult {
                        exit_code: status.code().unwrap_or(-1),
                        stdout,
                        stderr,
                        duration: start.elapsed(),
                    })
                },
                | Err(_) => {
                    let _ = child.kill().await;
                    Err(SandboxError::ExecutionTimeout(timeout))
                },
            }
        } else {
            let (status_res, stdout, stderr) = tokio::join!(wait_fut, stdout_fut, stderr_fut);
            let status = status_res.map_err(SandboxError::Io)?;
            Ok(ExecResult {
                exit_code: status.code().unwrap_or(-1),
                stdout,
                stderr,
                duration: start.elapsed(),
            })
        }
    }

    /// Execute a command in a running container and stream stdout/stderr chunks in real-time
    ///
    /// # Errors
    /// Returns an error if spawning the exec command fails.
    pub async fn exec_stream(
        &self,
        container_name: &str,
        opts: &ExecOptions,
    ) -> Result<ExecStream> {
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("exec");

        if opts.tty {
            cmd.arg("-t");
        }

        if let Some(ref wd) = opts.working_dir {
            cmd.arg("-w").arg(wd);
        }

        if let Some(ref user) = opts.user {
            cmd.arg("-u").arg(user);
        }

        for (k, v) in &opts.env {
            cmd.arg("-e").arg(format!("{k}={v}"));
        }

        cmd.arg(container_name);

        for arg in &opts.cmd {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(SandboxError::Io)?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel(128);

        if let Some(out) = stdout {
            spawn_chunk_reader(out, tx.clone(), true);
        }

        if let Some(err) = stderr {
            spawn_chunk_reader(err, tx.clone(), false);
        }

        spawn_exit_monitor(child, opts.timeout, tx);

        tokio::task::yield_now().await;

        Ok(ExecStream::new(rx))
    }

    /// Like `exec_stream`, but also opens the process's stdin for writing -- for commands that
    /// need real interactive input mid-run (e.g. `claude auth login`, which prints an OAuth URL
    /// then waits for the user to paste back a code from the browser callback page). Passes
    /// `-i` to `podman exec` so stdin is actually forwarded into the container; plain
    /// `exec`/`exec_stream` don't, and input written to a non-`-i` exec session is silently
    /// discarded by podman.
    ///
    /// # Errors
    /// Returns an error if spawning the interactive process or taking standard I/O handles fails.
    pub async fn exec_interactive(
        &self,
        container_name: &str,
        opts: &ExecOptions,
    ) -> Result<InteractiveExec> {
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("exec").arg("-i");

        if opts.tty {
            cmd.arg("-t");
        }

        if let Some(ref wd) = opts.working_dir {
            cmd.arg("-w").arg(wd);
        }

        if let Some(ref user) = opts.user {
            cmd.arg("-u").arg(user);
        }

        for (k, v) in &opts.env {
            cmd.arg("-e").arg(format!("{k}={v}"));
        }

        cmd.arg(container_name);

        for arg in &opts.cmd {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(SandboxError::Io)?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (out_tx, out_rx) = mpsc::channel(128);
        let (in_tx, mut in_rx) = mpsc::channel::<Vec<u8>>(16);

        if let Some(mut stdin) = stdin {
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                while let Some(bytes) = in_rx.recv().await {
                    if stdin.write_all(&bytes).await.is_err() {
                        break;
                    }
                    let _ = stdin.flush().await;
                }
            });
        }

        if let Some(out) = stdout {
            spawn_chunk_reader(out, out_tx.clone(), true);
        }

        if let Some(err) = stderr {
            spawn_chunk_reader(err, out_tx.clone(), false);
        }

        spawn_exit_monitor(child, opts.timeout, out_tx);

        tokio::task::yield_now().await;

        Ok(InteractiveExec {
            stream: ExecStream::new(out_rx),
            stdin_tx: in_tx,
        })
    }

    /// List container names matching a label filter
    ///
    /// # Errors
    /// Returns an error if querying container names fails.
    pub async fn list_containers(
        &self,
        label_filter: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("ps").arg("-a").arg("--format").arg("{{.Names}}");

        if let Some(filter) = label_filter {
            cmd.arg("--filter").arg(format!("label={filter}"));
        }

        let output = cmd.output().await.map_err(SandboxError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::CommandFailed {
                container: "host".to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let names = text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Ok(names)
    }
}

fn spawn_chunk_reader<R>(
    mut reader: R,
    tx: mpsc::Sender<OutputChunk>,
    is_stdout: bool,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                | Ok(0) => break,
                | Ok(n) => {
                    let chunk = buf.get(..n).unwrap_or_default().to_vec();
                    let output = if is_stdout {
                        OutputChunk::Stdout(chunk)
                    } else {
                        OutputChunk::Stderr(chunk)
                    };
                    if tx.send(output).await.is_err() {
                        break;
                    }
                },
                | Err(e) => {
                    error!("Error reading output stream: {e}");
                    break;
                },
            }
        }
    });
}

fn spawn_exit_monitor(
    mut child: tokio::process::Child,
    timeout: Option<Duration>,
    tx: mpsc::Sender<OutputChunk>,
) {
    tokio::spawn(async move {
        let wait_fut = child.wait();
        let exit_code = if let Some(dur) = timeout {
            match tokio::time::timeout(dur, wait_fut).await {
                | Ok(res) => {
                    match res {
                        | Ok(status) => status.code().unwrap_or(-1),
                        | Err(_) => -1,
                    }
                },
                | Err(_) => {
                    let _ = child.kill().await;
                    -124
                },
            }
        } else {
            match wait_fut.await {
                | Ok(status) => status.code().unwrap_or(-1),
                | Err(_) => -1,
            }
        };

        let _ = tx.send(OutputChunk::Exit(exit_code)).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_conversion() {
        assert_eq!(ContainerStatus::from("running"), ContainerStatus::Running);
        assert_eq!(ContainerStatus::from("RUNNING"), ContainerStatus::Running);
        assert_eq!(ContainerStatus::from("paused"), ContainerStatus::Paused);
        assert_eq!(ContainerStatus::from("exited"), ContainerStatus::Exited);
        assert_eq!(ContainerStatus::from("created"), ContainerStatus::Created);
        assert_eq!(ContainerStatus::from("stopped"), ContainerStatus::Stopped);
        assert_eq!(
            ContainerStatus::from("something_else"),
            ContainerStatus::Unknown
        );
    }

    #[test]
    fn test_parse_inspect_json() {
        let json_sample = r#"[
            {
                "Id": "abc123456",
                "Name": "apich-sandbox-user1",
                "Image": "sha256:deadbeef",
                "State": {
                    "Status": "running",
                    "Running": true,
                    "Paused": false,
                    "Pid": 4321,
                    "ExitCode": 0,
                    "StartedAt": "2026-09-06T10:00:00Z",
                    "FinishedAt": ""
                },
                "Created": "2026-09-06T09:59:00Z"
            }
        ]"#;

        let parsed: Vec<RawInspectOutput> = serde_json::from_str(json_sample).unwrap();
        assert_eq!(parsed.len(), 1);
        let first = &parsed[0];
        assert_eq!(first.id, "abc123456");
        assert_eq!(first.name, "apich-sandbox-user1");
        assert_eq!(first.image, "sha256:deadbeef");
        assert_eq!(first.state.status, "running");
        assert!(first.state.running);
        assert_eq!(first.state.pid, Some(4321));
    }

    #[test]
    fn test_parse_inspect_json_missing_image_field_defaults_empty() {
        // Older podman versions or an unusual inspect payload might omit "Image" entirely --
        // `#[serde(default)]` must keep this parseable rather than failing the whole inspect.
        let json_sample = r#"[
            {
                "Id": "abc123456",
                "Name": "apich-sandbox-user1",
                "State": {
                    "Status": "running",
                    "Running": true,
                    "Paused": false,
                    "Pid": 4321,
                    "ExitCode": 0,
                    "StartedAt": "2026-09-06T10:00:00Z",
                    "FinishedAt": ""
                },
                "Created": "2026-09-06T09:59:00Z"
            }
        ]"#;

        let parsed: Vec<RawInspectOutput> = serde_json::from_str(json_sample).unwrap();
        assert_eq!(parsed[0].image, "");
    }
}
