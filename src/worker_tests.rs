use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use crate::backend::OutputFormat;
use crate::worker::{WorkerCommand, WorkerHandle, WorkerResponse};
use crate::TelegrafConfig;
use std::path::Path;

// Helper function to wait for a response with a timeout
fn wait_for_response<F>(worker: &WorkerHandle, timeout: Duration, check: F) -> bool
where
    F: Fn(&WorkerResponse) -> bool,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(response) = worker.try_get_response() {
            if check(&response) {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

// Helper function to create a test config
fn create_test_config() -> TelegrafConfig {
    TelegrafConfig {
        folder: std::env::current_dir().unwrap(),
        ip: "127.0.0.1:49320".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        iot_host: "192.168.1.2:22".to_string(),
        iot_username: "iot".to_string(),
        iot_password: "iotpass".to_string(),
        listener_files: vec![],
        output_format: Some("influxdb".to_string()),
        include_test_inputs: false,
        include_opcua_diagnostics: false,
        selected_opcua_nodes: vec![],
        use_source_timestamp: false,
        ship_display_name: None,
        ship_hostname: None,
    }
}

#[test]
fn test_worker_creation() {
    // Test that we can create a worker
    let worker = WorkerHandle::new();

    // Test that we can send a command
    worker.send_command(WorkerCommand::DummyCommand).unwrap();

    // Wait for response with a timeout
    let received = wait_for_response(&worker, Duration::from_secs(2), |resp| {
        matches!(resp, WorkerResponse::DummyResponse)
    });

    assert!(received, "Expected DummyResponse but didn't receive it");
}

#[test]
fn test_worker_command_serialization() {
    // Test that all command variants can be sent and received
    let commands = vec![
        WorkerCommand::DummyCommand,
        WorkerCommand::SendTelegrafConfig {
            config: create_test_config(),
        },
        WorkerCommand::BackupInfluxDB {
            config: create_test_config(),
        },
        WorkerCommand::BackupGrafana {
            config: create_test_config(),
        },
        WorkerCommand::GetTelegrafStatus {
            config: create_test_config(),
        },
        WorkerCommand::GetTelegrafLogs {
            config: create_test_config(),
            lines: 10,
        },
        WorkerCommand::CheckServiceStatus {
            config: create_test_config(),
            service_url: "http://example.com".to_string(),
            service_type: crate::backend::ServiceType::InfluxDB,
            timeout_secs: 5,
        },
        WorkerCommand::CheckInfluxDbStatus {
            config: create_test_config(),
        },
        WorkerCommand::ExecuteSshCommand {
            host: "localhost".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            command: "echo test".to_string(),
        },
        WorkerCommand::SendFileOverSsh {
            host: "localhost".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            local_path: PathBuf::from("/tmp/test"),
            remote_path: "/tmp/remote_test".to_string(),
        },
    ];

    let worker = WorkerHandle::new();

    for cmd in commands {
        worker.send_command(cmd).unwrap();
        // Give the worker some time to process
        thread::sleep(Duration::from_millis(50));

        // Skip response checking since we're just testing command serialization
        // and some commands don't send responses
        let _ = worker.try_get_response();
    }
}

#[test]
fn test_worker_response_handling() {
    let worker = WorkerHandle::new();

    // Test that we can send a command and get a response
    worker.send_command(WorkerCommand::DummyCommand).unwrap();

    // Wait for the first response
    let first_response = wait_for_response(&worker, Duration::from_secs(2), |resp| {
        matches!(resp, WorkerResponse::DummyResponse)
    });

    assert!(
        first_response,
        "Expected first response to be DummyResponse"
    );

    // Second try should get none
    let second_response = worker.try_get_response();
    assert!(
        second_response.is_none(),
        "Expected no second response, got: {:?}",
        second_response
    );
}

#[test]
fn test_worker_multiple_commands() {
    const COMMAND_COUNT: usize = 5;
    const MAX_ATTEMPTS: usize = 10;

    // Try multiple times to account for timing issues
    for attempt in 1..=MAX_ATTEMPTS {
        let worker = WorkerHandle::new();
        // Send multiple commands
        for _ in 0..COMMAND_COUNT {
            if worker.send_command(WorkerCommand::DummyCommand).is_err() {
                panic!("Failed to send command");
            }
        }

        // Wait for all responses with a timeout
        let start = Instant::now();
        let timeout = Duration::from_secs(2);
        let mut responses = Vec::new();

        while start.elapsed() < timeout && responses.len() < COMMAND_COUNT {
            if let Some(response) = worker.try_get_response() {
                responses.push(response);
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }

        // Verify all responses are DummyResponse
        for response in &responses {
            assert!(
                matches!(response, WorkerResponse::DummyResponse),
                "Unexpected response: {:?}",
                response
            );
        }

        // If we got all responses, we're done
        if responses.len() == COMMAND_COUNT {
            // Verify no extra responses
            thread::sleep(Duration::from_millis(10)); // Small delay to ensure no more responses
            assert!(
                worker.try_get_response().is_none(),
                "Unexpected extra response received"
            );
            return; // Test passed
        }

        // If this isn't the last attempt, log and retry
        if attempt < MAX_ATTEMPTS {
            eprintln!(
                "Attempt {}/{}: Only received {}/{} responses, retrying...",
                attempt,
                MAX_ATTEMPTS,
                responses.len(),
                COMMAND_COUNT
            );
            continue;
        }

        // If we get here, we've exhausted all attempts
        panic!(
            "Failed to receive all responses after {} attempts. Received {}/{} responses.",
            MAX_ATTEMPTS,
            responses.len(),
            COMMAND_COUNT
        );
    }
}
