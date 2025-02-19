use std::fmt;

#[derive(Debug)]
pub enum TelegrafError {
    IoError(std::io::Error),
    SshError(String),
    ValidationError(String),
    ConfigError(String),
}

impl fmt::Display for TelegrafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelegrafError::IoError(e) => write!(f, "IO error: {}", e),
            TelegrafError::SshError(e) => write!(f, "SSH error: {}", e),
            TelegrafError::ValidationError(e) => write!(f, "Validation error: {}", e),
            TelegrafError::ConfigError(e) => write!(f, "Configuration error: {}", e),
        }
    }
}

impl std::error::Error for TelegrafError {}

impl From<std::io::Error> for TelegrafError {
    fn from(error: std::io::Error) -> Self {
        TelegrafError::IoError(error)
    }
}
