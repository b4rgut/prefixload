use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrefixloadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Input error: {0}")]
    Dialoguer(#[from] dialoguer::Error),
}

pub type Result<T> = std::result::Result<T, PrefixloadError>;
