#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IgnoreProfile {
    Academic,
    Python,
    R,
    Development,
}

impl IgnoreProfile {
    /// Return the list of glob patterns associated with the profile
    pub fn patterns(&self) -> &'static [&'static str] {
        match self {
            IgnoreProfile::Academic => &[
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
            ],
            IgnoreProfile::Python => &[
                "__pycache__/**",
                "*.pyc",
                "*.pyo",
                ".pytest_cache/**",
                "*.egg-info/**",
                ".venv/**",
                "venv/**",
                "env/**",
                ".ipynb_checkpoints/**",
            ],
            IgnoreProfile::R => &[
                ".Rhistory",
                ".RData",
                ".Ruserdata",
                ".Rproj.user/**",
            ],
            IgnoreProfile::Development => &[
                "target/**",
                "node_modules/**",
                ".DS_Store",
                "Thumbs.db",
                ".apich/**",
                ".git/**",
            ],
        }
    }
}
