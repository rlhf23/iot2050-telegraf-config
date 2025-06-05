use eframe::egui;
use crate::{
    backend::opcua_poller::{OpcUaNode, OpcUaPoller},
    TelegrafConfig,
};
use super::super::OpcUaBrowseState;

pub struct OpcUaBrowserWindow;

impl OpcUaBrowserWindow {
    pub fn show(
        ctx: &egui::Context,
        nodes: &mut Vec<OpcUaNode>,
        browse_state: &mut OpcUaBrowseState,
        browse_status_message: &str,
        config: &TelegrafConfig,
        show_browser: &mut bool,
    ) -> OpcUaBrowserAction {
        if !*show_browser {
            return OpcUaBrowserAction::None;
        }

        let mut result = OpcUaBrowserAction::None;
        
        egui::Window::new("OPC UA Browser")
            .default_size([400.0, 600.0])
            .show(ctx, |ui| {
                match browse_state {
                    OpcUaBrowseState::BrowsingNodes => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(browse_status_message);
                        });
                    }
                    OpcUaBrowseState::BrowsingNodesComplete => {
                        if !nodes.is_empty() {
                            let mut temp_nodes = std::mem::take(nodes);
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                Self::render_node_tree(ui, &mut temp_nodes, 0, config);
                            });
                            *nodes = temp_nodes;
                        } else {
                            ui.label(browse_status_message);
                        }
                    }
                    OpcUaBrowseState::BrowsingNodesFailed(err) => {
                        ui.colored_label(egui::Color32::RED, format!("Failed to browse OPC UA structure: {}", err));
                    }
                    OpcUaBrowseState::Idle => {
                        if !nodes.is_empty() {
                            let mut temp_nodes = std::mem::take(nodes);
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                Self::render_node_tree(ui, &mut temp_nodes, 0, config);
                            });
                            *nodes = temp_nodes;
                        } else {
                            ui.label("Click 'Browse OPC UA' to load the node structure.");
                        }
                    }
                    _ => {}
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Add Selected to Config").clicked() {
                        result = OpcUaBrowserAction::AddSelectedToConfig;
                    }
                    if ui.button("Refresh Structure").clicked() {
                        result = OpcUaBrowserAction::RefreshStructure;
                    }
                    if ui.button("Close").clicked() {
                        *show_browser = false;
                    }
                });
            });
            
        result
    }

    fn render_node_tree(
        ui: &mut egui::Ui,
        nodes: &mut [OpcUaNode],
        indent_level: usize,
        config: &TelegrafConfig,
    ) {
        for node in nodes.iter_mut() {
            // Calculate indentation
            let indent = (indent_level as f32) * 20.0;
            ui.horizontal(|ui| {
                ui.add_space(indent);

                // Determine if this is a folder-like node
                let is_folder = node.is_folder_node();
                
                // Show checkboxes for variables that can be selected and for folders
                // Skip checkbox for the root node (at indent_level 0)
                if (node.node_class == opcua::types::NodeClass::Variable || is_folder) && indent_level > 0 {
                    if ui.checkbox(&mut node.selected, "").changed() {
                        // For variables: If a node is deselected, also deselect all its children
                        if !node.selected && node.node_class == opcua::types::NodeClass::Variable {
                            node.deselect_children();
                        }
                        
                        // For folders: When selected, ensure children are loaded
                        if node.selected && is_folder && !node.children_loaded {
                            // Clone to avoid borrow issues
                            let node_clone = node.clone();
                            
                            // Create a new poller to load children
                            if let Ok(poller) = OpcUaPoller::new(config.clone()) {
                                match poller.load_node_children(&node_clone, indent_level) {
                                    Ok(children) => {
                                        // Update the node with loaded children
                                        node.children = children;
                                        node.children_loaded = true;
                                        
                                        // Force a redraw
                                        ui.ctx().request_repaint();
                                    }
                                    Err(e) => {
                                        // Log the error but don't display it in the UI to avoid disrupting the layout
                                        eprintln!("Error loading folder children when selecting checkbox: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                // Node display - show different icons based on node type
                let node_icon = match node.node_class {
                    opcua::types::NodeClass::Object => "[O] ",
                    opcua::types::NodeClass::Variable => "[V] ",
                    opcua::types::NodeClass::Method => "[M] ",
                    opcua::types::NodeClass::ObjectType => "[T] ",
                    opcua::types::NodeClass::VariableType => "[VT] ",
                    opcua::types::NodeClass::ReferenceType => "[R] ",
                    opcua::types::NodeClass::DataType => "[D] ",
                    opcua::types::NodeClass::View => "[~] ",
                    _ => "[?] ",
                };

                // Check if this is a folder-like node that can have children
                if is_folder {
                    let label = format!("{}{} ({:?})", node_icon, node.display_name, node.node_class);
                    
                    // Use collapsing header to show node and its children
                    let header = ui.collapsing(label, |ui| {
                        // Show additional node information
                        if let Some(data_type) = &node.data_type {
                            ui.label(format!("Data Type: {}", data_type));
                        }
                        if let Some(description) = &node.description {
                            ui.label(format!("Description: {}", description));
                        }

                        // Check if children need to be loaded
                        if !node.children_loaded && node.children.is_empty() {
                            // Show loading indicator
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Loading children...");
                            });

                            // Clone to avoid borrow issues
                            let node_clone = node.clone();

                            // Create a new poller to load children
                            if let Ok(poller) = OpcUaPoller::new(config.clone()) {
                                match poller.load_node_children(&node_clone, indent_level) {
                                    Ok(children) => {
                                        // Update the node with loaded children
                                        node.children = children;
                                        node.children_loaded = true;

                                        // Force a redraw
                                        ui.ctx().request_repaint();
                                    }
                                    Err(e) => {
                                        ui.label(format!("Error loading children: {}", e));
                                    }
                                }
                            }
                        } else {
                            // Render children recursively
                            Self::render_node_tree(ui, &mut node.children, indent_level + 1, config);
                        }
                    });

                    // If the header is expanded, we don't need to show the hover text
                    if !header.fully_open() {
                        header.header_response.on_hover_text(format!(
                            "NodeId: {:?}\nNamespace: {}\nBrowse Name: {}{}",
                            node.node_id,
                            node.node_id.namespace,
                            node.browse_name,
                            if let Some(data_type) = &node.data_type {
                                format!("\nData Type: {}", data_type)
                            } else {
                                String::new()
                            }
                        ));
                    }
                } else {
                    // Leaf node without children
                    let label = format!("{}{} ({:?})", node_icon, node.display_name, node.node_class);
                    ui.label(label).on_hover_text(format!(
                        "NodeId: {:?}\nNamespace: {}\nBrowse Name: {}{}",
                        node.node_id,
                        node.node_id.namespace,
                        node.browse_name,
                        if let Some(data_type) = &node.data_type {
                            format!("\nData Type: {}", data_type)
                        } else {
                            String::new()
                        }
                    ));
                }
            });
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum OpcUaBrowserAction {
    None,
    AddSelectedToConfig,
    RefreshStructure,
}
