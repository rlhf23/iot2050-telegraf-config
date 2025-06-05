use eframe::egui;
use sie_generate_config::{
    gui::{
        ConfigManager, OpcUaManager, WorkerManager,
        ui_components::*,
    },
    WorkerResponse,
};

struct TelegrafApp {
    config_manager: ConfigManager,
    opcua_manager: OpcUaManager,
    worker_manager: WorkerManager,
}

impl TelegrafApp {
    fn default() -> Self {
        Self {
            config_manager: ConfigManager::new(),
            opcua_manager: OpcUaManager::new(),
            worker_manager: WorkerManager::new(),
        }
    }
}

impl eframe::App for TelegrafApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for worker responses
        if let Some(response) = self.worker_manager.try_get_response() {
            let should_repaint = match &response {
                WorkerResponse::OpcUaNodes(nodes) => {
                    self.opcua_manager.handle_browse_complete(nodes.clone());
                    true
                }
                WorkerResponse::OpcUaNamespaces(namespace_map) => {
                    let message = self.opcua_manager.handle_namespaces_complete(
                        namespace_map.clone(),
                        &self.config_manager.xml_files,
                        &mut self.config_manager.file_configs,
                    );
                    self.worker_manager.status_message = message;
                    true
                }
                WorkerResponse::OpcUaError(err) => {
                    self.opcua_manager.handle_error(err.clone());
                    self.worker_manager.status_message = format!("OPC UA operation failed: {}", err);
                    true
                }
                _ => self.worker_manager.handle_response(response)
            };

            if should_repaint {
                ctx.request_repaint();
            }
        } else if self.worker_manager.is_working {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Telegraf Configuration Generator");
            
            // Configuration Section
            ConfigForm::show(ui, &mut self.config_manager);
            
            // XML Files Section
            XmlFilesSection::show(ui, &mut self.config_manager);
            
            // Main Action Buttons
            ActionButtons::show(
                ui,
                &mut self.config_manager,
                &mut self.opcua_manager,
                &mut self.worker_manager,
            );
            
            // Status Output
            StatusOutput::show(ui, &self.worker_manager);
            
            // Display currently selected nodes
            SelectedNodesSection::show(ui, &mut self.config_manager);
        });

        // Handle OPC UA Browser Window
        let browser_action = {
            OpcUaBrowserWindow::show(
                ctx,
                &mut self.opcua_manager.nodes,
                &mut self.opcua_manager.browse_state,
                &self.opcua_manager.browse_status_message,
                &self.config_manager.config,
                &mut self.opcua_manager.show_browser,
            )
        };
        
        // Handle browser actions
        match browser_action {
            OpcUaBrowserAction::AddSelectedToConfig => {
                self.opcua_manager.add_selected_nodes_to_config(&mut self.config_manager.config);
                self.worker_manager.status_message = "Selected OPC UA nodes added to configuration.".to_string();
            }
            OpcUaBrowserAction::RefreshStructure => {
                if let Err(e) = self.opcua_manager.refresh_structure(&self.worker_manager.worker, &self.config_manager.config) {
                    self.worker_manager.status_message = e;
                } else {
                    self.worker_manager.is_working = true;
                    self.worker_manager.status_message = "Refreshing OPC UA structure...".to_string();
                }
            }
            OpcUaBrowserAction::None => {}
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 1000.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Telegraf Config Generator",
        options,
        Box::new(|_cc| Ok(Box::new(TelegrafApp::default()))),
    )
}
