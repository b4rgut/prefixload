use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrefixloadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Input error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    #[error("Serde YAML error: {0}")]
    SerdeYAML(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, PrefixloadError>;
