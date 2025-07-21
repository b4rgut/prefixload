use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrefixloadError {
    #[error("Error [IO]: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error [Serde YAML]: {0}")]
    SerdeYAML(#[from] serde_yaml::Error),

    #[error("Error [Syntect]: {0}")]
    Syntect(#[from] syntect::Error),

    #[error("Error [AWS SDK S3]: {0}")]
    AWS(#[from] aws_sdk_s3::Error),

    #[error("Error [Requestty]: {0}")]
    Requestty(#[from] requestty::ErrorKind),
}

pub type Result<T> = std::result::Result<T, PrefixloadError>;
