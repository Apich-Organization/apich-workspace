//! Command execution options, results, and asynchronous output streaming.

use crate::error::Result;
use crate::error::SandboxError;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

/// Configuration options for executing a command inside a container.
#[derive(Debug, Clone)]
pub struct ExecOptions {
    /// Command and arguments to execute
    pub cmd: Vec<String>,
    /// Working directory inside container (defaults to container workspace)
    pub working_dir: Option<PathBuf>,
    /// Environment variables for this command
    pub env: HashMap<String, String>,
    /// Container user to run command as (e.g. "root" or "apich")
    pub user: Option<String>,
    /// Allocate pseudo-TTY
    pub tty: bool,
    /// Maximum execution timeout
    pub timeout: Option<Duration>,
}

impl ExecOptions {
    /// Creates a new `ExecOptions` instance with the specified command arguments.
    #[must_use]
    pub fn new<I, S>(cmd: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            cmd: cmd.into_iter().map(|s| s.as_ref().to_string()).collect(),
            working_dir: None,
            env: HashMap::new(),
            user: None,
            tty: false,
            timeout: None,
        }
    }

    /// Sets the working directory inside the container for command execution.
    #[must_use]
    pub fn working_dir(
        mut self,
        dir: impl AsRef<Path>,
    ) -> Self {
        self.working_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Adds an environment variable for the command execution.
    #[must_use]
    pub fn env(
        mut self,
        key: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        self.env.insert(key.into(), val.into());
        self
    }

    /// Adds multiple environment variables for the command execution.
    #[must_use]
    pub fn envs(
        mut self,
        envs: HashMap<String, String>,
    ) -> Self {
        self.env.extend(envs);
        self
    }

    /// Specifies the container user account to execute the command as.
    #[must_use]
    pub fn user(
        mut self,
        user: impl Into<String>,
    ) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Toggles pseudo-TTY allocation for interactive or color-enabled output.
    #[must_use]
    pub const fn tty(
        mut self,
        tty: bool,
    ) -> Self {
        self.tty = tty;
        self
    }

    /// Specifies an execution timeout.
    #[must_use]
    pub const fn timeout(
        mut self,
        timeout: Duration,
    ) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// The result of an executed command
#[derive(Debug, Clone)]
pub struct ExecResult {
    /// Process exit code.
    pub exit_code: i32,
    /// Raw bytes captured from stdout.
    pub stdout: Vec<u8>,
    /// Raw bytes captured from stderr.
    pub stderr: Vec<u8>,
    /// Execution elapsed time.
    pub duration: Duration,
}

impl ExecResult {
    /// Returns true if the process exited with status code 0.
    #[must_use]
    pub const fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Attempts to parse stdout as valid UTF-8 text.
    ///
    /// # Errors
    /// Returns an error if stdout is not valid UTF-8.
    pub fn stdout_str(&self) -> std::result::Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.stdout)
    }

    /// Attempts to parse stderr as valid UTF-8 text.
    ///
    /// # Errors
    /// Returns an error if stderr is not valid UTF-8.
    pub fn stderr_str(&self) -> std::result::Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.stderr)
    }

    /// Converts stdout into a lossy UTF-8 string.
    #[must_use]
    pub fn stdout_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    /// Converts stderr into a lossy UTF-8 string.
    #[must_use]
    pub fn stderr_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }

    /// Verifies exit code 0 or returns a `SandboxError::CommandFailed`.
    ///
    /// # Errors
    /// Returns `SandboxError::CommandFailed` if the exit code was non-zero.
    pub fn ensure_success(
        &self,
        container: &str,
    ) -> Result<()> {
        if self.success() {
            Ok(())
        } else {
            Err(SandboxError::CommandFailed {
                container: container.to_string(),
                exit_code: self.exit_code,
                stderr: self.stderr_lossy().into_owned(),
            })
        }
    }
}

/// Streamed chunk of execution output
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputChunk {
    /// Standard output data bytes.
    Stdout(Vec<u8>),
    /// Standard error data bytes.
    Stderr(Vec<u8>),
    /// Process exit event with exit code.
    Exit(i32),
}

/// Handle to receive streaming execution output asynchronously
pub struct ExecStream {
    receiver: mpsc::Receiver<OutputChunk>,
}

impl ExecStream {
    /// Creates a new `ExecStream` wrapping an asynchronous channel receiver.
    #[must_use]
    pub const fn new(receiver: mpsc::Receiver<OutputChunk>) -> Self {
        Self { receiver }
    }

    /// Receive the next chunk of output, or None if stream ended
    pub async fn next_chunk(&mut self) -> Option<OutputChunk> {
        self.receiver.recv().await
    }

    /// Read all output from stream until completion and assemble into an `ExecResult`
    pub async fn collect_result(
        mut self,
        start_time: std::time::Instant,
    ) -> ExecResult {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0;

        while let Some(chunk) = self.receiver.recv().await {
            match chunk {
                | OutputChunk::Stdout(bytes) => stdout.extend_from_slice(&bytes),
                | OutputChunk::Stderr(bytes) => stderr.extend_from_slice(&bytes),
                | OutputChunk::Exit(code) => exit_code = code,
            }
        }

        ExecResult {
            exit_code,
            stdout,
            stderr,
            duration: start_time.elapsed(),
        }
    }
}

/// A running exec session with a writable stdin.
///
/// For commands that need real interactive input
/// mid-run -- e.g. `claude auth login`, which prints an OAuth URL and then waits for the user to
/// paste back a code from the browser callback page. Plain `ExecStream` only reads output;
/// this additionally lets the caller write bytes into the process's stdin at any point before it
/// exits. Dropping `stdin_tx` (or the whole `InteractiveExec`) closes stdin, which is how a
/// command that reads until EOF (rather than a specific delimiter) is told input is done.
pub struct InteractiveExec {
    /// Output streaming handle.
    pub stream: ExecStream,
    /// Channel sender to transmit input bytes to the running command's stdin.
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_options_builder() {
        let opts = ExecOptions::new(["python3", "-c", "print(1)"])
            .working_dir("/tmp")
            .env("FOO", "BAR")
            .user("root")
            .tty(true)
            .timeout(Duration::from_secs(5));

        assert_eq!(opts.cmd, vec!["python3", "-c", "print(1)"]);
        assert_eq!(opts.working_dir, Some(PathBuf::from("/tmp")));
        assert_eq!(opts.env.get("FOO").unwrap(), "BAR");
        assert_eq!(opts.user.as_deref(), Some("root"));
        assert!(opts.tty);
        assert_eq!(opts.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_exec_result() {
        let res = ExecResult {
            exit_code: 0,
            stdout: b"hello world\n".to_vec(),
            stderr: Vec::new(),
            duration: Duration::from_millis(10),
        };

        assert!(res.success());
        assert_eq!(res.stdout_str().unwrap(), "hello world\n");
        assert_eq!(res.stdout_lossy(), "hello world\n");
        assert!(res.ensure_success("test-container").is_ok());

        let err_res = ExecResult {
            exit_code: 1,
            stdout: Vec::new(),
            stderr: b"error occurred".to_vec(),
            duration: Duration::from_millis(10),
        };
        assert!(!err_res.success());
        assert!(err_res.ensure_success("test-container").is_err());
    }
}
