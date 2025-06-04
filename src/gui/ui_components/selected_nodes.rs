use eframe::egui;
use super::super::ConfigManager;

pub struct SelectedNodesSection;

impl SelectedNodesSection {
    pub fn show(ui: &mut egui::Ui, config_manager: &mut ConfigManager) {
        if !config_manager.config.selected_opcua_nodes.is_empty() {
            ui.heading("Selected Nodes");
            ui.push_id("selected_nodes_area", |ui| {
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    egui::Grid::new("selected_opcua_nodes_grid")
                        .num_columns(5)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Node Name");
                            ui.label("Namespace");
                            ui.label("Measurement Name");
                            ui.label("Interval (ms)");
                            ui.label("Actions");
                            ui.end_row();
                            
                            let mut nodes_to_remove = Vec::new();
                            
                            for (i, node) in config_manager.config.selected_opcua_nodes.iter_mut().enumerate() {
                                ui.label(&node.display_name);
                                ui.label(&node.namespace.to_string());
                                
                                // Allow editing the measurement name
                                let mut measurement_name = node.measurement_name.clone();
                                if ui.text_edit_singleline(&mut measurement_name).changed() {
                                    node.measurement_name = measurement_name;
                                }
                                
                                // Allow editing the interval
                                let mut interval_str = node.interval_ms.to_string();
                                if ui.text_edit_singleline(&mut interval_str).changed() {
                                    if let Ok(interval) = interval_str.parse::<u32>() {
                                        node.interval_ms = interval;
                                    }
                                }
                                
                                // Remove button
                                if ui.button("Remove").clicked() {
                                    nodes_to_remove.push(i);
                                }
                                
                                ui.end_row();
                            }
                            
                            // Remove nodes that were marked for removal
                            for &index in nodes_to_remove.iter().rev() {
                                if index < config_manager.config.selected_opcua_nodes.len() {
                                    config_manager.config.selected_opcua_nodes.remove(index);
                                }
                            }
                        });
                });
            });
        }
    }
}
