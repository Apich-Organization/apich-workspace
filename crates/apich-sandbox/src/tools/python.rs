use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

pub struct PythonToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> PythonToolchain<'a> {
    pub fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check python version
    pub async fn python_version(&self) -> Result<String> {
        let res = self.container.exec(&["python3", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Execute inline Python code string
    pub async fn run_code(&self, code: &str) -> Result<ExecResult> {
        self.container.exec(&["python3", "-c", code]).await
    }

    /// Run a python script file with arguments
    pub async fn run_file(&self, script_path: &str, args: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["python3", script_path];
        cmd.extend(args);
        self.container.exec(&cmd).await
    }

    /// Install packages using pip
    pub async fn pip_install(&self, packages: &[&str]) -> Result<ExecResult> {
        let mut cmd = vec!["pip", "install", "--break-system-packages"];
        cmd.extend(packages);
        self.container.exec(&cmd).await
    }

    /// List installed pip packages
    pub async fn pip_list(&self) -> Result<ExecResult> {
        self.container.exec(&["pip", "list"]).await
    }
}
