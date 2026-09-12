//! LaTeX compilation engines and toolchain.

use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

/// LaTeX compilation engines supported inside the sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatexEngine {
    /// Traditional pdfLaTeX engine.
    PdfLatex,
    /// `XeLaTeX` engine with modern font and Unicode support.
    XeLatex,
    /// `LuaLaTeX` engine with Lua scripting capabilities.
    LuaLatex,
    /// Latexmk automated build tool.
    Latexmk,
}

impl LatexEngine {
    /// Returns the executable CLI binary name for the engine.
    #[must_use]
    pub const fn binary_name(&self) -> &'static str {
        match self {
            | Self::PdfLatex => "pdflatex",
            | Self::XeLatex => "xelatex",
            | Self::LuaLatex => "lualatex",
            | Self::Latexmk => "latexmk",
        }
    }
}

/// Helper for compiling LaTeX documents inside a container.
pub struct LatexToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> LatexToolchain<'a> {
    /// Creates a new `LatexToolchain` instance.
    #[must_use]
    pub const fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check engine version
    ///
    /// # Errors
    /// Returns an error if querying the engine version fails.
    pub async fn engine_version(
        &self,
        engine: LatexEngine,
    ) -> Result<String> {
        let bin = engine.binary_name();
        let res = self.container.exec(&[bin, "--version"]).await?;
        res.ensure_success(self.container.container_name())?;
        let first_line = res.stdout_lossy().lines().next().unwrap_or("").to_string();
        Ok(first_line)
    }

    /// Compile a .tex file with the specified engine
    ///
    /// # Errors
    /// Returns an error if compiling the LaTeX document fails.
    pub async fn compile(
        &self,
        tex_file: &str,
        engine: LatexEngine,
        output_dir: Option<&str>,
    ) -> Result<ExecResult> {
        let mut cmd = Vec::new();
        match engine {
            | LatexEngine::Latexmk => {
                cmd.extend(&["latexmk", "-pdf", "-interaction=nonstopmode"]);
                if let Some(out) = output_dir {
                    cmd.push("-outdir");
                    cmd.push(out);
                }
                cmd.push(tex_file);
            },
            | engine => {
                let bin = engine.binary_name();
                cmd.extend(&[bin, "-interaction=nonstopmode"]);
                if let Some(out) = output_dir {
                    cmd.push("-output-directory");
                    cmd.push(out);
                }
                cmd.push(tex_file);
            },
        }

        self.container.exec(&cmd).await
    }

    /// Clean auxiliary LaTeX files (*.aux, *.log, *.out, etc.)
    ///
    /// # Errors
    /// Returns an error if removing auxiliary files fails.
    pub async fn clean_aux_files(
        &self,
        output_dir: Option<&str>,
    ) -> Result<ExecResult> {
        let dir = output_dir.unwrap_or(".");
        let sh_cmd = format!(
            "rm -f {dir}/*.aux {dir}/*.log {dir}/*.out {dir}/*.toc {dir}/*.bbl {dir}/*.blg {dir}/*.fls {dir}/*.fdb_latexmk"
        );
        self.container.exec(&["sh", "-c", &sh_cmd]).await
    }
}
