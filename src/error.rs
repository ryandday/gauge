// Error types for the application
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum AppError {
    #[error("Git error: {0}")]
    Git(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("AI error: {0}")]
    #[allow(dead_code)] // Used in PHASE-2
    Ai(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Terminal error: {0}")]
    Terminal(String),
}
