use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrefixloadError {
    #[error("Error [IO]: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error [Serde YAML]: {0}")]
    SerdeYAML(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, PrefixloadError>;
