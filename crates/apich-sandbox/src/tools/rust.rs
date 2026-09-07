use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

pub struct RustToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> RustToolchain<'a> {
    pub fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check rustc version
    pub async fn rustc_version(&self) -> Result<String> {
        let res = self.container.exec(&["rustc", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Check cargo version
    pub async fn cargo_version(&self) -> Result<String> {
        let res = self.container.exec(&["cargo", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Run `cargo build` with extra arguments
    pub async fn cargo_build(&self, extra_args: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "build"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo check`
    pub async fn cargo_check(&self, extra_args: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "check"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo test`
    pub async fn cargo_test(&self, extra_args: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "test"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo run`
    pub async fn cargo_run(&self, extra_args: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "run"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Run `cargo-slide` (Section 2 of plan.md)
    pub async fn cargo_slide(&self, extra_args: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["cargo", "slide"];
        cmd.extend(extra_args);
        self.container.exec(&cmd).await
    }

    /// Compile a single rust source file directly with rustc
    pub async fn compile_file(&self, source_path: &str, out_path: &str) -> Result<ExecResult> {
        self.container
            .exec(&["rustc", source_path, "-o", out_path])
            .await
    }
}
