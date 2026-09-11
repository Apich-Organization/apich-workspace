use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

pub struct TypstToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> TypstToolchain<'a> {
    pub fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check typst version
    pub async fn typst_version(&self) -> Result<String> {
        let res = self.container.exec(&["typst", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Compile a .typ file to PDF
    pub async fn compile(
        &self,
        input_file: &str,
        output_pdf: &str,
    ) -> Result<ExecResult> {
        self.container
            .exec(&["typst", "compile", input_file, output_pdf])
            .await
    }

    /// Compile a .typ file to SVG (e.g. for browser preview)
    pub async fn compile_svg(
        &self,
        input_file: &str,
        output_svg: &str,
    ) -> Result<ExecResult> {
        self.container
            .exec(&[
                "typst", "compile", "--format", "svg", input_file, output_svg,
            ])
            .await
    }

    /// Compile a .typ file to PNG
    pub async fn compile_png(
        &self,
        input_file: &str,
        output_png: &str,
        ppi: Option<u32>,
    ) -> Result<ExecResult> {
        let ppi_str = ppi.unwrap_or(144).to_string();
        self.container
            .exec(&[
                "typst", "compile", "--format", "png", "--ppi", &ppi_str, input_file, output_png,
            ])
            .await
    }

    /// Query metadata / labels from a Typst document
    pub async fn query(
        &self,
        input_file: &str,
        selector: &str,
    ) -> Result<ExecResult> {
        self.container
            .exec(&["typst", "query", input_file, selector])
            .await
    }
}
