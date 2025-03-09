use std::fmt;

#[derive(Debug)]
pub enum TelegrafError {
    IoError(std::io::Error),
    SshError(String),
    ValidationError(String),
    ConfigError(String),
    DuplicateNodeError(String),
    HostFormatError(String),
    ConnectionError(String),
    AuthenticationError(String),
}

impl fmt::Display for TelegrafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelegrafError::IoError(e) => writeln!(f, "IO error:\n    {}", e),
            TelegrafError::SshError(e) => writeln!(f, "SSH error:\n    {}", e),
            TelegrafError::ValidationError(e) => writeln!(f, "Validation error:\n    {}", e),
            TelegrafError::ConfigError(e) => writeln!(f, "Configuration error:\n    {}", e),
            TelegrafError::DuplicateNodeError(e) => writeln!(f, "Duplicate node error:\n    {}", e),
            TelegrafError::HostFormatError(e) => writeln!(f, "Host format error:\n    {}", e),
            TelegrafError::ConnectionError(e) => writeln!(f, "Connection error:\n    {}", e),
            TelegrafError::AuthenticationError(e) => writeln!(f, "Authentication error:\n    {}", e),
        }
    }
}

impl TelegrafError {
    /// Converts this error into a user-friendly error message with context-specific guidance
    pub fn user_friendly_message(&self, context: &str) -> String {
        match self {
            TelegrafError::IoError(e) => {
                format!("⚠️ IO Error: {}\n\nPlease check file permissions and disk space.", e)
            }
            TelegrafError::HostFormatError(e) => {
                format!(
                    "⚠️ Host Format Error: {}\n\nPlease correct the IOT host field to use format: hostname:port\nExample: 192.168.0.1:22", 
                    e
                )
            }
            TelegrafError::ConnectionError(e) => {
                format!(
                    "⚠️ Connection Error: {}\n\nPlease check:\n- IOT host address is correct\n- IOT device is powered on and connected to the network\n- No firewall is blocking the connection", 
                    e
                )
            }
            TelegrafError::AuthenticationError(e) => {
                format!(
                    "⚠️ Authentication Error: {}\n\nPlease check:\n- SSH username and password are correct\n- SSH user has proper permissions", 
                    e
                )
            }
            TelegrafError::ConfigError(e) => {
                let additional_info = match context {
                    "generating config" => "- Check the XML file format\n- Verify namespace configuration\n- Make sure all required fields are filled",
                    _ => "- Check configuration parameters\n- Verify file paths exist"
                };
                
                format!(
                    "⚠️ Configuration Error: {}\n\nPossible issues:\n{}", 
                    e, additional_info
                )
            }
            TelegrafError::ValidationError(e) => {
                format!(
                    "⚠️ Validation Error: {}\n\nPlease check your input values.", 
                    e
                )
            }
            TelegrafError::SshError(e) => {
                if e.contains("not found") && context == "logs" {
                    format!(
                        "⚠️ Log File Error: {}\n\nPlease check:\n- Telegraf is installed and has been run at least once\n- Logs are stored in the expected location", 
                        e
                    )
                } else {
                    let additional_info = if context == "status" {
                        "- Telegraf is not installed\n- SSH user doesn't have sudo permissions\n- Telegraf service is not running"
                    } else {
                        "- Verify SSH connection parameters\n- Check if the IOT device is reachable\n- Ensure SSH service is running on the IOT device"
                    };

                    format!(
                        "⚠️ SSH Error during {}: {}\n\nPossible issues:\n{}",
                        context, e, additional_info
                    )
                }
            }
            TelegrafError::DuplicateNodeError(e) => {
                format!(
                    "⚠️ Duplicate Node Error: {}\n\nPlease check your XML configuration for duplicate node identifiers.", 
                    e
                )
            }
        }
    }
}

impl std::error::Error for TelegrafError {}

impl From<std::io::Error> for TelegrafError {
    fn from(error: std::io::Error) -> Self {
        TelegrafError::IoError(error)
    }
}

impl From<ssh2::Error> for TelegrafError {
    fn from(error: ssh2::Error) -> Self {
        // Categorize SSH errors based on their content
        let error_str = error.to_string();
        
        if error_str.contains("Authentication") || error_str.contains("Permission denied") {
            TelegrafError::AuthenticationError(error_str)
        } else if error_str.contains("No route to host") 
            || error_str.contains("Connection refused") 
            || error_str.contains("Network is unreachable")
            || error_str.contains("Connection failed")
            || error_str.contains("timed out") {
            TelegrafError::ConnectionError(error_str)
        } else {
            TelegrafError::SshError(error_str)
        }
    }
}

// We should not implement a generic From<String> or From<&str> for TelegrafError
// as it would cause ambiguity in error categorization.
// Instead, specific conversion functions should be used for different error types.

// This is specifically for SSH command execution errors
pub fn ssh_error_from_string(error: String) -> TelegrafError {
    if error.contains("Authentication") || error.contains("Permission denied") {
        TelegrafError::AuthenticationError(error)
    } else if error.contains("No route to host") 
        || error.contains("Connection refused") 
        || error.contains("Network is unreachable")
        || error.contains("Connection failed")
        || error.contains("timed out") {
        TelegrafError::ConnectionError(error)
    } else if error.contains("Host format") || error.contains("Invalid host") {
        TelegrafError::HostFormatError(error)
    } else {
        TelegrafError::SshError(error)
    }
}

impl From<std::num::TryFromIntError> for TelegrafError {
    fn from(error: std::num::TryFromIntError) -> Self {
        TelegrafError::SshError(error.to_string())
    }
}
