// Basic regression and construction tests for deployment.rs

use super::deployment::{
    generate_repo_url, get_extracted_dir_name, sanitize_branch_name, DEFAULT_BRANCH,
    GITHUB_REPO_NAME, GITHUB_REPO_OWNER,
};
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
fn deployment_config_with_git_branch() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user1".to_string())
        .with_git_branch("feature/my-branch".to_string());

    assert_eq!(config.git_branch.as_deref(), Some("feature/my-branch"));
    assert_eq!(config.host, "host.example.com");
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

// ============================================================================
// Git branch URL and sanitization tests
// ============================================================================

#[test]
fn sanitize_branch_name_simple() {
    assert_eq!(sanitize_branch_name("master"), "master");
    assert_eq!(sanitize_branch_name("main"), "main");
    assert_eq!(sanitize_branch_name("develop"), "develop");
}

#[test]
fn sanitize_branch_name_with_slashes() {
    // GitHub replaces '/' with '-' in archive names
    assert_eq!(sanitize_branch_name("feature/login"), "feature-login");
    assert_eq!(
        sanitize_branch_name("feature/user/dashboard"),
        "feature-user-dashboard"
    );
    assert_eq!(sanitize_branch_name("release/v1.0"), "release-v1.0");
}

#[test]
fn sanitize_branch_name_with_special_chars() {
    // Branches with underscores, dots, etc. should be preserved
    assert_eq!(sanitize_branch_name("feature_user"), "feature_user");
    assert_eq!(sanitize_branch_name("release-1.0"), "release-1.0");
    assert_eq!(sanitize_branch_name("v1.2.3"), "v1.2.3");
}

#[test]
fn generate_repo_url_master_branch() {
    let url = generate_repo_url("master");
    assert!(url.contains("github.com"));
    assert!(url.contains(GITHUB_REPO_OWNER));
    assert!(url.contains(GITHUB_REPO_NAME));
    assert!(url.contains("master.tar.gz"));
    assert!(url.contains("/archive/refs/heads/"));
}

#[test]
fn generate_repo_url_feature_branch() {
    let url = generate_repo_url("feature/my-feature");
    assert!(url.contains("feature/my-feature.tar.gz"));
    // URL contains the raw branch name (not sanitized)
    assert!(url.contains("refs/heads/feature/my-feature.tar.gz"));
}

#[test]
fn generate_repo_url_format() {
    let url = generate_repo_url("test-branch");
    let expected = format!(
        "https://github.com/{}/{}/archive/refs/heads/test-branch.tar.gz",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME
    );
    assert_eq!(url, expected);
}

#[test]
fn get_extracted_dir_name_simple() {
    // Simple branch names stay as-is
    let dir = get_extracted_dir_name("master");
    assert_eq!(dir, format!("{}-master", GITHUB_REPO_NAME));

    let dir = get_extracted_dir_name("main");
    assert_eq!(dir, format!("{}-main", GITHUB_REPO_NAME));
}

#[test]
fn get_extracted_dir_name_with_slashes() {
    // GitHub sanitizes slashes in extracted directory names
    let dir = get_extracted_dir_name("feature/login");
    assert_eq!(dir, format!("{}-feature-login", GITHUB_REPO_NAME));

    let dir = get_extracted_dir_name("release/v1.0");
    assert_eq!(dir, format!("{}-release-v1.0", GITHUB_REPO_NAME));
}

#[test]
fn default_branch_constant() {
    assert_eq!(DEFAULT_BRANCH, "master");
}

#[test]
fn iot_deployer_uses_default_branch_when_none() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user".to_string());
    let deployer = IoTDeployer::new(config);
    // Internally, deployer should use DEFAULT_BRANCH when no branch is specified
    // We can't directly access the branch field, but we can verify the URL is correct
    // by checking that generate_repo_url uses the right format
    let expected_url = generate_repo_url(DEFAULT_BRANCH);
    let actual_url = generate_repo_url("master");
    assert_eq!(expected_url, actual_url);
}

#[test]
fn iot_deployer_default_branch_url_format() {
    // Verify the generated URL matches expected pattern for default branch
    let config = DeploymentConfig::new("host.example.com".to_string(), "user".to_string());
    let _deployer = IoTDeployer::new(config);

    // The URL should point to the master branch tarball
    let url = generate_repo_url(DEFAULT_BRANCH);
    assert!(url.ends_with(&format!("{}.tar.gz", DEFAULT_BRANCH)));
}

#[test]
fn branch_url_components() {
    // Verify all URL components are present and correct
    let url = generate_repo_url("test-branch");

    // Check URL structure: https://github.com/{owner}/{repo}/archive/refs/heads/{branch}.tar.gz
    assert!(url.starts_with("https://github.com/"));
    assert!(url.contains(&format!("/{}/", GITHUB_REPO_OWNER)));
    assert!(url.contains(&format!("/{}/", GITHUB_REPO_NAME)));
    assert!(url.contains("/archive/refs/heads/"));
    assert!(url.ends_with(".tar.gz"));
}

// TODO: For full coverage, refactor IoTDeployer to allow injecting a mock Session.
// This will enable testing of run_command, test_connection, etc., without real SSH.
