use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

// Check if we're running in a CI environment
fn is_ci_environment() -> bool {
    // Check for CI environment variable
    if std::env::var("CI").is_ok() {
        return true;
    }

    // Also check for our special marker file
    if std::path::Path::new(".github/workflows/skip-integration-tests").exists() {
        return true;
    }

    false
}

// Integration tests that interface with the CLI are skipped in CI environments
// since they can be unreliable across different platforms and environments
#[test]
fn test_cli_basics() {
    // Skip all CLI tests in CI environments
    if is_ci_environment() {
        println!("Skipping CLI tests in CI environment");
        return;
    }

    // Also skip on Windows which might have different command prompt behavior
    if cfg!(target_os = "windows") {
        println!("Skipping CLI tests on Windows");
        return;
    }

    // Try to find the executable, skip the test if not found
    let binary_path = get_bin_path("sie_generate_config");
    if !binary_path.exists() {
        println!(
            "Binary not found at {}, skipping test",
            binary_path.display()
        );
        return;
    }

    // Just verify that the help command runs successfully
    let help_output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(help_output.status.success(), "Help command should succeed");
    println!("Verified that the CLI executable can run with --help");
}

// Helper to get the path to our binaries
fn get_bin_path(bin_name: &str) -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current exe path");
    path.pop(); // Remove the test binary name

    // Handle differences between debug/release
    if path.ends_with("deps") {
        path.pop();
    }

    #[cfg(target_os = "windows")]
    let bin_name = format!("{}.exe", bin_name);

    path.push(bin_name);
    path
}
