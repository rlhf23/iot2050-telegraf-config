use std::fmt;

#[derive(Debug)]
pub enum TelegrafError {
    IoError(std::io::Error),
    SshError(String),
    ValidationError(String),
    ConfigError(String),
    DuplicateNodeError(String), // Add this variant
}

impl fmt::Display for TelegrafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelegrafError::IoError(e) => writeln!(f, "IO error:\n    {}", e),
            TelegrafError::SshError(e) => writeln!(f, "SSH error:\n    {}", e),
            TelegrafError::ValidationError(e) => writeln!(f, "Validation error:\n    {}", e),
            TelegrafError::ConfigError(e) => writeln!(f, "Configuration error:\n    {}", e),
            TelegrafError::DuplicateNodeError(e) => writeln!(f, "Duplicate node error:\n    {}", e),
        }
    }
}

impl std::error::Error for TelegrafError {}

impl From<std::io::Error> for TelegrafError {
    fn from(error: std::io::Error) -> Self {
        TelegrafError::IoError(error)
    }
}
