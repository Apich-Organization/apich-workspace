//! Built-in ignore profile definitions.

/// Built-in ignore profile categories for synthetic filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IgnoreProfile {
    /// Academic files: LaTeX, Typst, BibTeX aux, and generated PDFs.
    Academic,
    /// Python build and cache artifacts: `__pycache__`, `.venv`, `.pytest_cache`.
    Python,
    /// R workspace and history artifacts: `.Rhistory`, `.RData`.
    R,
    /// General development artifacts: logs, environment files, swap files.
    Development,
    /// Rust build artifacts: `target/`.
    Rust,
    /// JavaScript and Node.js dependencies: `node_modules/`, `dist/`.
    JavaScript,
    /// Java build outputs: `*.class`, `target/`.
    Java,
    /// C and C++ compiled binaries, object files, and build directories.
    CCpp,
    /// Editor and IDE configuration directories (`.vscode`, `.idea`, swap files).
    Editor,
}

impl IgnoreProfile {
    /// All known profiles, in the order they should be shown to a user
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Development,
            Self::Rust,
            Self::JavaScript,
            Self::Python,
            Self::R,
            Self::Java,
            Self::CCpp,
            Self::Academic,
            Self::Editor,
        ]
    }

    /// Machine-readable id, stable across releases (used for config wiring / URLs)
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            | Self::Academic => "academic",
            | Self::Python => "python",
            | Self::R => "r",
            | Self::Development => "development",
            | Self::Rust => "rust",
            | Self::JavaScript => "javascript",
            | Self::Java => "java",
            | Self::CCpp => "ccpp",
            | Self::Editor => "editor",
        }
    }

    /// Short human label for UI display
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            | Self::Academic => "Academic (LaTeX/Typst)",
            | Self::Python => "Python",
            | Self::R => "R",
            | Self::Development => "General dev",
            | Self::Rust => "Rust / Cargo",
            | Self::JavaScript => "JavaScript / Node",
            | Self::Java => "Java / JVM",
            | Self::CCpp => "C / C++",
            | Self::Editor => "Editors & OS junk",
        }
    }

    /// Return the list of glob patterns associated with the profile
    #[must_use]
    pub const fn patterns(&self) -> &'static [&'static str] {
        match self {
            | Self::Academic => {
                &[
                    "*.aux",
                    "*.fls",
                    "*.fdb_latexmk",
                    "*.synctex.gz",
                    "*.bbl",
                    "*.blg",
                    "*.toc",
                    "*.out",
                    "*.log",
                    "*.nav",
                    "*.snm",
                    "*.vrb",
                    "*.typst-cache",
                    ".typst-cache/**",
                    // biber / biblatex
                    "*.bcf",
                    "*.run.xml",
                    // makeindex
                    "*.idx",
                    "*.ilg",
                    "*.ind",
                    // glossaries package
                    "*.acn",
                    "*.acr",
                    "*.alg",
                    "*.glg",
                    "*.glo",
                    "*.gls",
                    "*.glsdefs",
                    "*.ist",
                    // minted package (pygmentize cache)
                    "_minted-*/**",
                    // beamer / misc
                    "*.thm",
                    "*.dvi",
                    "*.xdv",
                ]
            },
            | Self::Python => {
                &[
                    "__pycache__/**",
                    "*.pyc",
                    "*.pyo",
                    "*.pyd",
                    ".pytest_cache/**",
                    "*.egg-info/**",
                    ".venv/**",
                    "venv/**",
                    "env/**",
                    ".ipynb_checkpoints/**",
                    ".mypy_cache/**",
                    ".ruff_cache/**",
                    ".pytype/**",
                    "dist/**",
                    "build/**",
                    ".tox/**",
                    ".nox/**",
                    "htmlcov/**",
                    ".coverage",
                    ".coverage.*",
                    "*.cover",
                    "pip-wheel-metadata/**",
                    ".pdm-python",
                    ".hypothesis/**",
                ]
            },
            | Self::R => {
                &[
                    ".Rhistory",
                    ".RData",
                    ".Ruserdata",
                    ".Rproj.user/**",
                    "renv/library/**",
                    "renv/staging/**",
                    "renv/python/**",
                    "renv/sandbox/**",
                    "*.Rout",
                    "*.Rout.save",
                    "*.Rout.fail",
                    ".httr-oauth",
                ]
            },
            | Self::Development => {
                &[
                    "target/**",
                    "node_modules/**",
                    ".DS_Store",
                    "Thumbs.db",
                    ".apich/**",
                    ".apich_notebook_tmp/**",
                    ".git/**",
                    "*.log",
                    ".env",
                    ".env.local",
                    "dist/**",
                    "build/**",
                    "out/**",
                    "tmp/**",
                    ".cache/**",
                ]
            },
            | Self::Rust => {
                &[
                    "target/**",
                    "Cargo.lock.bak",
                    "**/*.rs.bk",
                    "*.pdb",
                    ".cargo/registry/**",
                    ".cargo/git/**",
                ]
            },
            | Self::JavaScript => {
                &[
                    "node_modules/**",
                    "dist/**",
                    "build/**",
                    ".next/**",
                    ".nuxt/**",
                    ".svelte-kit/**",
                    ".turbo/**",
                    ".parcel-cache/**",
                    ".vite/**",
                    "coverage/**",
                    "*.tsbuildinfo",
                    "npm-debug.log*",
                    "yarn-debug.log*",
                    "yarn-error.log*",
                    "pnpm-debug.log*",
                    ".pnpm-store/**",
                    ".yarn/cache/**",
                    ".yarn/install-state.gz",
                ]
            },
            | Self::Java => {
                &[
                    "*.class",
                    "target/**",
                    "build/**",
                    ".gradle/**",
                    "*.jar",
                    "*.war",
                    "*.ear",
                    ".mvn/wrapper/maven-wrapper.jar",
                    "hs_err_pid*.log",
                ]
            },
            | Self::CCpp => {
                &[
                    "*.o",
                    "*.obj",
                    "*.a",
                    "*.lib",
                    "*.so",
                    "*.so.*",
                    "*.dylib",
                    "*.dll",
                    "*.exe",
                    "*.out",
                    "*.gch",
                    "*.pch",
                    "CMakeFiles/**",
                    "CMakeCache.txt",
                    "cmake-build-*/**",
                    "compile_commands.json",
                    ".ccls-cache/**",
                ]
            },
            | Self::Editor => {
                &[
                    ".vscode/**",
                    ".idea/**",
                    "*.swp",
                    "*.swo",
                    "*~",
                    ".*.sw?",
                    ".vim/**",
                    "*.sublime-workspace",
                    "*.sublime-project",
                    ".fleet/**",
                    ".zed/**",
                    ".DS_Store",
                    "Desktop.ini",
                ]
            },
        }
    }
}
