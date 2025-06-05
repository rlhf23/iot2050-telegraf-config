use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
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
    LoadOpcUaNodeChildren {
        config: TelegrafConfig,
        node_id: opcua::types::NodeId,
        browse_name: String,
        display_name: String,
        node_class: opcua::types::NodeClass,
        depth: usize,
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
    OpcUaNodeChildren {
        parent_node_id: opcua::types::NodeId,
        children: Vec<crate::backend::opcua_poller::OpcUaNode>,
    },
    OpcUaNamespaces(std::collections::HashMap<String, u16>),
    OpcUaError(String),
}

pub struct WorkerHandle {
    command_sender: Sender<WorkerCommand>,
    response_receiver: Receiver<WorkerResponse>,
    _thread: thread::JoinHandle<()>,
}

impl WorkerHandle {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = channel();
        let (response_sender, response_receiver) = channel();

        let thread = thread::spawn(move || {
            while let Ok(cmd) = command_receiver.recv() {
                let response = match cmd {
                    WorkerCommand::DummyCommand => {
                        // Simulate work
                        std::thread::sleep(Duration::from_secs(1));
                        WorkerResponse::DummyResponse
                    }
                    WorkerCommand::SendTelegrafConfig { config } => {
                        let response_sender = response_sender.clone();
                        
                        // Use a single thread to handle both SSH operation and progress updates
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
                            loop {
                                match rx.try_recv() {
                                    Ok(progress) => {
                                        let _ = ssh_response_sender.send(WorkerResponse::ProgressUpdate(progress));
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                        // Check if SSH thread is done
                                        if ssh_handle.is_finished() {
                                            break;
                                        }
                                        std::thread::sleep(std::time::Duration::from_millis(10));
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                        break;
                                    }
                                }
                            }
                            
                            // Wait for SSH operation to complete and send final result
                            match ssh_handle.join() {
                                Ok(Ok(_)) => {
                                    let _ = ssh_response_sender.send(WorkerResponse::SshCommandOutput(
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
                    WorkerCommand::LoadOpcUaNodeChildren { config, node_id, browse_name, display_name, node_class, depth } => {
                        match crate::backend::opcua_poller::OpcUaPoller::new(config) {
                            Ok(poller) => {
                                // Create a temporary node to load children for
                                let node = crate::backend::opcua_poller::OpcUaNode {
                                    node_id: node_id.clone(),
                                    browse_name: browse_name.clone(),
                                    display_name: display_name.clone(),
                                    node_class: node_class.clone(),
                                    data_type: None,
                                    description: None,
                                    children: Vec::new(),
                                    selected: false,
                                    children_loaded: false,
                                    has_more_children: false,
                                    continuation_point: None,
                                };

                                match poller.load_node_children(&node, depth) {
                                    Ok(children) => WorkerResponse::OpcUaNodeChildren {
                                        parent_node_id: node_id,
                                        children,
                                    },
                                    Err(e) => WorkerResponse::OpcUaError(format!("Error loading node children: {}", e)),
                                }
                            }
                            Err(e) => WorkerResponse::OpcUaError(format!("Error creating OPC UA poller: {}", e)),
                        }
                    }
                };
                
                if response_sender.send(response).is_err() {
                    break; // Channel was disconnected
                }
            }
        });

        Self {
            command_sender,
            response_receiver,
            _thread: thread,
        }
    }

    pub fn send_command(&self, cmd: WorkerCommand) -> Result<(), std::sync::mpsc::SendError<WorkerCommand>> {
        self.command_sender.send(cmd)
    }

    pub fn try_get_response(&self) -> Option<WorkerResponse> {
        match self.response_receiver.try_recv() {
            Ok(response) => Some(response),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                // Handle disconnection if needed
                None
            }
        }
    }
}
