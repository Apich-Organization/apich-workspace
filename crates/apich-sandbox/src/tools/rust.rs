//! Rust compiler and Cargo toolchain integration.

use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

/// Helper for invoking rustc, cargo, and rustfmt inside a container.
pub struct RustToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> RustToolchain<'a> {
    /// Creates a new `RustToolchain` instance.
    #[must_use]
    pub const fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check rustc version
    ///
    /// # Errors
    /// Returns an error if querying rustc version fails.
    pub async fn rustc_version(&self) -> Result<String> {
        let res = self.container.exec(&["rustc", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Check cargo version
    ///
    /// # Errors
    /// Returns an error if querying cargo version fails.
    pub async fn cargo_version(&self) -> Result<String> {
        let res = self.container.exec(&["cargo", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Run `cargo build` with extra arguments
    ///
    /// # Errors
    /// Returns an error if executing cargo build fails.
    pub async fn cargo_build(
        &self,
        extra_args: &[&str],
    ) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "build"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo check`
    ///
    /// # Errors
    /// Returns an error if executing cargo check fails.
    pub async fn cargo_check(
        &self,
        extra_args: &[&str],
    ) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "check"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo test`
    ///
    /// # Errors
    /// Returns an error if executing cargo test fails.
    pub async fn cargo_test(
        &self,
        extra_args: &[&str],
    ) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "test"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo run`
    ///
    /// # Errors
    /// Returns an error if executing cargo run fails.
    pub async fn cargo_run(
        &self,
        extra_args: &[&str],
    ) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "run"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo-slide` (Section 2 of plan.md)
    ///
    /// # Errors
    /// Returns an error if executing cargo slide fails.
    pub async fn cargo_slide(
        &self,
        extra_args: &[&str],
    ) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "slide"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Compile a single rust source file directly with rustc
    ///
    /// # Errors
    /// Returns an error if executing rustc compile fails.
    pub async fn compile_file(
        &self,
        source_path: &str,
        out_path: &str,
    ) -> Result<ExecResult> {
        self.container
            .exec(&["rustc", source_path, "-o", out_path])
            .await
    }
}
