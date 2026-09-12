//! Git toolchain integration for sandbox containers.

use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

/// Helper for executing Git commands inside a container.
pub struct GitToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> GitToolchain<'a> {
    /// Creates a new `GitToolchain` instance.
    #[must_use]
    pub const fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check git version
    ///
    /// # Errors
    /// Returns an error if executing git version fails.
    pub async fn git_version(&self) -> Result<String> {
        let res = self.container.exec(&["git", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Initialize a git repository
    ///
    /// # Errors
    /// Returns an error if running git init fails.
    pub async fn init(
        &self,
        repo_dir: Option<&str>,
    ) -> Result<ExecResult> {
        let dir = repo_dir.unwrap_or(".");
        self.container.exec(&["git", "-C", dir, "init"]).await
    }

    /// Run `git status --porcelain`
    ///
    /// # Errors
    /// Returns an error if running git status fails.
    pub async fn status(
        &self,
        repo_dir: Option<&str>,
    ) -> Result<ExecResult> {
        let dir = repo_dir.unwrap_or(".");
        self.container
            .exec(&["git", "-C", dir, "status", "--porcelain"])
            .await
    }

    /// Run `git add <path>`
    ///
    /// # Errors
    /// Returns an error if running git add fails.
    pub async fn add(
        &self,
        repo_dir: Option<&str>,
        path_spec: &str,
    ) -> Result<ExecResult> {
        let dir = repo_dir.unwrap_or(".");
        self.container
            .exec(&["git", "-C", dir, "add", path_spec])
            .await
    }

    /// Commit with message and optional author
    ///
    /// # Errors
    /// Returns an error if running git commit fails.
    pub async fn commit(
        &self,
        repo_dir: Option<&str>,
        message: &str,
        author: Option<(&str, &str)>, // (name, email)
    ) -> Result<ExecResult> {
        let dir = repo_dir.unwrap_or(".");
        let mut cmd = vec!["git", "-C", dir];

        let author_str;
        if let Some((name, email)) = author {
            author_str = format!("{name} <{email}>");
            cmd.extend(&[
                "-c",
                "user.name=Temp",
                "-c",
                "user.email=temp@example.com",
                "commit",
                "--author",
                &author_str,
                "-m",
                message,
            ]);
        } else {
            cmd.extend(&[
                "-c",
                "user.name=APICH",
                "-c",
                "user.email=apich@workspace.local",
                "commit",
                "-m",
                message,
            ]);
        }

        self.container.exec(&cmd).await
    }

    /// Clone repository into destination
    ///
    /// # Errors
    /// Returns an error if running git clone fails.
    pub async fn clone(
        &self,
        url: &str,
        target_dir: &str,
    ) -> Result<ExecResult> {
        self.container
            .exec(&["git", "clone", url, target_dir])
            .await
    }

    /// Show git commit log
    ///
    /// # Errors
    /// Returns an error if running git log fails.
    pub async fn log(
        &self,
        repo_dir: Option<&str>,
        max_count: Option<usize>,
    ) -> Result<ExecResult> {
        let dir = repo_dir.unwrap_or(".");
        let count_str = max_count.unwrap_or(20).to_string();
        self.container
            .exec(&["git", "-C", dir, "log", "--oneline", "-n", &count_str])
            .await
    }

    /// Get current branch name
    ///
    /// # Errors
    /// Returns an error if querying current git branch fails.
    pub async fn current_branch(
        &self,
        repo_dir: Option<&str>,
    ) -> Result<String> {
        let dir = repo_dir.unwrap_or(".");
        let res = self
            .container
            .exec(&["git", "-C", dir, "branch", "--show-current"])
            .await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Get git diff
    ///
    /// # Errors
    /// Returns an error if running git diff fails.
    pub async fn diff(
        &self,
        repo_dir: Option<&str>,
    ) -> Result<ExecResult> {
        let dir = repo_dir.unwrap_or(".");
        self.container.exec(&["git", "-C", dir, "diff"]).await
    }
}
