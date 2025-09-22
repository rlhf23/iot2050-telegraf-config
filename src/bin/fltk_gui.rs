use fltk::{
    app::{self, App, Receiver, Sender},
    button::{Button, CheckButton, RadioButton},
    enums::{Align, Color, Font},
    frame::Frame,
    group::{Group, Pack, PackType, Scroll, Tabs},
    input::{Input, SecretInput},
    prelude::*,
    text::{TextBuffer, TextDisplay},
    window::Window,
};
use sie_generate_config::gui_controller::GuiController;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom messages for inter-thread communication
#[derive(Debug, Clone)]
pub enum AppMessage {
    UpdateStatus(String),
    RefreshFiles,
    WorkerComplete,
    UpdateOpcUaNodes,
}

// Dark theme colors
const BG_COLOR: Color = Color::from_u32(0x2d2d30);
const WIDGET_BG: Color = Color::from_u32(0x3c3c41);
const TEXT_COLOR: Color = Color::from_u32(0xdcdcdc);
const ACCENT_COLOR: Color = Color::from_u32(0x0078d7);
const ERROR_COLOR: Color = Color::from_u32(0xdc3232);
const SUCCESS_COLOR: Color = Color::from_u32(0x32c832);

pub struct TelegrafFltkApp {
    controller: Arc<Mutex<GuiController>>,
    sender: Sender<AppMessage>,
    receiver: Receiver<AppMessage>,
    
    // UI Components
    main_window: Window,
    tabs: Tabs,
    
    // Configuration tab widgets
    ip_input: Input,
    iot_host_input: Input,
    username_input: Input,
    password_input: SecretInput,
    iot_username_input: Input,
    iot_password_input: SecretInput,
    folder_input: Input,
    include_test_inputs: CheckButton,
    output_format_prometheus: RadioButton,
    output_format_influx: RadioButton,
    anonymous_auth: CheckButton,
    
    // XML Files tab widgets
    xml_files_scroll: Scroll,
    xml_files_group: Group,
    file_checkboxes: Vec<CheckButton>,
    file_inputs: Vec<(Input, Input, Input)>, // namespace, interval, ip
    
    // Actions tab widgets
    generate_btn: Button,
    send_btn: Button,
    backup_influx_btn: Button,
    backup_grafana_btn: Button,
    status_btn: Button,
    logs_btn: Button,
    service_status_btn: Button,
    restart_telegraf_btn: Button,
    
    // Status tab widgets
    status_display: TextDisplay,
    status_buffer: TextBuffer,
    clear_status_btn: Button,
    
    // OPC-UA Browser
    opcua_browser_window: Option<Window>,
    browse_btn: Button,
}

impl TelegrafFltkApp {
    pub fn new() -> Self {
        let _app = App::default();
        let (sender, receiver) = app::channel::<AppMessage>();
        
        // Set up dark theme
        app::set_scheme(app::Scheme::Gtk);
        app::set_background_color(45, 45, 48);
        app::set_background2_color(60, 60, 65);
        app::set_foreground_color(220, 220, 220);
        app::set_selection_color(0, 120, 215);
        
        let controller = Arc::new(Mutex::new(GuiController::new()));
        
        // Create main window
        let mut main_window = Window::new(100, 100, 900, 700, "Telegraf Configuration Generator");
        main_window.set_color(BG_COLOR);
        
        // Create tabs
        let mut tabs = Tabs::new(10, 10, 880, 680, "");
        tabs.set_color(WIDGET_BG);
        tabs.set_selection_color(ACCENT_COLOR);
        tabs.set_label_color(TEXT_COLOR);
        
        // Configuration Tab
        let mut config_group = Group::new(10, 40, 880, 650, "Configuration");
        config_group.set_color(BG_COLOR);
        
        let mut config_scroll = Scroll::new(10, 40, 880, 650, "");
        config_scroll.set_color(BG_COLOR);
        config_scroll.set_type(fltk::group::ScrollType::Vertical);
        
        let mut config_inner = Group::new(20, 50, 860, 600, "");
        config_inner.set_color(BG_COLOR);
        
        let mut y_pos = 60;
        
        // Folder selection
        let mut folder_label = Frame::new(30, y_pos, 120, 30, "XML Folder:");
        folder_label.set_align(Align::Left | Align::Inside);
        folder_label.set_label_color(TEXT_COLOR);
        
        let mut folder_input = Input::new(160, y_pos, 500, 30, "");
        folder_input.set_color(WIDGET_BG);
        folder_input.set_text_color(TEXT_COLOR);
        let mut browse_folder_btn = Button::new(670, y_pos, 100, 30, "Browse");
        browse_folder_btn.set_color(ACCENT_COLOR);
        browse_folder_btn.set_label_color(TEXT_COLOR);
        
        y_pos += 50;
        
        // Main configuration inputs with proper positioning
        let mut ip_label = Frame::new(30, y_pos, 120, 30, "OPC IP:");
        ip_label.set_align(Align::Left | Align::Inside);
        ip_label.set_label_color(TEXT_COLOR);
        let mut ip_input = Input::new(160, y_pos, 200, 30, "");
        ip_input.set_color(WIDGET_BG);
        ip_input.set_text_color(TEXT_COLOR);
        
        y_pos += 40;
        
        let mut iot_host_label = Frame::new(30, y_pos, 120, 30, "IOT Host:");
        iot_host_label.set_align(Align::Left | Align::Inside);
        iot_host_label.set_label_color(TEXT_COLOR);
        let mut iot_host_input = Input::new(160, y_pos, 200, 30, "");
        iot_host_input.set_color(WIDGET_BG);
        iot_host_input.set_text_color(TEXT_COLOR);
        
        y_pos += 60;
        
        // Credentials section
        let mut cred_frame = Frame::new(30, y_pos, 300, 30, "Credentials:");
        cred_frame.set_align(Align::Left | Align::Inside);
        cred_frame.set_label_color(TEXT_COLOR);
        cred_frame.set_label_font(Font::HelveticaBold);
        
        y_pos += 40;
        
        let mut username_label = Frame::new(30, y_pos, 120, 30, "OPC Username:");
        username_label.set_align(Align::Left | Align::Inside);
        username_label.set_label_color(TEXT_COLOR);
        let mut username_input = Input::new(160, y_pos, 200, 30, "");
        username_input.set_color(WIDGET_BG);
        username_input.set_text_color(TEXT_COLOR);
        
        y_pos += 40;
        
        let mut password_label = Frame::new(30, y_pos, 120, 30, "OPC Password:");
        password_label.set_align(Align::Left | Align::Inside);
        password_label.set_label_color(TEXT_COLOR);
        let mut password_input = SecretInput::new(160, y_pos, 200, 30, "");
        password_input.set_color(WIDGET_BG);
        password_input.set_text_color(TEXT_COLOR);
        
        y_pos += 40;
        
        let mut iot_username_label = Frame::new(30, y_pos, 120, 30, "IOT Username:");
        iot_username_label.set_align(Align::Left | Align::Inside);
        iot_username_label.set_label_color(TEXT_COLOR);
        let mut iot_username_input = Input::new(160, y_pos, 200, 30, "");
        iot_username_input.set_color(WIDGET_BG);
        iot_username_input.set_text_color(TEXT_COLOR);
        
        y_pos += 40;
        
        let mut iot_password_label = Frame::new(30, y_pos, 120, 30, "IOT Password:");
        iot_password_label.set_align(Align::Left | Align::Inside);
        iot_password_label.set_label_color(TEXT_COLOR);
        let mut iot_password_input = SecretInput::new(160, y_pos, 200, 30, "");
        iot_password_input.set_color(WIDGET_BG);
        iot_password_input.set_text_color(TEXT_COLOR);
        
        y_pos += 60;
        
        // Options
        let mut include_test_inputs = CheckButton::new(30, y_pos, 400, 30, "Include System Inputs (CPU, Disk, Memory)");
        include_test_inputs.set_color(WIDGET_BG);
        include_test_inputs.set_label_color(TEXT_COLOR);
        
        y_pos += 40;
        
        let mut anonymous_auth = CheckButton::new(30, y_pos, 300, 30, "Anonymous OPC-UA Authentication");
        anonymous_auth.set_color(WIDGET_BG);
        anonymous_auth.set_label_color(TEXT_COLOR);
        
        y_pos += 60;
        
        // Output format
        let mut format_frame = Frame::new(30, y_pos, 300, 30, "Output Format:");
        format_frame.set_align(Align::Left | Align::Inside);
        format_frame.set_label_color(TEXT_COLOR);
        format_frame.set_label_font(Font::HelveticaBold);
        
        y_pos += 40;
        
        let mut output_format_prometheus = RadioButton::new(30, y_pos, 150, 30, "Prometheus");
        output_format_prometheus.set_color(WIDGET_BG);
        output_format_prometheus.set_label_color(TEXT_COLOR);
        output_format_prometheus.set_value(true);
        
        let mut output_format_influx = RadioButton::new(190, y_pos, 150, 30, "InfluxDB");
        output_format_influx.set_color(WIDGET_BG);
        output_format_influx.set_label_color(TEXT_COLOR);
        
        config_inner.end();
        config_scroll.end();
        config_group.end();
        
        // XML Files Tab
        let mut files_group = Group::new(10, 40, 880, 650, "XML Files");
        files_group.set_color(BG_COLOR);
        
        let mut xml_files_scroll = Scroll::new(10, 40, 880, 650, "");
        xml_files_scroll.set_color(BG_COLOR);
        xml_files_scroll.set_type(fltk::group::ScrollType::Vertical);
        
        let mut xml_files_group = Group::new(20, 50, 860, 600, "");
        xml_files_group.set_color(BG_COLOR);
        xml_files_group.end();
        
        xml_files_scroll.end();
        files_group.end();
        
        // Actions Tab
        let mut actions_group = Group::new(10, 40, 880, 650, "Actions");
        actions_group.set_color(BG_COLOR);
        
        let mut actions_pack = Pack::new(20, 60, 860, 580, "");
        actions_pack.set_type(PackType::Vertical);
        actions_pack.set_spacing(15);
        actions_pack.set_color(BG_COLOR);
        
        // Main actions
        let mut main_actions_frame = Frame::new(0, 0, 860, 30, "Main Actions:");
        main_actions_frame.set_align(Align::Left | Align::Inside);
        main_actions_frame.set_label_color(TEXT_COLOR);
        main_actions_frame.set_label_font(Font::HelveticaBold);
        
        let mut generate_btn = Button::new(0, 0, 200, 40, "Generate Config");
        generate_btn.set_color(SUCCESS_COLOR);
        generate_btn.set_label_color(TEXT_COLOR);
        
        let mut send_btn = Button::new(0, 0, 200, 40, "Send Config");
        send_btn.set_color(ACCENT_COLOR);
        send_btn.set_label_color(TEXT_COLOR);
        
        let mut browse_btn = Button::new(0, 0, 200, 40, "Browse OPC-UA");
        browse_btn.set_color(ACCENT_COLOR);
        browse_btn.set_label_color(TEXT_COLOR);
        
        // Other commands
        let mut other_actions_frame = Frame::new(0, 0, 860, 30, "Other Commands:");
        other_actions_frame.set_align(Align::Left | Align::Inside);
        other_actions_frame.set_label_color(TEXT_COLOR);
        other_actions_frame.set_label_font(Font::HelveticaBold);
        
        let mut backup_influx_btn = Button::new(0, 0, 200, 35, "Backup InfluxDB");
        backup_influx_btn.set_color(WIDGET_BG);
        backup_influx_btn.set_label_color(TEXT_COLOR);
        
        let mut backup_grafana_btn = Button::new(0, 0, 200, 35, "Backup Grafana");
        backup_grafana_btn.set_color(WIDGET_BG);
        backup_grafana_btn.set_label_color(TEXT_COLOR);
        
        let mut status_btn = Button::new(0, 0, 200, 35, "Telegraf Status");
        status_btn.set_color(WIDGET_BG);
        status_btn.set_label_color(TEXT_COLOR);
        
        let mut logs_btn = Button::new(0, 0, 200, 35, "Telegraf Logs");
        logs_btn.set_color(WIDGET_BG);
        logs_btn.set_label_color(TEXT_COLOR);
        
        let mut service_status_btn = Button::new(0, 0, 200, 35, "Service Status");
        service_status_btn.set_color(WIDGET_BG);
        service_status_btn.set_label_color(TEXT_COLOR);
        
        let mut restart_telegraf_btn = Button::new(0, 0, 200, 35, "Restart Telegraf");
        restart_telegraf_btn.set_color(ERROR_COLOR);
        restart_telegraf_btn.set_label_color(TEXT_COLOR);
        
        actions_pack.end();
        actions_group.end();
        
        // Status Tab
        let mut status_group = Group::new(10, 40, 880, 650, "Status");
        status_group.set_color(BG_COLOR);
        
        let mut status_buffer = TextBuffer::default();
        let mut status_display = TextDisplay::new(20, 50, 860, 550, "");
        status_display.set_buffer(status_buffer.clone());
        status_display.set_color(WIDGET_BG);
        status_display.set_text_color(TEXT_COLOR);
        status_display.wrap_mode(fltk::text::WrapMode::AtBounds, 0);
        
        let mut clear_status_btn = Button::new(20, 610, 150, 30, "Clear Status");
        clear_status_btn.set_color(WIDGET_BG);
        clear_status_btn.set_label_color(TEXT_COLOR);
        
        status_group.end();
        
        tabs.end();
        main_window.end();
        
        let mut app = Self {
            controller,
            sender,
            receiver,
            main_window,
            tabs,
            ip_input,
            iot_host_input,
            username_input,
            password_input,
            iot_username_input,
            iot_password_input,
            folder_input,
            include_test_inputs,
            output_format_prometheus,
            output_format_influx,
            anonymous_auth,
            xml_files_scroll,
            xml_files_group,
            file_checkboxes: Vec::new(),
            file_inputs: Vec::new(),
            generate_btn,
            send_btn,
            backup_influx_btn,
            backup_grafana_btn,
            status_btn,
            logs_btn,
            service_status_btn,
            restart_telegraf_btn,
            status_display,
            status_buffer,
            clear_status_btn,
            opcua_browser_window: None,
            browse_btn,
        };
        
        app.setup_callbacks();
        app.load_initial_data();
        
        app
    }
    
    fn setup_callbacks(&mut self) {
        let sender = self.sender.clone();
        let controller = self.controller.clone();
        
        // Folder browse button
        let mut folder_input = self.folder_input.clone();
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        
        // Generate config button
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.generate_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.generate_config();
            sender_clone.send(AppMessage::UpdateStatus("Config generation started...".to_string()));
        });
        
        // Send config button
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.send_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.send_config();
            sender_clone.send(AppMessage::UpdateStatus("Sending config...".to_string()));
        });
        
        // Browse OPC-UA button
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.browse_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.start_opcua_browsing();
            sender_clone.send(AppMessage::UpdateStatus("Starting OPC-UA browsing...".to_string()));
        });
        
        // Other action buttons
        self.setup_other_action_callbacks();
        
        // Clear status button
        let mut status_buffer = self.status_buffer.clone();
        self.clear_status_btn.set_callback(move |_| {
            status_buffer.remove(0, status_buffer.length());
        });
        
        // Anonymous auth checkbox
        let controller_clone = controller.clone();
        let mut username_input = self.username_input.clone();
        let mut password_input = self.password_input.clone();
        self.anonymous_auth.set_callback(move |cb| {
            let mut ctrl = controller_clone.lock().unwrap();
            if cb.is_checked() {
                ctrl.config.username.clear();
                ctrl.config.password.clear();
                username_input.set_value("");
                password_input.set_value("");
                username_input.deactivate();
                password_input.deactivate();
            } else {
                username_input.activate();
                password_input.activate();
            }
        });
        
        // Output format radio buttons
        let controller_clone = controller.clone();
        self.output_format_prometheus.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.config.output_format = Some("prometheus".to_string());
        });
        
        let controller_clone = controller.clone();
        self.output_format_influx.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.config.output_format = Some("influxdb".to_string());
        });
    }
    
    fn setup_other_action_callbacks(&mut self) {
        let sender = self.sender.clone();
        let controller = self.controller.clone();
        
        // Backup InfluxDB
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.backup_influx_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.backup_influxdb();
            sender_clone.send(AppMessage::UpdateStatus("Backing up InfluxDB...".to_string()));
        });
        
        // Backup Grafana
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.backup_grafana_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.backup_grafana();
            sender_clone.send(AppMessage::UpdateStatus("Backing up Grafana...".to_string()));
        });
        
        // Telegraf Status
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.status_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.get_telegraf_status();
            sender_clone.send(AppMessage::UpdateStatus("Getting Telegraf status...".to_string()));
        });
        
        // Telegraf Logs
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.logs_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.get_telegraf_logs();
            sender_clone.send(AppMessage::UpdateStatus("Getting Telegraf logs...".to_string()));
        });
        
        // Service Status
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.service_status_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.check_service_status();
            sender_clone.send(AppMessage::UpdateStatus("Checking service status...".to_string()));
        });
        
        // Restart Telegraf
        let sender_clone = sender.clone();
        let controller_clone = controller.clone();
        self.restart_telegraf_btn.set_callback(move |_| {
            let mut ctrl = controller_clone.lock().unwrap();
            ctrl.restart_telegraf();
            sender_clone.send(AppMessage::UpdateStatus("Restarting Telegraf...".to_string()));
        });
    }
    
    fn load_initial_data(&mut self) {
        {
            let ctrl = self.controller.lock().unwrap();
            
            // Load initial values from controller
            self.ip_input.set_value(&ctrl.config.ip);
            self.iot_host_input.set_value(&ctrl.config.iot_host);
            self.username_input.set_value(&ctrl.config.username);
            self.password_input.set_value(&ctrl.config.password);
            self.iot_username_input.set_value(&ctrl.config.iot_username);
            self.iot_password_input.set_value(&ctrl.config.iot_password);
            self.folder_input.set_value(&ctrl.config.folder.to_string_lossy());
            self.include_test_inputs.set_checked(ctrl.config.include_test_inputs);
            
            match ctrl.config.output_format.as_deref() {
                Some("prometheus") => self.output_format_prometheus.set_value(true),
                Some("influxdb") => self.output_format_influx.set_value(true),
                _ => self.output_format_prometheus.set_value(true), // default
            }
        } // Drop the lock here
        
        // Load XML files
        self.refresh_xml_files();
    }
    
    fn refresh_xml_files(&mut self) {
        let ctrl = self.controller.lock().unwrap();
        
        // Clear existing widgets
        self.xml_files_group.clear();
        self.file_checkboxes.clear();
        self.file_inputs.clear();
        
        let mut y_pos = 60;
        
        for (i, file) in ctrl.xml_files.iter().enumerate() {
            // File checkbox
            let mut checkbox = CheckButton::new(30, y_pos, 300, 25, file.as_str());
            checkbox.set_color(WIDGET_BG);
            checkbox.set_label_color(TEXT_COLOR);
            checkbox.set_checked(ctrl.selected_listener_files.get(i).copied().unwrap_or(false));
            
            // Configuration inputs for this file
            let mut namespace_input = Input::new(350, y_pos, 120, 25, "");
            namespace_input.set_color(WIDGET_BG);
            namespace_input.set_text_color(TEXT_COLOR);
            namespace_input.set_tooltip("Namespace");
            
            let mut interval_input = Input::new(480, y_pos, 80, 25, "");
            interval_input.set_color(WIDGET_BG);
            interval_input.set_text_color(TEXT_COLOR);
            interval_input.set_tooltip("Interval (ms)");
            
            let mut ip_input = Input::new(570, y_pos, 150, 25, "");
            ip_input.set_color(WIDGET_BG);
            ip_input.set_text_color(TEXT_COLOR);
            ip_input.set_tooltip("IP Address");
            
            // Load existing values if available
            if let Some(config) = ctrl.file_configs.get(file) {
                namespace_input.set_value(&config.namespace);
                interval_input.set_value(&config.interval_ms);
                ip_input.set_value(&config.ip);
            }
            
            self.xml_files_group.add(&checkbox);
            self.xml_files_group.add(&namespace_input);
            self.xml_files_group.add(&interval_input);
            self.xml_files_group.add(&ip_input);
            
            self.file_checkboxes.push(checkbox);
            self.file_inputs.push((namespace_input, interval_input, ip_input));
            
            y_pos += 35;
        }
        
        self.xml_files_group.redraw();
    }
    
    fn update_controller_from_ui(&mut self) {
        let mut ctrl = self.controller.lock().unwrap();
        
        // Update configuration values
        ctrl.config.ip = self.ip_input.value();
        ctrl.config.iot_host = self.iot_host_input.value();
        ctrl.config.username = self.username_input.value();
        ctrl.config.password = self.password_input.value();
        ctrl.config.iot_username = self.iot_username_input.value();
        ctrl.config.iot_password = self.iot_password_input.value();
        ctrl.config.include_test_inputs = self.include_test_inputs.is_checked();
        
        // Update file selections and configurations
        for (i, checkbox) in self.file_checkboxes.iter().enumerate() {
            if i < ctrl.selected_listener_files.len() {
                ctrl.selected_listener_files[i] = checkbox.is_checked();
            }
        }
        
        // Update file configurations separately to avoid borrowing issues
        for (i, (ns_input, int_input, ip_input)) in self.file_inputs.iter().enumerate() {
            if let Some(file) = ctrl.xml_files.get(i) {
                let file_name = file.clone();
                let config = ctrl.file_configs.entry(file_name).or_default();
                config.namespace = ns_input.value();
                config.interval_ms = int_input.value();
                config.ip = ip_input.value();
            }
        }
    }
    
    fn process_messages(&mut self) {
        while let Some(msg) = self.receiver.recv() {
            match msg {
                AppMessage::UpdateStatus(status) => {
                    self.status_buffer.append(&format!("{}\n", status));
                    self.status_display.scroll(1000000, 0); // Scroll to bottom
                }
                AppMessage::RefreshFiles => {
                    self.refresh_xml_files();
                }
                AppMessage::WorkerComplete => {
                    // Handle worker completion
                    let ctrl = self.controller.lock().unwrap();
                    for msg in &ctrl.status_messages {
                        self.status_buffer.append(&format!("{}\n", msg));
                    }
                    self.status_display.scroll(1000000, 0);
                }
                AppMessage::UpdateOpcUaNodes => {
                    // Handle OPC-UA node updates
                    self.sender.send(AppMessage::UpdateStatus("OPC-UA nodes updated".to_string()));
                }
            }
        }
    }
    
    pub fn run(&mut self) {
        self.main_window.show();
        
        // Start background worker thread
        let controller = self.controller.clone();
        let sender = self.sender.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(100));
                
                let needs_update = {
                    let mut ctrl = controller.lock().unwrap();
                    ctrl.process_worker_responses()
                };
                
                if needs_update {
                    let _ = sender.send(AppMessage::WorkerComplete);
                }
            }
        });
        
        // Main event loop
        while self.main_window.shown() {
            app::wait_for(0.1).unwrap();
            
            // Update controller from UI before processing
            self.update_controller_from_ui();
            
            // Process any pending messages
            self.process_messages();
        }
    }
}

fn main() {
    let mut app = TelegrafFltkApp::new();
    app.run();
}
