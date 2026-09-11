use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

pub struct RToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> RToolchain<'a> {
    pub fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check R version
    pub async fn r_version(&self) -> Result<String> {
        let res = self.container.exec(&["R", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        // R --version outputs several lines, the first line contains version
        let first_line = res.stdout_lossy().lines().next().unwrap_or("").to_string();
        Ok(first_line)
    }

    /// Run R expression using Rscript -e
    pub async fn run_code(
        &self,
        code: &str,
    ) -> Result<ExecResult> {
        self.container.exec(&["Rscript", "-e", code]).await
    }

    /// Run R script file with arguments
    pub async fn run_file(
        &self,
        script_path: &str,
        args: &[&str],
    ) -> Result<ExecResult> {
        let mut cmd = vec!["Rscript", script_path];
        cmd.extend(args);
        self.container.exec(&cmd).await
    }
}
