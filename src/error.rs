/// A struct to hold XML file configuration for validation purposes
#[derive(Debug, Clone, Default)]
pub struct XmlFileValidation {
    pub namespace: String,
    pub interval_ms: String,
    pub ip: String,
}

// Using thiserror to automatically implement Error and Display traits
#[derive(Debug, thiserror::Error)]
pub enum TelegrafError {
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("SSH error: {0}")]
    SshError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Duplicate node error: {0}")]
    DuplicateNodeError(String),
    #[error("Host format error: {0}")]
    HostFormatError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    #[error("OPC UA connection error: {0}")]
    OpcUaConnectionError(String),
    #[error("OPC UA timeout error: {0}")]
    OpcUaTimeoutError(String),
    #[error("OPC UA client error: {0}")]
    OpcUaClientError(String),
}

// We don't need a manual Display implementation since thiserror handles this
// through the #[error] attributes on each variant

impl TelegrafError {
    /// Converts this error into a user-friendly error message with context-specific guidance
    pub fn user_friendly_message(&self, context: &str) -> String {
        // Get the basic error message as a string first to check for patterns
        let error_str = self.to_string();

        // Check for specific error message patterns regardless of the TelegrafError variant
        if error_str.contains("Host format error")
            || error_str.contains("Invalid host format")
            || error_str.contains("Invalid port in host")
        {
            return format!(
                "⚠️ Host Format Error: {}\n\nPlease correct the IOT host field to use format: hostname:port\nExample: 192.168.0.1:22", 
                error_str
            );
        } else if error_str.contains("Failed to resolve hostname") {
            return format!(
                "⚠️ Hostname Error: {}\n\nPlease check:\n- IOT host address is correct\n- Your network can reach the host\n- DNS settings are correct (if using hostname)", 
                error_str
            );
        } else if error_str.contains("No route to host")
            || error_str.contains("Connection refused")
            || error_str.contains("Network is unreachable")
            || error_str.contains("Connection failed")
            || error_str.contains("timed out")
        {
            return format!(
                "⚠️ Connection Error: {}\n\nPlease check:\n- Host address is correct\n- Device is powered on and connected to the network\n- No firewall is blocking the connection", 
                error_str
            );
        } else if error_str.contains("Authentication") || error_str.contains("Permission denied") {
            return format!(
                "⚠️ Authentication Error: {}\n\nPlease check:\n- Username and password are correct\n- User has proper permissions", 
                error_str
            );
        } else if error_str.contains("not found") && context == "logs" {
            return format!(
                "⚠️ Log File Error: {}\n\nPlease check:\n- Telegraf is installed and has been run at least once\n- Logs are stored in the expected location", 
                error_str
            );
        }

        // If no pattern matched, proceed with variant-specific handling
        match self {
            TelegrafError::IoError(e) => {
                format!(
                    "⚠️ IO Error: {}\n\nPlease check file permissions and disk space.",
                    e
                )
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
            TelegrafError::OpcUaConnectionError(e) => {
                format!(
                    "⚠️ OPC UA Connection Error: {}\n\nPlease check:\n- OPC UA IP address is correct\n- OPC UA server is running and accessible\n- No firewall is blocking the connection", 
                    e
                )
            }
            TelegrafError::OpcUaTimeoutError(e) => {
                format!(
                    "⚠️ OPC UA Timeout: {}\n\nThe server did not respond in time. Please check:\n- OPC UA server is running properly\n- Network latency is not too high", 
                    e
                )
            }
            TelegrafError::OpcUaClientError(e) => {
                format!(
                    "⚠️ OPC UA Client Error: {}\n\nThere was a problem with the OPC UA client. Please check:\n- Username and password are correct (if authentication is required)\n- Server security settings", 
                    e
                )
            }
            TelegrafError::ConfigError(e) => {
                let additional_info = match context {
                    "generating config" => "- Check the XML file format\n- Verify namespace configuration\n- Make sure all required fields are filled",
                    "OPC UA namespace lookup" => "- Check if the OPC UA server is running\n- Verify OPC UA IP address is correct\n- Make sure XML file names match namespace names",
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
            || error_str.contains("timed out")
        {
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
        || error.contains("timed out")
    {
        TelegrafError::ConnectionError(error)
    } else if error.contains("Host format") || error.contains("Invalid host") {
        TelegrafError::HostFormatError(error)
    } else {
        TelegrafError::SshError(error)
    }
}

impl From<std::num::TryFromIntError> for TelegrafError {
    fn from(error: std::num::TryFromIntError) -> Self {
        TelegrafError::ValidationError(format!("Invalid integer conversion: {}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use ssh2::Error as Ssh2Error;
    use ssh2::ErrorCode;

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let telegraf_err = TelegrafError::from(io_err);
        assert!(matches!(telegraf_err, TelegrafError::IoError(_)));
    }

    #[test]
    fn test_ssh_auth_error() {
        let ssh_err = Ssh2Error::new(ErrorCode::Session(-16), "Authentication failed");
        let telegraf_err = TelegrafError::from(ssh_err);
        assert!(matches!(telegraf_err, TelegrafError::AuthenticationError(_)));
    }

    #[test]
    fn test_ssh_connection_error() {
        let ssh_err = Ssh2Error::new(ErrorCode::Session(-43), "Connection refused");
        let telegraf_err = TelegrafError::from(ssh_err);
        assert!(matches!(telegraf_err, TelegrafError::ConnectionError(_)));
    }

    #[test]
    fn test_try_from_int_error() {
        let result: Result<u8, _> = 1000u32.try_into();
        let err = result.unwrap_err();
        let telegraf_err = TelegrafError::from(err);
        assert!(matches!(telegraf_err, TelegrafError::ValidationError(_)));
    }

    #[test]
    fn test_user_friendly_message_connection() {
        let err = TelegrafError::ConnectionError("Connection refused".to_string());
        let msg = err.user_friendly_message("test");
        assert!(msg.starts_with("⚠️ Connection Error: Connection refused"));
        assert!(msg.contains("Please check"));
        assert!(msg.contains("IOT host address"));
        assert!(msg.contains("device is powered on"));
        assert!(msg.contains("firewall is blocking"));
    }

    #[test]
    fn test_user_friendly_message_auth() {
        let err = TelegrafError::AuthenticationError("Permission denied".to_string());
        let msg = err.user_friendly_message("ssh");
        assert!(msg.starts_with("⚠️ Authentication Error: Permission denied"));
        assert!(msg.contains("username and password are correct"));
        assert!(msg.contains("user has proper permissions"));
    }

    #[test]
    fn test_user_friendly_message_host_format() {
        let err = TelegrafError::HostFormatError("Invalid format".to_string());
        let msg = err.user_friendly_message("");
        assert!(msg.starts_with("⚠️ Host Format Error"));
        assert!(msg.contains("hostname:port"));
        assert!(msg.contains("192.168.0.1:22"));
    }

    #[test]
    fn test_user_friendly_message_config_error_generating() {
        let err = TelegrafError::ConfigError("Invalid config".to_string());
        let msg = err.user_friendly_message("generating config");
        assert!(msg.contains("XML file format"));
    }

    #[test]
    fn test_user_friendly_message_config_error_opcua() {
        let err = TelegrafError::ConfigError("Invalid config".to_string());
        let msg = err.user_friendly_message("OPC UA namespace lookup");
        assert!(msg.contains("OPC UA server"));
    }

    #[test]
    fn test_user_friendly_message_ssh_error_status() {
        let err = TelegrafError::SshError("command not found".to_string());
        let msg = err.user_friendly_message("status");
        assert!(msg.contains("Telegraf is not installed"));
    }

    #[test]
    fn test_user_friendly_message_ssh_error_logs() {
        let err = TelegrafError::SshError("not found".to_string());
        let msg = err.user_friendly_message("logs");
        assert!(msg.contains("Log File Error"));
    }

    #[test]
    fn test_user_friendly_message_ssh_error_default() {
        let err = TelegrafError::SshError("some error".to_string());
        let msg = err.user_friendly_message("test");
        assert!(msg.contains("SSH Error"));
    }
}
