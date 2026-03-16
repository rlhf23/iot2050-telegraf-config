use eframe::egui;
use crate::{
    backend::opcua_poller::OpcUaNode,
    gui_controller::{GuiController, OpcUaBrowseState},
};

/// Handles all GUI rendering logic
pub struct GuiRenderer;

impl GuiRenderer {
    /// Render the main configuration section
    pub fn render_configuration(ui: &mut egui::Ui, controller: &mut GuiController) {
        ui.collapsing("Configuration", |ui| {
            // Folder selection
            ui.horizontal(|ui| {
                ui.label("XML Folder:");
                if ui.button("Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        controller.update_folder(path);
                    }
                }
                ui.label(controller.config.folder.to_string_lossy().to_string());
            });

            // Main Configuration using Grid
            egui::Grid::new("config_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    // IP Configuration
                    ui.label("OPC IP:");
                    ui.text_edit_singleline(&mut controller.config.ip);
                    ui.end_row();

                    // IOT Host with validation
                    ui.label("IOT Host:");
                    let text_edit = egui::TextEdit::singleline(&mut controller.config.iot_host);
                    if controller.form_state.show_iot_host_error {
                        egui::Frame::NONE
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(255, 0, 0),
                            ))
                            .show(ui, |ui| ui.add(text_edit));
                    } else {
                        ui.add(text_edit);
                    }
                    ui.end_row();
                });

            // Test Inputs Toggle
            ui.horizontal(|ui| {
                ui.label("Include System Inputs");
                ui.checkbox(
                    &mut controller.config.include_test_inputs,
                    "CPU, Disk, Memory, of the IOT device",
                );
            });

            // Credentials
            Self::render_credentials(ui, controller);
        });
    }

    /// Render the credentials section
    fn render_credentials(ui: &mut egui::Ui, controller: &mut GuiController) {
        ui.collapsing("Credentials", |ui| {
            egui::Grid::new("credentials_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // OPC Username
                    ui.label("OPC Username:");
                    ui.text_edit_singleline(&mut controller.config.username);
                    ui.end_row();
                    // OPC Password
                    ui.label("OPC Password:");
                    ui.add(egui::TextEdit::singleline(&mut controller.config.password).password(true));
                    ui.end_row();
                    // IOT Username
                    ui.label("IOT Username:");
                    ui.text_edit_singleline(&mut controller.config.iot_username);
                    ui.end_row();
                    // IOT Password
                    ui.label("IOT Password:");
                    ui.add(egui::TextEdit::singleline(&mut controller.config.iot_password).password(true));
                    ui.end_row();
                });

            // Output Format Toggle
            Self::render_output_format_toggle(ui, controller);
        });
    }

    /// Render the output format toggle
    fn render_output_format_toggle(ui: &mut egui::Ui, controller: &mut GuiController) {
        let mut is_prometheus = controller
            .config
            .output_format
            .clone()
            .unwrap_or_else(|| "influxdb".to_string())
            == "prometheus";

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Output Format:");

            let toggle_text = if is_prometheus {
                "Prometheus"
            } else {
                "InfluxDB"
            };
            if ui.button(toggle_text).clicked() {
                is_prometheus = !is_prometheus;
                controller.config.output_format = Some(if is_prometheus {
                    "prometheus".to_string()
                } else {
                    "influxdb".to_string()
                });
            }

            ui.label(if is_prometheus {
                "(exposes metrics via HTTP)"
            } else {
                "(sends to InfluxDB)"
            });
        });
    }

    /// Render the XML files configuration section
    pub fn render_xml_files_configuration(ui: &mut egui::Ui, controller: &mut GuiController) {
        ui.heading("XML Files Configuration");
        if controller.xml_files.is_empty() {
            ui.label("No XML files found in the selected folder");
        } else {
            ui.label("Configure XML files (check for listeners/subscribers):");
            
            // Clone the files list to avoid borrowing issues
            let xml_files = controller.xml_files.clone();
            let mut ip_changes = Vec::new(); // Store IP changes to apply later
            
            for (i, file) in xml_files.iter().enumerate() {
                ui.group(|ui| {
                    // File name and listener checkbox
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut controller.selected_listener_files[i], "Listener");
                        ui.strong(file);
                    });

                    // Get or create config for this file
                    let file_config = controller.file_configs.entry(file.clone()).or_default();

                    // Use grid layout for all fields in the XML file configuration
                    egui::Grid::new(&format!("xml_file_grid_{}", i))
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            // Namespace input
                            ui.label("Namespace:");
                            let text_edit = egui::TextEdit::singleline(&mut file_config.namespace);
                            if controller.form_state.show_namespace_error {
                                egui::Frame::NONE
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(255, 0, 0),
                                    ))
                                    .show(ui, |ui| ui.add(text_edit));
                            } else {
                                ui.add(text_edit);
                            }
                            ui.end_row();

                            // IP Address input
                            ui.label("OPC IP:");

                            // Check if we have a validation error for this file
                            let has_error = controller.form_state.ip_errors.get(file).unwrap_or(&false);

                            // Show the field with appropriate styling
                            let response = if *has_error {
                                // If there's an error, show red border
                                let response = egui::Frame::NONE
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(255, 0, 0),
                                    ))
                                    .show(ui, |ui| {
                                        ui.add(egui::TextEdit::singleline(&mut file_config.ip)
                                            .hint_text(&controller.config.ip))
                                    })
                                    .inner;
                                response.on_hover_text("Invalid IP format. Must be four numbers 0-255 separated by dots (e.g., 192.168.1.1)")
                            } else {
                                // No error, show normal text field
                                let response = ui.add(egui::TextEdit::singleline(&mut file_config.ip)
                                    .hint_text(&controller.config.ip));
                                response.on_hover_text("Override the default OPC IP address for this file")
                            };

                            // Store IP changes to apply later
                            if response.changed() {
                                ip_changes.push((file.clone(), file_config.ip.clone()));
                            }
                            ui.end_row();

                            // Interval input
                            let is_listener = controller.selected_listener_files[i];
                            let label = if is_listener {
                                "Sampling Interval (ms):"
                            } else {
                                "Interval (ms):"
                            };
                            ui.label(label);

                            let default_interval = if is_listener { "500" } else { "1000" };
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut file_config.interval_ms)
                                    .hint_text(default_interval),
                            );

                            response.on_hover_text(if is_listener {
                                "Default: 500ms for listeners"
                            } else {
                                "Default: 1000ms for regular files"
                            });
                            ui.end_row();
                    });
                });
                ui.add_space(4.0);
            }
            
            // Apply IP changes after the loop
            for (file, ip) in ip_changes {
                controller.validate_file_ip(&file, &ip);
            }
        }
    }

    /// Render the main action buttons
    pub fn render_main_action_buttons(ui: &mut egui::Ui, controller: &mut GuiController) {
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                // OPC UA Browser button
                if ui.button("Browse OPC UA Structure").clicked() {
                    controller.start_opcua_browsing();
                }
            
                if ui.button("Get OPC UA Namespaces").clicked() {
                    controller.start_getting_namespaces();
                }
            });
            
            if ui.button("Generate Config").clicked() {
                controller.generate_config();
            }

            if ui.button("Send Config").clicked() {
                controller.send_config();
            }
        });
    }

    /// Render the other commands section
    pub fn render_other_commands(ui: &mut egui::Ui, controller: &mut GuiController) {
        ui.collapsing("Other Commands", |ui| {
            ui.horizontal(|ui| {
                if ui.button("💾 Backup InfluxDB").clicked() {
                    controller.backup_influxdb();
                }

                if ui.button("📊 Backup Grafana").clicked() {
                    controller.backup_grafana();
                }
            });

            ui.horizontal(|ui| {
                if ui.button("🔍 Get Telegraf Status").clicked() {
                    controller.get_telegraf_status();
                }

                if ui.button("📋 Get Telegraf Logs").clicked() {
                    controller.get_telegraf_logs();
                }
                
                if ui.button("🔄 Restart Telegraf").clicked() {
                    controller.restart_telegraf();
                }
            });

            ui.horizontal(|ui| {
                if ui.button("🕐 Sync Device Time").clicked() {
                    controller.sync_time();
                }
            });

            // Service status check
            ui.horizontal(|ui| {
                let is_prometheus = controller
                    .config
                    .output_format
                    .clone()
                    .unwrap_or_else(|| "influxdb".to_string())
                    == "prometheus";

                let service_name = if is_prometheus { "Prometheus" } else { "InfluxDB" };

                if ui.button(format!("✅ Check {} Status", service_name)).clicked() {
                    controller.check_service_status();
                }
            });
        });
    }

    /// Render the status messages section
    pub fn render_status_messages(ui: &mut egui::Ui, controller: &mut GuiController) {
        if !controller.status_messages.is_empty() {
            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("Command Output:");
                if controller.is_working {
                    ui.add(egui::Spinner::new().size(16.0));
                    ui.label("Working...");
                }
                if ui.button("Clear").clicked() {
                    controller.clear_status_messages();
                }
            });
            
            let frame = egui::Frame::dark_canvas(ui.style())
                .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
                
            frame.show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                        
                        for (i, message) in controller.status_messages.iter().enumerate() {
                            if i > 0 {
                                ui.separator();
                            }
                            for line in message.lines() {
                                ui.label(line);
                            }
                        }
                        
                        if controller.should_scroll {
                            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover())
                                .on_hover_cursor(egui::CursorIcon::Default);
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            controller.should_scroll = false;
                        }
                    });
                
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Showing {} message(s)", controller.status_messages.len()));
                });
            });
        }
    }

    /// Render the OPC UA browser window
    pub fn render_opcua_browser(ctx: &egui::Context, controller: &mut GuiController) {
        if controller.show_opcua_browser {
            egui::Window::new("OPC UA Browser")
                .default_size([400.0, 600.0])
                .show(ctx, |ui| {
                    match &controller.opcua_browse_state {
                        OpcUaBrowseState::BrowsingNodes => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(&controller.browse_status_message);
                            });
                        }
                        OpcUaBrowseState::BrowsingNodesComplete => {
                            if !controller.opcua_nodes.is_empty() {
                                let mut temp_nodes = std::mem::take(&mut controller.opcua_nodes);
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    Self::render_node_tree(ui, &mut temp_nodes, 0, controller);
                                });
                                controller.opcua_nodes = temp_nodes;
                            } else {
                                ui.label(&controller.browse_status_message);
                            }
                        }
                        OpcUaBrowseState::BrowsingNodesFailed(err) => {
                            ui.colored_label(egui::Color32::RED, format!("Failed to browse OPC UA structure: {}", err));
                        }
                        OpcUaBrowseState::Idle => {
                            if !controller.opcua_nodes.is_empty() {
                                let mut temp_nodes = std::mem::take(&mut controller.opcua_nodes);
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    Self::render_node_tree(ui, &mut temp_nodes, 0, controller);
                                });
                                controller.opcua_nodes = temp_nodes;
                            } else {
                                ui.label("Click 'Browse OPC UA Structure' or 'Refresh Structure' to load nodes.");
                            }
                        }
                        _ => { 
                            ui.label(if controller.browse_status_message.is_empty() { "OPC UA Browser" } else { &controller.browse_status_message });
                        }
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Add Selected to Config").clicked() {
                            controller.add_selected_nodes_to_config();
                            controller.status_messages.push("Selected OPC UA nodes added to configuration.".to_string());
                            controller.should_scroll = true;
                        }
                        if ui.button("Refresh Structure").clicked() {
                            controller.refresh_opcua_structure();
                        }
                        if ui.button("Close").clicked() {
                            controller.show_opcua_browser = false;
                        }
                    });
                });
        }
    }

    /// Render the OPC UA node tree recursively
    fn render_node_tree(
        ui: &mut egui::Ui,
        nodes: &mut [OpcUaNode],
        indent_level: usize,
        controller: &GuiController,
    ) {
        for node in nodes.iter_mut() {
            let indent = (indent_level as f32) * 20.0;
            ui.horizontal(|ui| {
                ui.add_space(indent);

                let is_folder = node.is_folder_node();
                
                if (node.node_class == opcua::types::NodeClass::Variable || is_folder) && indent_level > 0 {
                    if ui.checkbox(&mut node.selected, "").changed() {
                        if !node.selected && node.node_class == opcua::types::NodeClass::Variable {
                            node.deselect_children();
                        }
                        
                        if node.selected && is_folder && !node.children_loaded {
                            let node_clone = node.clone();
                            
                            if let Ok(children) = controller.load_node_children(&node_clone, indent_level) {
                                node.children = children;
                                node.children_loaded = true;
                                ui.ctx().request_repaint();
                            }
                        }
                    }
                }

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

                if is_folder {
                    let label = format!("{}{} ({:?})", node_icon, node.display_name, node.node_class);
                    
                    let header = ui.collapsing(label, |ui| {
                        if let Some(data_type) = &node.data_type {
                            ui.label(format!("Data Type: {}", data_type));
                        }
                        if let Some(description) = &node.description {
                            ui.label(format!("Description: {}", description));
                        }

                        if !node.children_loaded && node.children.is_empty() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Loading children...");
                            });

                            let node_clone = node.clone();
                            if let Ok(children) = controller.load_node_children(&node_clone, indent_level) {
                                node.children = children;
                                node.children_loaded = true;
                                ui.ctx().request_repaint();
                            }
                        } else {
                            Self::render_node_tree(ui, &mut node.children, indent_level + 1, controller);
                        }
                    });

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

    /// Render the selected nodes section
    pub fn render_selected_nodes(ui: &mut egui::Ui, controller: &mut GuiController) {
        if !controller.config.selected_opcua_nodes.is_empty() {
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
                        
                        for (i, node) in controller.config.selected_opcua_nodes.iter_mut().enumerate() {
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
                            controller.remove_selected_opcua_node(index);
                        }
                    });
                });
            });
        }
    }
}