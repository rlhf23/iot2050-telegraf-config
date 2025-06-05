use eframe::egui;
use crate::{
    backend::opcua_poller::OpcUaNode,
    worker::WorkerHandle,
    TelegrafConfig,
};
use super::super::OpcUaBrowseState;

pub struct OpcUaBrowserWindow;

impl OpcUaBrowserWindow {
    pub fn show(
        ctx: &egui::Context,
        nodes: &mut Vec<OpcUaNode>,
        browse_state: &mut OpcUaBrowseState,
        browse_status_message: &mut String,
        worker: &Option<WorkerHandle>,
        config: &TelegrafConfig,
        show_browser: &mut bool,
    ) -> Result<OpcUaBrowserAction, String> {
        use OpcUaBrowserAction::*;
        
        if !*show_browser {
            return Ok(None);
        }

        let mut result = OpcUaBrowserAction::None;
        
        // Store node loading requests to process after rendering
        let mut node_loading_requests = Vec::new();
        
        // Then render the UI
        egui::Window::new("OPC UA Browser")
            .default_size([400.0, 600.0])
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    match *browse_state {
                        OpcUaBrowseState::BrowsingNodes => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(browse_status_message.as_str());
                            });
                        }
                        OpcUaBrowseState::BrowsingNodesComplete | OpcUaBrowseState::Idle => {
                            if !nodes.is_empty() {
                                match Self::render_node_tree(
                                    ui,
                                    nodes,
                                    0,
                                    config,
                                    worker,
                                ) {
                                    Ok(node_indices) => {
                                        // Convert node indices back to node references
                                        for (idx, depth) in node_indices {
                                            let i = idx.get() - 1; // Convert back to 0-based index
                                            if let Some(node) = nodes.get(i) {
                                                node_loading_requests.push((
                                                    node.node_id.clone(),
                                                    node.browse_name.clone(),
                                                    node.display_name.clone(),
                                                    node.node_class,
                                                    depth,
                                                ));
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                                    }
                                }
                            } else {
                                ui.label(browse_status_message.as_str());
                            }
                        }
                        OpcUaBrowseState::BrowsingNodesFailed(ref err) => {
                            ui.colored_label(egui::Color32::RED, format!("Failed to browse OPC UA structure: {}", err));
                        }
                        _ => {}
                    }
                });
                
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
            
        // Process node loading requests after UI rendering is complete
        for (node_id, _browse_name, display_name, _node_class, _depth) in node_loading_requests {
            // We can't load children here directly anymore, so we'll need to return these requests
            // and let the caller handle them
            // For now, we'll just log an error
            eprintln!("Node loading is not implemented in this version: {} ({})", display_name, node_id);
        }
            
        Ok(result)
    }

    fn render_node_tree(
        ui: &mut egui::Ui,
        nodes: &mut [OpcUaNode],
        indent_level: usize,
        config: &TelegrafConfig,
        worker: &Option<WorkerHandle>,
    ) -> Result<Vec<(std::num::NonZeroUsize, usize)>, String> {
        use opcua::types::NodeClass;
        use std::num::NonZeroUsize;
        
        let mut nodes_needing_loading = Vec::new();
        
        // First pass: render nodes and collect indices of nodes that need loading
        for (i, node) in nodes.iter_mut().enumerate() {
            let is_folder = node.is_folder_node();
            let mut needs_loading = false;
            
            // Calculate indentation
            let indent = (indent_level as f32) * 20.0;
            
            // Render the node
            let response = ui.horizontal(|ui| {
                ui.add_space(indent);
                
                // Show checkboxes for variables that can be selected and for folders
                // Skip checkbox for the root node (at indent_level 0)
                if (node.node_class == NodeClass::Variable || is_folder) && indent_level > 0 {
                    if ui.checkbox(&mut node.selected, "").clicked() {
                        // For variables: If a node is deselected, also deselect all its children
                        if !node.selected && node.node_class == NodeClass::Variable {
                            node.deselect_children();
                        }
                        
                        // If this is a folder that needs children loaded
                        if node.selected && is_folder && !node.children_loaded {
                            // Mark as loading to prevent multiple requests
                            node.children_loaded = true;
                            needs_loading = true;
                        }
                    }
                }
                
                // Show node name and icon with appropriate styling
                let node_icon = match node.node_class {
                    NodeClass::Object => "📁 ",
                    NodeClass::Variable => "📊 ",
                    NodeClass::Method => "⚙️ ",
                    NodeClass::ObjectType => "📦 ",
                    NodeClass::VariableType => "📈 ",
                    NodeClass::ReferenceType => "🔗 ",
                    NodeClass::DataType => "🔢 ",
                    _ => "❓ ",
                };
                
                // Create hover text with node details
                let hover_text = format!(
                    "NodeId: {:?}\nNamespace: {}\nBrowse Name: {}{}",
                    node.node_id,
                    node.node_id.namespace,
                    node.browse_name,
                    if let Some(data_type) = &node.data_type {
                        format!("\nData Type: {}", data_type)
                    } else {
                        String::new()
                    }
                );
                
                // Render the node with appropriate styling
                let node_text = format!("{} {}", node_icon, node.display_name);
                ui.label(node_text).on_hover_text(hover_text);
                
                Ok::<_, String>(())
            });
            
            // Check for errors in the UI response
            if let Err(e) = response.inner {
                return Err(e);
            }
            
            // If this node needs loading, store its index (1-based to use NonZeroUsize)
            if needs_loading {
                if let Some(idx) = NonZeroUsize::new(i + 1) {
                    nodes_needing_loading.push((idx, indent_level + 1));
                }
            }
        }
        
        // Second pass: recursively render children of expanded nodes
        for (_i, node) in nodes.iter_mut().enumerate() {
            if node.selected && !node.children.is_empty() {
                let child_needs_loading = Self::render_node_tree(
                    ui,
                    &mut node.children,
                    indent_level + 1,
                    config,
                    worker,
                )?;
                
                // Add any child nodes that need loading to our main list
                nodes_needing_loading.extend(child_needs_loading);
            }
        }
        
        Ok(nodes_needing_loading)
    }
}

#[derive(Debug, PartialEq)]
pub enum OpcUaBrowserAction {
    None,
    AddSelectedToConfig,
    RefreshStructure,
}
