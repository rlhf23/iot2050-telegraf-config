use sie_generate_config::{
    backend::ServiceType,
    WorkerCommand, WorkerHandle, WorkerResponse, TelegrafConfig,
};

pub struct WorkerManager {
    pub worker: Option<WorkerHandle>,
    pub is_working: bool,
    pub status_message: String,
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            worker: Some(WorkerHandle::new()),
            is_working: false,
            status_message: String::new(),
        }
    }

    /// Helper method to send a command to the worker and update UI state
    pub fn send_command(&mut self, command: WorkerCommand, working_message: &str) -> Result<(), String> {
        if let Some(worker) = &self.worker {
            worker.send_command(command).map_err(|e| format!("Failed to start operation: {}", e))?;
            self.is_working = true;
            self.status_message = working_message.to_string();
            Ok(())
        } else {
            Err("Worker not initialized".to_string())
        }
    }

    pub fn send_config(&mut self, config: TelegrafConfig) -> Result<(), String> {
        self.send_command(
            WorkerCommand::SendTelegrafConfig { config },
            "Sending configuration..."
        )
    }

    pub fn backup_influxdb(&mut self, config: TelegrafConfig) -> Result<(), String> {
        self.send_command(
            WorkerCommand::BackupInfluxDB { config },
            "Backing up InfluxDB..."
        )
    }

    pub fn backup_grafana(&mut self, config: TelegrafConfig) -> Result<(), String> {
        self.send_command(
            WorkerCommand::BackupGrafana { config },
            "Backing up Grafana..."
        )
    }

    pub fn get_telegraf_status(&mut self, config: TelegrafConfig) -> Result<(), String> {
        self.send_command(
            WorkerCommand::GetTelegrafStatus { config },
            "Retrieving Telegraf status..."
        )
    }

    pub fn get_telegraf_logs(&mut self, config: TelegrafConfig) -> Result<(), String> {
        self.send_command(
            WorkerCommand::GetTelegrafLogs { config, lines: 30 },
            "Retrieving Telegraf logs..."
        )
    }

    pub fn check_service_status(&mut self, config: TelegrafConfig, is_prometheus: bool) -> Result<(), String> {
        let service_url = config.iot_host.clone();
        let service_name = if is_prometheus { "Prometheus" } else { "InfluxDB" };

        if is_prometheus {
            self.send_command(
                WorkerCommand::CheckServiceStatus {
                    config,
                    service_url: service_url.clone(),
                    service_type: ServiceType::Prometheus,
                    timeout_secs: 5,
                },
                &format!("Checking {} status at {}...", service_name, service_url)
            )
        } else {
            self.send_command(
                WorkerCommand::CheckInfluxDbStatus { config },
                &format!("Checking InfluxDB status at {}...", service_url)
            )
        }
    }

    pub fn try_get_response(&mut self) -> Option<WorkerResponse> {
        if let Some(worker) = &self.worker {
            worker.try_get_response()
        } else {
            None
        }
    }

    pub fn handle_response(&mut self, response: WorkerResponse) -> bool {
        // Process progress updates separately to maintain working state
        if let WorkerResponse::ProgressUpdate(progress) = &response {
            self.status_message = progress.clone();
            return false; // Don't set is_working to false for progress updates
        } else {
            self.is_working = false;
        }

        // Process the response and return true if we should request a repaint
        match response {
            WorkerResponse::DummyResponse => {
                self.status_message = "Dummy operation completed!".to_string();
            }
            WorkerResponse::SshCommandOutput(output) => {
                self.status_message = format!("SSH command output:\n{}", output);
            }
            WorkerResponse::SshError(err) => {
                self.status_message = format!("SSH error: {}", err);
            }
            WorkerResponse::FileTransferComplete => {
                self.status_message = "File transfer completed successfully".to_string();
            }
            WorkerResponse::FileTransferError(err) => {
                self.status_message = format!("File transfer error: {}", err);
            }
            WorkerResponse::ProgressUpdate(_) => {
                // Already handled above
            }
            WorkerResponse::OpcUaNodes(_) | WorkerResponse::OpcUaNamespaces(_) | WorkerResponse::OpcUaError(_) => {
                // These responses are handled by the OpcUaManager
                return true;
            }
        }

        true // Request repaint for most responses
    }
}
