use eframe::egui;
use super::super::{
    ConfigManager,
    OpcUaManager,
    WorkerManager,
};

pub struct ActionButtons;

impl ActionButtons {
    pub fn show(
        ui: &mut egui::Ui, 
        config_manager: &mut ConfigManager,
        opcua_manager: &mut OpcUaManager,
        worker_manager: &mut WorkerManager,
    ) -> ActionButtonsResult {
        let result = ActionButtonsResult::default();

        // Main Action Buttons
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                // OPC UA Browser button
                if ui.button("Browse OPC UA Structure").clicked() {
                    opcua_manager.show_browser = true;
                    if let Err(e) = opcua_manager.start_browsing(&worker_manager.worker, &config_manager.config) {
                        worker_manager.status_message = e;
                    } else {
                        worker_manager.is_working = true;
                        worker_manager.status_message = "Browsing OPC UA structure...".to_string();
                    }
                }

                if ui.button("Get OPC UA Namespaces").clicked() {
                    if let Err(e) = opcua_manager.get_namespaces(
                        &worker_manager.worker, 
                        &config_manager.config, 
                        &config_manager.xml_files
                    ) {
                        worker_manager.status_message = e;
                    } else {
                        worker_manager.is_working = true;
                        worker_manager.status_message = "Getting OPC UA namespaces...".to_string();
                    }
                }
            });

            if ui.button("Generate Config").clicked() {
                match config_manager.validate_and_generate_config() {
                    Ok(message) => {
                        worker_manager.status_message = message;
                    }
                    Err(error) => {
                        worker_manager.status_message = error;
                    }
                }
            }

            if ui.button("Send Config").clicked() {
                if let Err(e) = worker_manager.send_config(config_manager.config.clone()) {
                    worker_manager.status_message = e;
                }
            }
        });

        // Other Commands Section
        ui.collapsing("Other Commands", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Backup InfluxDB").clicked() {
                    if let Err(e) = worker_manager.backup_influxdb(config_manager.config.clone()) {
                        worker_manager.status_message = e;
                    }
                }

                if ui.button("Backup Grafana").clicked() {
                    if let Err(e) = worker_manager.backup_grafana(config_manager.config.clone()) {
                        worker_manager.status_message = e;
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Get Telegraf Status").clicked() {
                    if let Err(e) = worker_manager.get_telegraf_status(config_manager.config.clone()) {
                        worker_manager.status_message = e;
                    }
                }

                if ui.button("Get Telegraf Logs").clicked() {
                    if let Err(e) = worker_manager.get_telegraf_logs(config_manager.config.clone()) {
                        worker_manager.status_message = e;
                    }
                }
            });

            // Service status check
            ui.horizontal(|ui| {
                let is_prometheus = config_manager.is_prometheus_format();
                let service_name = if is_prometheus { "Prometheus" } else { "InfluxDB" };

                if ui.button(format!("Check {} Status", service_name)).clicked() {
                    if let Err(e) = worker_manager.check_service_status(
                        config_manager.config.clone(), 
                        is_prometheus
                    ) {
                        worker_manager.status_message = e;
                    }
                }
            });
        });

        result
    }
}

#[derive(Default)]
pub struct ActionButtonsResult {
    // Add any action results if needed in the future
}
