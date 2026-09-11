use crate::container::UserContainer;
use crate::error::Result;
use crate::exec::ExecResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatexEngine {
    PdfLatex,
    XeLatex,
    LuaLatex,
    Latexmk,
}

impl LatexEngine {
    pub fn binary_name(&self) -> &'static str {
        match self {
            | LatexEngine::PdfLatex => "pdflatex",
            | LatexEngine::XeLatex => "xelatex",
            | LatexEngine::LuaLatex => "lualatex",
            | LatexEngine::Latexmk => "latexmk",
        }
    }
}

pub struct LatexToolchain<'a> {
    container: &'a UserContainer,
}

impl<'a> LatexToolchain<'a> {
    pub fn new(container: &'a UserContainer) -> Self {
        Self { container }
    }

    /// Check engine version
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
    pub async fn clean_aux_files(
        &self,
        output_dir: Option<&str>,
    ) -> Result<ExecResult> {
        let dir = output_dir.unwrap_or(".");
        let sh_cmd = format!(
            "rm -f {}/*.aux {}/*.log {}/*.out {}/*.toc {}/*.bbl {}/*.blg {}/*.fls {}/*.fdb_latexmk",
            dir, dir, dir, dir, dir, dir, dir, dir
        );
        self.container.exec(&["sh", "-c", &sh_cmd]).await
    }
}
