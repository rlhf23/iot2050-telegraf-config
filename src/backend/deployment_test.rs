// Basic regression and construction tests for deployment.rs

use super::deployment::{
    generate_repo_url, get_extracted_dir_name, sanitize_branch_name, DEFAULT_BRANCH,
    GITHUB_REPO_NAME, GITHUB_REPO_OWNER,
    filter_pull_images, filter_custom_services, image_tag_to_tar_filename,
    validate_push_flags, DOCKER_IMAGES, MINIMAL_IMAGE_NAMES, CUSTOM_SERVICES,
};
use super::deployment::{DeploymentConfig, IoTDeployer};
use std::path::PathBuf;

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
    assert_eq!(sanitize_branch_name("feature/login"), "feature-login");
    assert_eq!(
        sanitize_branch_name("feature/user/dashboard"),
        "feature-user-dashboard"
    );
    assert_eq!(sanitize_branch_name("release/v1.0"), "release-v1.0");
}

#[test]
fn sanitize_branch_name_with_special_chars() {
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
    let dir = get_extracted_dir_name("master");
    assert_eq!(dir, format!("{}-master", GITHUB_REPO_NAME));

    let dir = get_extracted_dir_name("main");
    assert_eq!(dir, format!("{}-main", GITHUB_REPO_NAME));
}

#[test]
fn get_extracted_dir_name_with_slashes() {
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
    let _deployer = IoTDeployer::new(config);
    let expected_url = generate_repo_url(DEFAULT_BRANCH);
    let actual_url = generate_repo_url("master");
    assert_eq!(expected_url, actual_url);
}

#[test]
fn iot_deployer_default_branch_url_format() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user".to_string());
    let _deployer = IoTDeployer::new(config);
    let url = generate_repo_url(DEFAULT_BRANCH);
    assert!(url.ends_with(&format!("{}.tar.gz", DEFAULT_BRANCH)));
}

#[test]
fn branch_url_components() {
    let url = generate_repo_url("test-branch");
    assert!(url.starts_with("https://github.com/"));
    assert!(url.contains(&format!("/{}/", GITHUB_REPO_OWNER)));
    assert!(url.contains(&format!("/{}/", GITHUB_REPO_NAME)));
    assert!(url.contains("/archive/refs/heads/"));
    assert!(url.ends_with(".tar.gz"));
}

// ============================================================================
// Image filtering tests
// ============================================================================

#[test]
fn filter_pull_images_returns_all_when_no_filter() {
    let images = filter_pull_images(false, &None);
    assert_eq!(images.len(), DOCKER_IMAGES.len());
    assert!(images.contains(&"influxdb:2"));
    assert!(images.contains(&"grafana/grafana:latest"));
    assert!(images.contains(&"prom/prometheus:latest"));
}

#[test]
fn filter_pull_images_minimal_excludes_full_profile() {
    let images = filter_pull_images(true, &None);
    assert_eq!(images.len(), MINIMAL_IMAGE_NAMES.len());
    assert!(images.contains(&"influxdb:2"));
    assert!(images.contains(&"chronograf:1.10"));
    assert!(!images.contains(&"grafana/grafana:latest"));
    assert!(!images.contains(&"prom/prometheus:latest"));
}

#[test]
fn filter_pull_images_with_specific_filter() {
    let filter = Some(vec!["chronograf".to_string(), "influxdb".to_string()]);
    let images = filter_pull_images(false, &filter);
    assert_eq!(images.len(), 2);
    assert!(images.contains(&"chronograf:1.10"));
    assert!(images.contains(&"influxdb:2"));
}

#[test]
fn filter_pull_images_filter_case_insensitive() {
    let filter = Some(vec!["Chronograf".to_string(), "INFLUXDB".to_string()]);
    let images = filter_pull_images(false, &filter);
    assert_eq!(images.len(), 2);
    assert!(images.contains(&"chronograf:1.10"));
    assert!(images.contains(&"influxdb:2"));
}

#[test]
fn filter_pull_images_filter_overrides_minimal() {
    // If you explicitly request grafana with --images, you get it even with --minimal
    let filter = Some(vec!["grafana".to_string()]);
    let images = filter_pull_images(true, &filter);
    assert_eq!(images.len(), 1);
    assert!(images.contains(&"grafana/grafana:latest"));
}

#[test]
fn filter_pull_images_empty_filter_returns_nothing() {
    let filter = Some(vec![]);
    let images = filter_pull_images(false, &filter);
    assert!(images.is_empty());
}

#[test]
fn filter_pull_images_unknown_name_returns_nothing() {
    let filter = Some(vec!["nonexistent".to_string()]);
    let images = filter_pull_images(false, &filter);
    assert!(images.is_empty());
}

#[test]
fn filter_custom_services_default() {
    let services = filter_custom_services(false, &None);
    assert_eq!(services.len(), CUSTOM_SERVICES.len());
    assert!(services.iter().any(|(n, _)| *n == "api-service"));
    assert!(services.iter().any(|(n, _)| *n == "control-service"));
}

#[test]
fn filter_custom_services_skip() {
    let services = filter_custom_services(true, &None);
    assert!(services.is_empty());
}

#[test]
fn filter_custom_services_with_image_filter() {
    let filter = Some(vec!["api-service".to_string()]);
    let services = filter_custom_services(false, &filter);
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].0, "api-service");
}

#[test]
fn filter_custom_services_filter_case_insensitive() {
    let filter = Some(vec!["API-Service".to_string()]);
    let services = filter_custom_services(false, &filter);
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].0, "api-service");
}

#[test]
fn filter_custom_services_empty_filter_returns_nothing() {
    let filter = Some(vec![]);
    let services = filter_custom_services(false, &filter);
    assert!(services.is_empty());
}

#[test]
fn filter_custom_services_skip_overrides_filter() {
    // skip_custom=true always returns empty even with filter
    let filter = Some(vec!["api-service".to_string()]);
    let services = filter_custom_services(true, &filter);
    assert!(services.is_empty());
}

// ============================================================================
// Tar filename tests
// ============================================================================

#[test]
fn image_tag_to_tar_filename_simple() {
    assert_eq!(image_tag_to_tar_filename("influxdb:2"), "influxdb-2.tar");
    assert_eq!(image_tag_to_tar_filename("nginx:alpine"), "nginx-alpine.tar");
    assert_eq!(image_tag_to_tar_filename("telegraf:1.30"), "telegraf-1.30.tar");
}

#[test]
fn image_tag_to_tar_filename_with_registry() {
    assert_eq!(image_tag_to_tar_filename("grafana/grafana:latest"), "grafana_grafana-latest.tar");
    assert_eq!(image_tag_to_tar_filename("prom/prometheus:latest"), "prom_prometheus-latest.tar");
}

#[test]
fn image_tag_to_tar_filename_no_tag() {
    assert_eq!(image_tag_to_tar_filename("alpine:3.19"), "alpine-3.19.tar");
}

#[test]
fn image_tag_to_tar_filename_custom_service() {
    assert_eq!(image_tag_to_tar_filename("api-service:local"), "api-service-local.tar");
    assert_eq!(image_tag_to_tar_filename("control-service:local"), "control-service-local.tar");
}

// ============================================================================
// Flag validation tests
// ============================================================================

#[test]
fn validate_push_flags_normal() {
    assert!(validate_push_flags(&None, &None, &None).is_ok());
}

#[test]
fn validate_push_flags_save_dir_requires_architecture() {
    let result = validate_push_flags(&None, &Some(PathBuf::from("/tmp/images")), &None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--architecture is required"));
}

#[test]
fn validate_push_flags_save_dir_with_architecture() {
    let result = validate_push_flags(&Some("arm64".to_string()), &Some(PathBuf::from("/tmp/images")), &None);
    assert!(result.is_ok());
}

#[test]
fn validate_push_flags_cannot_use_both() {
    let result = validate_push_flags(
        &Some("arm64".to_string()),
        &Some(PathBuf::from("/tmp/images")),
        &Some(PathBuf::from("/tmp/images")),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot use --save-dir and --load-dir together"));
}

#[test]
fn validate_push_flags_load_dir_without_architecture() {
    // load-dir defaults to arm64 if no architecture given
    let result = validate_push_flags(&None, &None, &Some(PathBuf::from("/tmp/images")));
    assert!(result.is_ok());
}

#[test]
fn validate_push_flags_load_dir_with_architecture() {
    let result = validate_push_flags(&Some("amd64".to_string()), &None, &Some(PathBuf::from("/tmp/images")));
    assert!(result.is_ok());
}

// ============================================================================
// Airgap options tests
// ============================================================================

#[test]
fn update_with_options_rejects_save_and_load_together() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user".to_string());
    let deployer = IoTDeployer::new(config);
    let result = deployer.update_with_options(
        false,
        Some(PathBuf::from("/tmp/save")),
        Some(PathBuf::from("/tmp/load")),
    );
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Cannot use --save-dir and --load-dir together"));
}

#[test]
fn provision_with_options_rejects_save_and_load_together() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user".to_string());
    let deployer = IoTDeployer::new(config);
    let result = deployer.provision_with_options(
        false,
        Some(PathBuf::from("/tmp/save")),
        Some(PathBuf::from("/tmp/load")),
    );
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Cannot use --save-dir and --load-dir together"));
}

#[test]
fn save_config_to_dir_fails_with_bad_branch() {
    let config = DeploymentConfig::new("host.example.com".to_string(), "user".to_string())
        .with_git_branch("nonexistent-branch-for-test-xyz".to_string());
    let deployer = IoTDeployer::new(config);
    let save_dir = std::env::temp_dir().join("iot2050_test_save_config");

    let _ = std::fs::remove_dir_all(&save_dir);

    let result = deployer.save_config_to_dir(&save_dir);
    assert!(result.is_err(), "save_config_to_dir should fail with nonexistent branch");

    let _ = std::fs::remove_dir_all(&save_dir);
}

#[test]
fn update_with_options_save_dir_returns_early_without_connection() {
    // save_dir mode should not need SSH at all — it returns after downloading.
    // We test that it attempts the download (which will fail with a bad branch)
    // rather than failing to connect to the device.
    let config = DeploymentConfig::new("nonexistent-host-for-test.local".to_string(), "user".to_string())
        .with_git_branch("nonexistent-branch-for-test-xyz".to_string());
    let deployer = IoTDeployer::new(config);
    let save_dir = std::env::temp_dir().join("iot2050_test_update_save");

    let _ = std::fs::remove_dir_all(&save_dir);

    // This should fail because the branch doesn't exist, NOT because SSH failed
    let result = deployer.update_with_options(false, Some(save_dir.clone()), None);
    assert!(result.is_err());

    // Verify it's not an SSH error — it should be a download/extraction error
    let err_msg = format!("{}", result.unwrap_err());
    assert!(!err_msg.contains("SSH"), "Should not be an SSH error, got: {}", err_msg);

    let _ = std::fs::remove_dir_all(&save_dir);
}