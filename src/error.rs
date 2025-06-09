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
    IoError(#[from] std::io::Error),

    // SSH-related errors
    #[error("SSH error: {0}")]
    SshError(#[from] SshError),

    #[error("SSH operation error: {0}")]
    SshOperationError(String),


    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Duplicate node error: {0}")]
    DuplicateNodeError(String),

    #[error("Host format error: {0}")]
    HostFormatError(String),

    // Legacy error variants (kept for backward compatibility)
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    // OPC UA related errors
    #[error("OPC UA connection error: {0}")]
    OpcUaConnectionError(String),

    #[error("OPC UA timeout error: {0}")]
    OpcUaTimeoutError(String),

    #[error("OPC UA client error: {0}")]
    OpcUaClientError(String),
}

/// SSH-specific errors
#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("Authentication failed: {0}")]
    Auth(#[from] SshAuthError),

    #[error("Connection failed: {0}")]
    Connection(#[from] SshConnectionError),

    #[error("Command execution failed: {0}")]
    CommandExecution(#[source] ssh2::Error),

    #[error("Other SSH error: {0}")]
    Other(#[source] ssh2::Error),
}

/// SSH authentication errors
#[derive(Debug, thiserror::Error)]
pub enum SshAuthError {
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(#[source] ssh2::Error),

    #[error("Unsupported authentication method: {0}")]
    UnsupportedMethod(#[source] ssh2::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(#[source] ssh2::Error),
}

/// SSH connection errors
#[derive(Debug, thiserror::Error)]
pub enum SshConnectionError {
    #[error("Connection timeout: {0}")]
    Timeout(#[source] ssh2::Error),

    #[error("Host unreachable: {0}")]
    HostUnreachable(#[source] ssh2::Error),

    #[error("Connection refused: {0}")]
    ConnectionRefused(#[source] ssh2::Error),

    #[error("Network unreachable: {0}")]
    NetworkUnreachable(#[source] ssh2::Error),
}

impl TelegrafError {
    /// Converts this error into a user-friendly error message with context-specific guidance
    pub fn user_friendly_message(&self, context: &str) -> String {
        match self {
            TelegrafError::IoError(e) => format!("⚠️ I/O Error: {}", e),
            
            TelegrafError::SshError(ssh_err) => match ssh_err {
                SshError::Auth(auth_err) => match auth_err {
                    SshAuthError::InvalidCredentials(e) => {
                        let mut msg = format!("⚠️ Authentication Error: {}\n\nPlease verify that:", e);
                        if context == "ssh" {
                            msg.push_str("\n- The username and password are correct");
                            msg.push_str("\n- The user has proper permissions on the remote system");
                        }
                        msg
                    },
                    SshAuthError::PermissionDenied(e) => {
                        format!("⚠️ Permission Denied: {}\n\nPlease check that the user has the necessary permissions on the remote system.", e)
                    },
                    SshAuthError::UnsupportedMethod(e) => {
                        format!("⚠️ Unsupported Authentication Method: {}\n\nTry using a different authentication method.", e)
                    },
                },
                SshError::Connection(conn_err) => match conn_err {
                    SshConnectionError::Timeout(e) => {
                        format!("⚠️ Connection Timeout: {}\n\nPlease check:\n- The host is reachable\n- The port is correct\n- No firewall is blocking the connection", e)
                    },
                    SshConnectionError::HostUnreachable(e) => {
                        format!("⚠️ Host Unreachable: {}\n\nPlease check:\n- The host address is correct\n- The device is powered on and connected to the network", e)
                    },
                    SshConnectionError::ConnectionRefused(e) => {
                        format!("⚠️ Connection Refused: {}\n\nPlease check:\n- The SSH service is running on the remote host\n- The port number is correct\n- The firewall allows SSH connections (port 22)", e)
                    },
                    SshConnectionError::NetworkUnreachable(e) => {
                        format!("⚠️ Network Unreachable: {}\n\nPlease check your network connection and try again.", e)
                    },
                },
                SshError::CommandExecution(e) => {
                    if context == "status" {
                        "⚠️ Telegraf is not installed or not running. Please install Telegraf and start the service.".to_string()
                    } else if context == "logs" {
                        "⚠️ Log File Error: Could not read Telegraf logs. Make sure Telegraf is installed and the log file exists.".to_string()
                    } else {
                        format!("⚠️ Command Execution Error: {}", e)
                    }
                },
                SshError::Other(e) => format!("⚠️ SSH Error: {}", e),
            },
            
            TelegrafError::ValidationError(e) => format!("⚠️ Validation Error: {}", e),
            
            TelegrafError::ConfigError(e) => match context {
                "generating config" => format!("⚠️ Configuration Error: {}\n\nPlease check the XML file format and ensure all required fields are present.", e),
                "OPC UA namespace lookup" => format!("⚠️ OPC UA Configuration Error: {}\n\nPlease verify the OPC UA server is running and the namespace index is correct.", e),
                _ => format!("⚠️ Configuration Error: {}", e),
            },
            
            TelegrafError::DuplicateNodeError(e) => format!("⚠️ Duplicate Node Error: {}", e),
            
            TelegrafError::HostFormatError(_) => "⚠️ Host Format Error\n\nPlease provide the host in the format 'hostname:port' (e.g., '192.168.0.1:22')".to_string(),
            
            // Legacy error types (kept for backward compatibility)
            TelegrafError::ConnectionError(e) => format!("⚠️ Connection Error: {}\n\nPlease check the following:\n- The IOT host address is correct\n- The device is powered on and connected to the network\n- No firewall is blocking the connection", e),
            
            TelegrafError::AuthenticationError(e) => {
                let mut msg = format!("⚠️ Authentication Error: {}\n\nPlease verify that:", e);
                if context == "ssh" {
                    msg.push_str("\n- The username and password are correct");
                    msg.push_str("\n- The user has proper permissions on the remote system");
                }
                msg
            },
            
            TelegrafError::OpcUaConnectionError(e) => format!("⚠️ OPC UA Connection Error: {}\n\nPlease check the OPC UA server is running and accessible.", e),
            
            TelegrafError::OpcUaTimeoutError(e) => format!("⚠️ OPC UA Timeout Error: {}\n\nThe OPC UA server did not respond in time. Please check the server status and network connectivity.", e),
            
            TelegrafError::OpcUaClientError(e) => format!("⚠️ OPC UA Client Error: {}", e),
            
            TelegrafError::SshOperationError(e) => format!("⚠️ SSH Operation Error: {}", e),
        }
    }
}

impl From<ssh2::Error> for TelegrafError {
    fn from(error: ssh2::Error) -> Self {
        TelegrafError::SshError(SshError::from(error))
    }
}

impl From<ssh2::Error> for SshError {
    fn from(error: ssh2::Error) -> Self {
        let error_str = error.to_string();

        if error_str.contains("Authentication") {
            SshError::Auth(SshAuthError::InvalidCredentials(error))
        } else if error_str.contains("Permission denied") {
            SshError::Auth(SshAuthError::PermissionDenied(error))
        } else if error_str.contains("No route to host")
            || error_str.contains("Network is unreachable")
        {
            SshError::Connection(SshConnectionError::HostUnreachable(error))
        } else if error_str.contains("Connection refused") {
            SshError::Connection(SshConnectionError::ConnectionRefused(error))
        } else if error_str.contains("timed out") {
            SshError::Connection(SshConnectionError::Timeout(error))
        } else if error_str.contains("Connection failed") {
            SshError::Connection(SshConnectionError::NetworkUnreachable(error))
        } else {
            SshError::CommandExecution(error)
        }
    }
}

// This is specifically for SSH command execution errors
pub fn ssh_error_from_string(error: String) -> TelegrafError {
    // Create a static string for the error message to avoid lifetime issues
    let error_msg = format!("SSH error: {}", error);
    let error_msg = Box::leak(error_msg.into_boxed_str());
    let ssh_err = ssh2::Error::new(ssh2::ErrorCode::Session(-1), error_msg);

    if error.contains("Authentication") || error.contains("Permission denied") {
        TelegrafError::SshError(SshError::Auth(SshAuthError::InvalidCredentials(ssh_err)))
    } else if error.contains("No route to host") || error.contains("Network is unreachable") {
        TelegrafError::SshError(SshError::Connection(SshConnectionError::HostUnreachable(
            ssh_err,
        )))
    } else if error.contains("Connection refused") {
        TelegrafError::SshError(SshError::Connection(SshConnectionError::ConnectionRefused(
            ssh_err,
        )))
    } else if error.contains("timed out") || error.contains("Connection failed") {
        TelegrafError::SshError(SshError::Connection(SshConnectionError::Timeout(ssh_err)))
    } else if error.contains("Host format") || error.contains("Invalid host") {
        TelegrafError::HostFormatError(error)
    } else {
        TelegrafError::SshError(SshError::Other(ssh_err))
    }
}

impl From<std::num::TryFromIntError> for TelegrafError {
    fn from(error: std::num::TryFromIntError) -> Self {
        TelegrafError::ValidationError(format!("Invalid integer conversion: {}", error))
    }
}
