#[cfg(test)]
mod tests {
    use crate::error::{SshAuthError, SshConnectionError, SshError, TelegrafError};
    use ssh2::Error as Ssh2Error;
    use ssh2::ErrorCode;

    // Helper function to create a test SSH error
    fn create_ssh_error(message: &'static str) -> ssh2::Error {
        Ssh2Error::new(ErrorCode::Session(-1), message)
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let telegraf_err = TelegrafError::from(io_err);
        assert!(matches!(telegraf_err, TelegrafError::IoError(_)));
    }

    #[test]
    fn test_ssh_auth_error() {
        let ssh_err = create_ssh_error("Authentication failed");
        let telegraf_err = TelegrafError::from(ssh_err);

        if let TelegrafError::SshError(SshError::Auth(SshAuthError::InvalidCredentials(_))) =
            telegraf_err
        {
            // Expected case
        } else {
            panic!(
                "Expected SshError::Auth(InvalidCredentials), got {:?}",
                telegraf_err
            );
        }
    }

    #[test]
    fn test_ssh_permission_denied() {
        let ssh_err = create_ssh_error("Permission denied");
        let telegraf_err = TelegrafError::from(ssh_err);

        if let TelegrafError::SshError(SshError::Auth(SshAuthError::PermissionDenied(_))) =
            telegraf_err
        {
            // Expected case
        } else {
            panic!(
                "Expected SshError::Auth(PermissionDenied), got {:?}",
                telegraf_err
            );
        }
    }

    #[test]
    fn test_ssh_connection_refused() {
        let ssh_err = create_ssh_error("Connection refused");
        let telegraf_err = TelegrafError::from(ssh_err);

        if let TelegrafError::SshError(SshError::Connection(
            SshConnectionError::ConnectionRefused(_),
        )) = telegraf_err
        {
            // Expected case
        } else {
            panic!(
                "Expected SshError::Connection(ConnectionRefused), got {:?}",
                telegraf_err
            );
        }
    }

    #[test]
    fn test_ssh_host_unreachable() {
        let ssh_err = create_ssh_error("No route to host");
        let telegraf_err = TelegrafError::from(ssh_err);

        if let TelegrafError::SshError(SshError::Connection(SshConnectionError::HostUnreachable(
            _,
        ))) = telegraf_err
        {
            // Expected case
        } else {
            panic!(
                "Expected SshError::Connection(HostUnreachable), got {:?}",
                telegraf_err
            );
        }
    }

    #[test]
    fn test_ssh_timeout() {
        let ssh_err = create_ssh_error("Connection timed out");
        let telegraf_err = TelegrafError::from(ssh_err);

        if let TelegrafError::SshError(SshError::Connection(SshConnectionError::Timeout(_))) =
            telegraf_err
        {
            // Expected case
        } else {
            panic!(
                "Expected SshError::Connection(Timeout), got {:?}",
                telegraf_err
            );
        }
    }

    #[test]
    fn test_ssh_command_error() {
        let ssh_err = create_ssh_error("Command execution failed");
        let telegraf_err = TelegrafError::from(ssh_err);

        if let TelegrafError::SshError(SshError::CommandExecution(_)) = telegraf_err {
            // Expected case
        } else {
            panic!(
                "Expected SshError::CommandExecution, got {:?}",
                telegraf_err
            );
        }
    }

    #[test]
    fn test_try_from_int_error() {
        let result: Result<u8, _> = 1000u32.try_into();
        let err = result.unwrap_err();
        let telegraf_err = TelegrafError::from(err);
        assert!(matches!(telegraf_err, TelegrafError::ValidationError(_)));
    }

    // Test user-friendly messages for legacy error types
    #[test]
    fn test_user_friendly_message_legacy_connection() {
        let err = TelegrafError::ConnectionError("Connection refused".to_string());
        let msg = err.user_friendly_message("test");
        assert!(msg.starts_with("⚠️ Connection Error: Connection refused"));
        assert!(msg.contains("Please check"));
        assert!(msg.contains("IOT host address"));
        assert!(msg.contains("device is powered on"));
        assert!(msg.contains("firewall is blocking"));
    }

    #[test]
    fn test_user_friendly_message_legacy_auth() {
        let err = TelegrafError::AuthenticationError("Permission denied".to_string());
        let msg = err.user_friendly_message("ssh");
        assert!(msg.starts_with("⚠️ Authentication Error: Permission denied"));
        assert!(msg.contains("username and password are correct"));
        assert!(msg.contains("user has proper permissions"));
    }

    #[test]
    fn test_user_friendly_message_ssh_auth() {
        let ssh_err = create_ssh_error("Authentication failed");
        let err = TelegrafError::from(ssh_err);
        let msg = err.user_friendly_message("ssh");
        assert!(msg.contains("Authentication failed"));
        assert!(msg.contains("username and password"));
    }

    #[test]
    fn test_user_friendly_message_ssh_connection_refused() {
        let ssh_err = create_ssh_error("Connection refused");
        let err = TelegrafError::from(ssh_err);
        let msg = err.user_friendly_message("test");
        assert!(msg.contains("Connection refused"));
        assert!(msg.contains("SSH service is running"));
        assert!(msg.contains("port number is correct"));
        assert!(msg.contains("firewall allows SSH connections"));
    }

    #[test]
    fn test_user_friendly_message_host_format() {
        let err = TelegrafError::HostFormatError("Invalid format".to_string());
        let msg = err.user_friendly_message("");
        assert!(msg.starts_with("⚠️ Host Format Error"));
        assert!(msg.contains("hostname:port"));
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

    // Note: These tests are for the legacy error type and will need to be updated
    // or removed when the legacy error types are removed
    #[test]
    fn test_user_friendly_message_ssh_error_status() {
        let err = TelegrafError::SshError(SshError::CommandExecution(create_ssh_error(
            "command not found",
        )));
        let msg = err.user_friendly_message("status");
        assert!(msg.contains("Telegraf is not installed"));
    }

    #[test]
    fn test_user_friendly_message_ssh_error_logs() {
        let err =
            TelegrafError::SshError(SshError::CommandExecution(create_ssh_error("not found")));
        let msg = err.user_friendly_message("logs");
        assert!(msg.contains("Log File Error"));
    }

    #[test]
    fn test_user_friendly_message_ssh_error_default() {
        let err = TelegrafError::SshError(SshError::Other(create_ssh_error("some error")));
        let msg = err.user_friendly_message("test");
        assert!(msg.contains("SSH Error"));
    }
}
