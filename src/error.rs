use std::path::PathBuf;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("directory walk error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("PDF extraction error: {0}")]
    Pdf(String),

    #[error("terminal error: {0}")]
    Terminal(String),

    #[error("path does not exist: {0}")]
    MissingPath(PathBuf),

    #[error("path is not a directory: {0}")]
    NotDirectory(PathBuf),

    #[error(
        "index file was created by an incompatible version: expected {expected}, found {found}"
    )]
    IncompatibleIndex { expected: u32, found: u32 },

    #[error("index contains no documents")]
    EmptyIndex,
}
