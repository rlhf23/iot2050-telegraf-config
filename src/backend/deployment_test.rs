// Basic regression and construction tests for deployment.rs

use super::deployment::{DeploymentConfig, IoTDeployer};

#[test]
fn deployment_config_builder_methods_work() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user1".to_string())
        .with_password("pass123".to_string())
        .with_key_file("/path/to/key".to_string())
        .with_port(2222);

    assert_eq!(config.host, "host.example.com");
    assert_eq!(config.user, "user1");
    assert_eq!(config.password.as_deref(), Some("pass123"));
    assert_eq!(config.key_file.as_deref(), Some("/path/to/key"));
    assert_eq!(config.port, 2222);
}

#[test]
fn iot_deployer_can_be_constructed() {
    let config = DeploymentConfig::new("localhost".to_string(), "tester".to_string());
    let deployer = IoTDeployer::new(config.clone());
    // Just check construction; config should match
    assert_eq!(deployer.host(), config.host);
    assert_eq!(deployer.user(), config.user);
}

#[test]
fn create_ssh_session_fails_without_authentication() {
    use super::deployment::IoTDeployer;
    let config =
        super::deployment::DeploymentConfig::new("localhost".to_string(), "tester".to_string());
    let deployer = IoTDeployer::new(config);
    let result = deployer.create_ssh_session();
    assert!(result.is_err());
    let err = result.err().unwrap();
    let msg = format!("{}", err);
    println!("Observed error message: {}", msg);
    assert!(
        msg.contains("No authentication method provided")
            || msg.contains("Failed to connect")
            || msg.contains("Unable to exchange encryption keys")
    );
}

#[test]
fn create_ssh_session_fails_with_missing_key_file() {
    use super::deployment::IoTDeployer;
    let config =
        super::deployment::DeploymentConfig::new("localhost".to_string(), "tester".to_string())
            .with_key_file("/nonexistent/keyfile".to_string());
    let deployer = IoTDeployer::new(config);
    let result = deployer.create_ssh_session();
    assert!(result.is_err());
    let err = result.err().unwrap();
    let msg = format!("{}", err);
    println!("Observed error message: {}", msg);
    assert!(
        msg.contains("SSH key file not found")
            || msg.contains("Failed to connect")
            || msg.contains("Unable to exchange encryption keys")
    );
}

#[test]
fn telegraf_error_ssh_operation_error_variant() {
    use crate::error::TelegrafError;
    let error = TelegrafError::SshOperationError("test error".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("test error"));
}

// TODO: For full coverage, refactor IoTDeployer to allow injecting a mock Session.
// This will enable testing of run_command, test_connection, etc., without real SSH.
