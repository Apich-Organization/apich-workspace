use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlideError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SVG parse error: {0}")]
    SvgParse(String),
    #[error("Compilation error: {0}")]
    Compilation(String),
    #[error("Format error: {0}")]
    Format(String),
    #[error("Chart error: {0}")]
    Chart(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Typst slide overflow error: {0}")]
    Overflow(String),
    #[error("Watcher error: {0}")]
    Watcher(String),
}

impl From<notify::Error> for SlideError {
    fn from(err: notify::Error) -> Self {
        Self::Watcher(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SlideError>;
