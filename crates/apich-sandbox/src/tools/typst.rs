//! Typst typesetting compiler toolchain.

use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

/// Helper for compiling Typst documents inside a container.
pub struct TypstToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> TypstToolchain<'a> {
    /// Creates a new `TypstToolchain` instance.
    #[must_use]
    pub const fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check typst version
    ///
    /// # Errors
    /// Returns an error if executing typst --version fails.
    pub async fn typst_version(&self) -> Result<String> {
        let res = self.container.exec(&["typst", "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        Ok(res.stdout_lossy().trim().to_string())
    }

    /// Compile a .typ file to PDF
    ///
    /// # Errors
    /// Returns an error if compiling Typst document to PDF fails.
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
    ///
    /// # Errors
    /// Returns an error if compiling Typst document to SVG fails.
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
    ///
    /// # Errors
    /// Returns an error if compiling Typst document to PNG fails.
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
    ///
    /// # Errors
    /// Returns an error if executing typst query fails.
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
