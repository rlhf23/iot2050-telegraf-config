use eframe::egui;
use sie_generate_config::{
    backend::opcua_poller::OpcUaNode,
    TelegrafConfig, WorkerCommand, WorkerHandle,
};

#[derive(Default, Debug, PartialEq, Clone)]
pub enum OpcUaBrowseState {
    #[default]
    Idle,
    BrowsingNodes,
    BrowsingNodesFailed(String),
    BrowsingNodesComplete,
    GettingNamespaces,
    GettingNamespacesFailed(String),
    GettingNamespacesComplete,
}

pub struct OpcUaManager {
    pub nodes: Vec<OpcUaNode>,
    pub show_browser: bool,
    pub browse_status_message: String,
    pub browse_state: OpcUaBrowseState,
}

impl OpcUaManager {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            show_browser: false,
            browse_status_message: String::new(),
            browse_state: OpcUaBrowseState::default(),
        }
    }

    pub fn start_browsing(&mut self, worker: &Option<WorkerHandle>, config: &TelegrafConfig) -> Result<(), String> {
        if self.browse_state != OpcUaBrowseState::BrowsingNodes 
            && self.browse_state != OpcUaBrowseState::BrowsingNodesComplete {
            
            self.nodes.clear();
            self.browse_state = OpcUaBrowseState::BrowsingNodes;
            self.browse_status_message = "Requesting OPC UA structure...".to_string();
            
            if let Some(worker) = worker {
                let command = WorkerCommand::BrowseOpcUaNodes {
                    config: config.clone(),
                };
                worker.send_command(command).map_err(|e| format!("Failed to start browsing: {}", e))?;
            } else {
                return Err("Worker not initialized".to_string());
            }
        } else if self.browse_state == OpcUaBrowseState::BrowsingNodesComplete {
            self.browse_status_message = "OPC UA structure previously loaded.".to_string();
        }
        Ok(())
    }

    pub fn get_namespaces(&mut self, worker: &Option<WorkerHandle>, config: &TelegrafConfig, xml_files: &[String]) -> Result<(), String> {
        if self.browse_state != OpcUaBrowseState::GettingNamespaces {
            self.browse_state = OpcUaBrowseState::GettingNamespaces;
            
            if let Some(worker) = worker {
                let command = WorkerCommand::GetOpcUaNamespaces {
                    config: config.clone(),
                    xml_files: xml_files.to_vec(),
                };
                worker.send_command(command).map_err(|e| format!("Failed to get namespaces: {}", e))?;
            } else {
                return Err("Worker not initialized".to_string());
            }
        }
        Ok(())
    }

    pub fn handle_browse_complete(&mut self, nodes: Vec<OpcUaNode>) {
        self.nodes = nodes;
        self.browse_state = OpcUaBrowseState::BrowsingNodesComplete;
        self.browse_status_message = if self.nodes.is_empty() {
            "OPC UA structure loaded, but no nodes found.".to_string()
        } else {
            "OPC UA structure loaded successfully.".to_string()
        };
    }

    pub fn handle_namespaces_complete(&mut self, namespace_map: std::collections::HashMap<String, u16>, xml_files: &[String], file_configs: &mut std::collections::HashMap<String, super::config_manager::XmlFileConfig>) -> String {
        let mut found_count = 0;
        for (file_name, namespace_index) in namespace_map {
            if let Some(full_path) = xml_files.iter().find(|path| path.ends_with(&file_name)) {
                if let Some(config) = file_configs.get_mut(full_path) {
                    config.namespace = namespace_index.to_string();
                    found_count += 1;
                }
            }
        }
        
        self.browse_state = OpcUaBrowseState::GettingNamespacesComplete;
        
        if found_count > 0 {
            format!("Found namespaces for {} XML files!", found_count)
        } else {
            "No matching namespaces found. Check XML filenames match namespace names.".to_string()
        }
    }

    pub fn handle_error(&mut self, error: String) {
        match self.browse_state {
            OpcUaBrowseState::BrowsingNodes => {
                self.browse_state = OpcUaBrowseState::BrowsingNodesFailed(error.clone());
                self.browse_status_message = format!("Failed to browse OPC UA structure: {}", error);
            }
            OpcUaBrowseState::GettingNamespaces => {
                self.browse_state = OpcUaBrowseState::GettingNamespacesFailed(error);
            }
            _ => {
                // For other states, just update status message
            }
        }
    }

    pub fn add_selected_nodes_to_config(&self, config: &mut TelegrafConfig) {
        config.selected_opcua_nodes.clear();
        let selected_nodes = OpcUaNode::convert_selected_nodes_to_config(&self.nodes);
        config.selected_opcua_nodes = selected_nodes;
    }

    pub fn refresh_structure(&mut self, worker: &Option<WorkerHandle>, config: &TelegrafConfig) -> Result<(), String> {
        if self.browse_state != OpcUaBrowseState::BrowsingNodes {
            self.nodes.clear();
            self.browse_state = OpcUaBrowseState::BrowsingNodes;
            self.browse_status_message = "Requesting OPC UA structure refresh...".to_string();
            
            if let Some(worker) = worker {
                let command = WorkerCommand::BrowseOpcUaNodes {
                    config: config.clone(),
                };
                worker.send_command(command).map_err(|e| format!("Failed to refresh structure: {}", e))?;
            } else {
                return Err("Worker not initialized".to_string());
            }
        }
        Ok(())
    }
}
