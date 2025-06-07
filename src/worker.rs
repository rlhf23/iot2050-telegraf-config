use std::sync::mpsc::{channel, Sender, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use crate::backend::ssh_utils;
use crate::backend::{ConfigGenerator, ServiceType};
use crate::TelegrafConfig;
use std::path::PathBuf;
#[derive(Debug)]
pub enum WorkerCommand {
    DummyCommand,
    SendTelegrafConfig {
        config: TelegrafConfig,
    },
    BackupInfluxDB {
        config: TelegrafConfig,
    },
    BackupGrafana {
        config: TelegrafConfig,
    },
    GetTelegrafStatus {
        config: TelegrafConfig,
    },
    GetTelegrafLogs {
        config: TelegrafConfig,
        lines: usize,
    },
    CheckServiceStatus {
        config: TelegrafConfig,
        service_url: String,
        service_type: ServiceType,
        timeout_secs: u64,
    },
    CheckInfluxDbStatus {
        config: TelegrafConfig,
    },
    ExecuteSshCommand {
        host: String,
        username: String,
        password: String,
        command: String,
    },
    SendFileOverSsh {
        host: String,
        username: String,
        password: String,
        local_path: PathBuf,
        remote_path: String,
    },
    BrowseOpcUaNodes {
        config: TelegrafConfig,
    },
    GetOpcUaNamespaces {
        config: TelegrafConfig,
        xml_files: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum WorkerResponse {
    DummyResponse,
    SshCommandOutput(String),
    SshError(String),
    FileTransferComplete,
    FileTransferError(String),
    ProgressUpdate(String),
    OpcUaNodes(Vec<crate::backend::opcua_poller::OpcUaNode>),
    OpcUaNamespaces(std::collections::HashMap<String, u16>),
    OpcUaError(String),
    Error(String),  // Generic error response for unhandled commands
}

pub struct WorkerHandle {
    command_sender: Sender<WorkerCommand>,
    _response_sender: Sender<WorkerResponse>,
    response_receiver: Receiver<WorkerResponse>,
    _thread: thread::JoinHandle<()>,
}

impl WorkerHandle {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = channel();
        let (response_sender, response_receiver) = channel();

        // Clone the sender for the worker thread
        let thread_response_sender = response_sender.clone();

        let _thread = thread::spawn(move || {
            while let Ok(cmd) = command_receiver.recv() {
                let response_sender = thread_response_sender.clone();
                
                match cmd {
                    WorkerCommand::DummyCommand => {
                        // Spawn a new thread to handle this command
                        std::thread::spawn(move || {
                            // Simulate work
                            std::thread::sleep(Duration::from_millis(10));
                            let _ = response_sender.send(WorkerResponse::DummyResponse);
                        });
                        continue; // Skip the response sending below since we're handling it in the spawned thread
                    }
                    WorkerCommand::SendTelegrafConfig { config } => {
                        // Spawn a new thread to handle the SSH operation
                        std::thread::spawn(move || {
                            let (tx, rx) = std::sync::mpsc::channel();
                            
                            // Clone config for the SSH operation thread
                            let config_clone = config.clone();
                            let ssh_tx = tx.clone();
                            let ssh_response_sender = response_sender.clone();
                            
                            // Spawn SSH operation in a separate thread
                            let ssh_handle = std::thread::spawn(move || {
                                ssh_utils::send_and_restart_telegraf_with_progress(
                                    &config_clone.folder.join("telegraf.conf"),
                                    "/etc/telegraf/telegraf.conf",
                                    &config_clone.iot_host,
                                    &config_clone.iot_username,
                                    &config_clone.iot_password,
                                    ssh_tx,
                                )
                            });
                            
                            // Handle progress updates in this thread
                            let _last_progress = 0;
                            loop {
                                match rx.recv_timeout(Duration::from_millis(100)) {
                                    Ok(progress) => {
                                        // Just forward the progress update
                                        let _ = response_sender.send(WorkerResponse::ProgressUpdate(progress));
                                    }
                                    Err(RecvTimeoutError::Timeout) => {
                                        // No progress update, check if the SSH operation is done
                                        if ssh_handle.is_finished() {
                                            break;
                                        }
                                        // Small sleep to prevent busy waiting
                                        std::thread::sleep(Duration::from_millis(10));
                                    }
                                    Err(RecvTimeoutError::Disconnected) => {
                                        // Channel disconnected, exit the loop
                                        break;
                                    }
                                }
                            }
                            
                            // Get the final result from the SSH operation
                            match ssh_handle.join() {
                                Ok(Ok(_)) => {
                                    let _ = response_sender.send(WorkerResponse::SshCommandOutput(
                                        "Configuration sent and Telegraf restarted successfully.".to_string()
                                    ));
                                }
                                Ok(Err(e)) => {
                                    let _ = ssh_response_sender.send(WorkerResponse::SshError(
                                        format!("Failed to send config: {}", e)
                                    ));
                                }
                                Err(_) => {
                                    let _ = ssh_response_sender.send(WorkerResponse::SshError(
                                        "SSH thread panicked".to_string()
                                    ));
                                }
                            }
                        });
                        
                        // Don't send a response here - it will be sent by the worker thread
                        continue;
                    }
                    WorkerCommand::BackupInfluxDB { config } => {
                        match ConfigGenerator::new(config) {
                            Ok(generator) => {
                                match generator.backup_influx() {
                                    Ok(output) => WorkerResponse::SshCommandOutput(output),
                                    Err(e) => WorkerResponse::SshError(format!("Failed to backup InfluxDB: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::SshError(format!("Failed to create config generator: {}", e)),
                        }
                    }
                    WorkerCommand::BackupGrafana { config } => {
                        match ConfigGenerator::new(config) {
                            Ok(generator) => {
                                match generator.backup_grafana() {
                                    Ok(output) => WorkerResponse::SshCommandOutput(output),
                                    Err(e) => WorkerResponse::SshError(format!("Failed to backup Grafana: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::SshError(format!("Failed to create config generator: {}", e)),
                        }
                    }
                    WorkerCommand::GetTelegrafStatus { config } => {
                        match ConfigGenerator::new(config) {
                            Ok(generator) => {
                                match generator.get_telegraf_status() {
                                    Ok(output) => WorkerResponse::SshCommandOutput(output),
                                    Err(e) => WorkerResponse::SshError(format!("Failed to get Telegraf status: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::SshError(format!("Failed to create config generator: {}", e)),
                        }
                    }
                    WorkerCommand::GetTelegrafLogs { config, lines } => {
                        match ConfigGenerator::new(config) {
                            Ok(generator) => {
                                match generator.get_telegraf_logs(lines) {
                                    Ok(output) => WorkerResponse::SshCommandOutput(output),
                                    Err(e) => WorkerResponse::SshError(format!("Failed to get Telegraf logs: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::SshError(format!("Failed to create config generator: {}", e)),
                        }
                    }
                    WorkerCommand::CheckServiceStatus { config, service_url, service_type, timeout_secs } => {
                        match ConfigGenerator::new(config) {
                            Ok(generator) => {
                                match generator.check_service_status(&service_url, service_type.clone(), timeout_secs) {
                                    Ok(true) => WorkerResponse::SshCommandOutput("✅ Service is responding normally".to_string()),
                                    Ok(false) => WorkerResponse::SshCommandOutput("❌ Service is not responding".to_string()),
                                    Err(e) => WorkerResponse::SshError(format!("Failed to check service status: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::SshError(format!("Failed to create config generator: {}", e)),
                        }
                    }
                    WorkerCommand::CheckInfluxDbStatus { config } => {
                        match ConfigGenerator::new(config) {
                            Ok(generator) => {
                                match generator.check_influxdb_status() {
                                    Ok(true) => WorkerResponse::SshCommandOutput("✅ InfluxDB is responding normally".to_string()),
                                    Ok(false) => WorkerResponse::SshCommandOutput("❌ InfluxDB is not responding".to_string()),
                                    Err(e) => WorkerResponse::SshError(format!("Failed to check InfluxDB status: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::SshError(format!("Failed to create config generator: {}", e)),
                        }
                    }
                    WorkerCommand::ExecuteSshCommand { host, username, password, command } => {
                        match ssh_utils::execute_command_over_ssh(&host, &username, &password, &command) {
                            Ok(output) => WorkerResponse::SshCommandOutput(output),
                            Err(e) => WorkerResponse::SshError(format!("SSH command failed: {}", e)),
                        }
                    }
                    WorkerCommand::SendFileOverSsh { host, username, password, local_path, remote_path } => {
                        match ssh_utils::send_file_over_ssh(&local_path, &remote_path, &host, &username, &password) {
                            Ok(_) => WorkerResponse::FileTransferComplete,
                            Err(e) => WorkerResponse::FileTransferError(format!("File transfer failed: {}", e)),
                        }
                    }
                    WorkerCommand::BrowseOpcUaNodes { config } => {
                        match crate::backend::opcua_poller::OpcUaPoller::new(config) {
                            Ok(poller) => {
                                match poller.browse_complete_structure() {
                                    Ok(nodes) => WorkerResponse::OpcUaNodes(nodes),
                                    Err(e) => WorkerResponse::OpcUaError(format!("Error browsing OPC UA structure: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::OpcUaError(format!("Error creating OPC UA poller: {}", e)),
                        }
                    }
                    WorkerCommand::GetOpcUaNamespaces { config, xml_files } => {
                        match crate::backend::opcua_poller::OpcUaPoller::new(config) {
                            Ok(poller) => {
                                match poller.get_namespace_info(&xml_files) {
                                    Ok(namespace_map) => WorkerResponse::OpcUaNamespaces(namespace_map),
                                    Err(e) => WorkerResponse::OpcUaError(format!("Error getting namespaces: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::OpcUaError(format!("Error creating OPC UA poller: {}", e)),
                        }
                    }
                    _ => {
                        // Send response for unhandled commands
                        let _ = response_sender.send(WorkerResponse::Error("Command not implemented".to_string()));
                        continue;  // Skip the rest of the loop iteration
                    }
                };
                
                // Responses are now sent directly in each command handler
                // This is a no-op since we've already sent the response
            }
        });

        Self {
            command_sender,
            _response_sender: response_sender,
            response_receiver,
            _thread,
        }
    }

    pub fn send_command(&self, cmd: WorkerCommand) -> Result<(), std::sync::mpsc::SendError<WorkerCommand>> {
        self.command_sender.send(cmd)
    }

    pub fn try_get_response(&self) -> Option<WorkerResponse> {
        // Use a small timeout to prevent busy waiting
        match self.response_receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(response) => Some(response),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("Worker response channel disconnected");
                None
            }
        }
    }
}
