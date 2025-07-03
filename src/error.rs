use std::{error, fmt, io, result};

#[derive(Debug)]
pub enum PrefixloadError {
    Io(io::Error),
}

impl fmt::Display for PrefixloadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrefixloadError::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl error::Error for PrefixloadError {}

impl From<io::Error> for PrefixloadError {
    fn from(err: io::Error) -> Self {
        PrefixloadError::Io(err)
    }
}

pub type Result<T> = result::Result<T, PrefixloadError>;
