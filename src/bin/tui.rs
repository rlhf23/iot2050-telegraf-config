use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use sie_generate_config::{
    backend::ConfigGenerator,
    TelegrafConfig,
};
use std::{
    collections::HashMap,
    io,
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Files,
    OpcUaConfig,
    IoTConfig,
    Actions,
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, PartialEq)]
enum EditField {
    OpcUaIp,
    OpcUaUsername,
    OpcUaPassword,
    IoTHost,
    IoTUsername,
    IoTPassword,
    OutputFormat,
}

struct App {
    // Navigation
    current_tab: Tab,
    input_mode: InputMode,
    current_edit_field: Option<EditField>,
    
    // Configuration
    config: TelegrafConfig,
    
    // File management
    xml_files: Vec<String>,
    selected_files: Vec<bool>,
    file_list_state: ListState,
    
    // Status and messages
    status_messages: Vec<String>,
    show_help: bool,
    
    // Anonymous mode
    anonymous_mode: bool,
    
    // Temporary input buffer
    input_buffer: String,
}

impl App {
    fn new() -> App {
        let mut app = App {
            current_tab: Tab::Files,
            input_mode: InputMode::Normal,
            current_edit_field: None,
            config: TelegrafConfig {
                folder: PathBuf::from("."),
                ip: env!("DEFAULT_IP").to_string(),
                username: env!("DEFAULT_USERNAME").to_string(),
                password: env!("DEFAULT_PASSWORD").to_string(),
                iot_host: env!("DEFAULT_IOT_IP").to_string(),
                iot_username: env!("DEFAULT_IOT_USERNAME").to_string(),
                iot_password: env!("DEFAULT_IOT_PASSWORD").to_string(),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                selected_opcua_nodes: Vec::new(),
            },
            xml_files: Vec::new(),
            selected_files: Vec::new(),
            file_list_state: ListState::default(),
            status_messages: Vec::new(),
            show_help: false,
            anonymous_mode: false,
            input_buffer: String::new(),
        };
        
        app.load_xml_files();
        app
    }
    
    fn load_xml_files(&mut self) {
        self.xml_files = sie_generate_config::discover_xml_files(&self.config.folder);
        self.selected_files = vec![false; self.xml_files.len()];
        
        if self.xml_files.is_empty() {
            self.add_status_message("No XML files found in current directory".to_string());
        } else {
            self.add_status_message(format!("Found {} XML files", self.xml_files.len()));
        }
    }
    
    fn add_status_message(&mut self, message: String) {
        self.status_messages.push(message);
        if self.status_messages.len() > 10 {
            self.status_messages.remove(0);
        }
    }
    
    fn toggle_file_selection(&mut self) {
        if let Some(selected) = self.file_list_state.selected() {
            if selected < self.selected_files.len() {
                self.selected_files[selected] = !self.selected_files[selected];
            }
        }
    }
    
    fn toggle_anonymous_mode(&mut self) {
        self.anonymous_mode = !self.anonymous_mode;
        if self.anonymous_mode {
            self.add_status_message("Anonymous mode enabled".to_string());
        } else {
            self.add_status_message("Anonymous mode disabled".to_string());
        }
    }
    
    fn generate_config(&mut self) {
        // Update config based on anonymous mode
        if self.anonymous_mode {
            self.config.username = String::new();
            self.config.password = String::new();
        }
        
        // Get selected files
        let selected_xml_files: Vec<String> = self.xml_files
            .iter()
            .enumerate()
            .filter(|(i, _)| *self.selected_files.get(*i).unwrap_or(&false))
            .map(|(_, file)| file.clone())
            .collect();
        
        if selected_xml_files.is_empty() && !self.config.include_test_inputs {
            self.add_status_message("No files selected and test inputs not enabled".to_string());
            return;
        }
        
        match ConfigGenerator::new(self.config.clone()) {
            Ok(mut generator) => {
                // Configure each selected file with default settings
                for file in &selected_xml_files {
                    generator.set_file_config(file.clone(), "2".to_string(), 500, None);
                }
                
                match generator.generate_config(&selected_xml_files, &Vec::new()) {
                    Ok(_) => {
                        self.add_status_message("Configuration generated successfully!".to_string());
                    }
                    Err(e) => {
                        self.add_status_message(format!("Failed to generate config: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_status_message(format!("Failed to create generator: {}", e));
            }
        }
    }
    
    fn send_config(&mut self) {
        if self.config.iot_host.is_empty() {
            self.add_status_message("IoT host not configured".to_string());
            return;
        }
        
        match ConfigGenerator::new(self.config.clone()) {
            Ok(generator) => {
                match generator.send_config() {
                    Ok(_) => {
                        self.add_status_message("Configuration sent successfully!".to_string());
                    }
                    Err(e) => {
                        self.add_status_message(format!("Failed to send config: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_status_message(format!("Failed to create generator: {}", e));
            }
        }
    }
    
    fn start_editing(&mut self, field: EditField) {
        self.input_mode = InputMode::Editing;
        self.current_edit_field = Some(field.clone());
        
        // Pre-populate input buffer with current value
        self.input_buffer = match field {
            EditField::OpcUaIp => self.config.ip.clone(),
            EditField::OpcUaUsername => self.config.username.clone(),
            EditField::OpcUaPassword => self.config.password.clone(),
            EditField::IoTHost => self.config.iot_host.clone(),
            EditField::IoTUsername => self.config.iot_username.clone(),
            EditField::IoTPassword => self.config.iot_password.clone(),
            EditField::OutputFormat => self.config.output_format.as_ref().unwrap_or(&"influxdb".to_string()).clone(),
        };
    }
    
    fn finish_editing(&mut self) {
        if let Some(field) = &self.current_edit_field {
            match field {
                EditField::OpcUaIp => self.config.ip = self.input_buffer.clone(),
                EditField::OpcUaUsername => self.config.username = self.input_buffer.clone(),
                EditField::OpcUaPassword => self.config.password = self.input_buffer.clone(),
                EditField::IoTHost => self.config.iot_host = self.input_buffer.clone(),
                EditField::IoTUsername => self.config.iot_username = self.input_buffer.clone(),
                EditField::IoTPassword => self.config.iot_password = self.input_buffer.clone(),
                EditField::OutputFormat => self.config.output_format = Some(self.input_buffer.clone()),
            }
        }
        
        self.input_mode = InputMode::Normal;
        self.current_edit_field = None;
        self.input_buffer.clear();
    }
    
    fn cancel_editing(&mut self) {
        self.input_mode = InputMode::Normal;
        self.current_edit_field = None;
        self.input_buffer.clear();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') | KeyCode::F(1) => app.show_help = !app.show_help,
                        KeyCode::Tab => {
                            app.current_tab = match app.current_tab {
                                Tab::Files => Tab::OpcUaConfig,
                                Tab::OpcUaConfig => Tab::IoTConfig,
                                Tab::IoTConfig => Tab::Actions,
                                Tab::Actions => Tab::Files,
                            };
                        }
                        KeyCode::Char('1') => app.current_tab = Tab::Files,
                        KeyCode::Char('2') => app.current_tab = Tab::OpcUaConfig,
                        KeyCode::Char('3') => app.current_tab = Tab::IoTConfig,
                        KeyCode::Char('4') => app.current_tab = Tab::Actions,
                        _ => {
                            match app.current_tab {
                                Tab::Files => handle_files_input(&mut app, key.code),
                                Tab::OpcUaConfig => handle_opcua_input(&mut app, key.code),
                                Tab::IoTConfig => handle_iot_input(&mut app, key.code),
                                Tab::Actions => handle_actions_input(&mut app, key.code),
                            }
                        }
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => app.finish_editing(),
                        KeyCode::Esc => app.cancel_editing(),
                        KeyCode::Char(c) => app.input_buffer.push(c),
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

fn handle_files_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            let i = match app.file_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        app.xml_files.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            app.file_list_state.select(Some(i));
        }
        KeyCode::Down => {
            let i = match app.file_list_state.selected() {
                Some(i) => {
                    if i >= app.xml_files.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            app.file_list_state.select(Some(i));
        }
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_file_selection(),
        KeyCode::Char('r') => app.load_xml_files(),
        _ => {}
    }
}

fn handle_opcua_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('i') => app.start_editing(EditField::OpcUaIp),
        KeyCode::Char('u') => app.start_editing(EditField::OpcUaUsername),
        KeyCode::Char('p') => app.start_editing(EditField::OpcUaPassword),
        KeyCode::Char('o') => app.start_editing(EditField::OutputFormat),
        KeyCode::Char('a') => app.toggle_anonymous_mode(),
        KeyCode::Char('t') => app.config.include_test_inputs = !app.config.include_test_inputs,
        _ => {}
    }
}

fn handle_iot_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('h') => app.start_editing(EditField::IoTHost),
        KeyCode::Char('u') => app.start_editing(EditField::IoTUsername),
        KeyCode::Char('p') => app.start_editing(EditField::IoTPassword),
        _ => {}
    }
}

fn handle_actions_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('g') => app.generate_config(),
        KeyCode::Char('s') => app.send_config(),
        KeyCode::Char('c') => app.status_messages.clear(),
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),     // Main content
            Constraint::Length(6),  // Status messages
        ])
        .split(f.area());

    // Render tabs
    let tab_titles = vec!["Files", "OPC-UA Config", "IoT Config", "Actions"];
    let selected_tab = match app.current_tab {
        Tab::Files => 0,
        Tab::OpcUaConfig => 1,
        Tab::IoTConfig => 2,
        Tab::Actions => 3,
    };
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("IoT2050 Config TUI"))
        .select(selected_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    f.render_widget(tabs, chunks[0]);

    // Render main content based on current tab
    match app.current_tab {
        Tab::Files => render_files_tab(f, app, chunks[1]),
        Tab::OpcUaConfig => render_opcua_tab(f, app, chunks[1]),
        Tab::IoTConfig => render_iot_tab(f, app, chunks[1]),
        Tab::Actions => render_actions_tab(f, app, chunks[1]),
    }

    // Render status messages
    render_status_messages(f, app, chunks[2]);

    // Render help popup if needed
    if app.show_help {
        render_help_popup(f, app);
    }
}

fn render_files_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // File list
    let items: Vec<ListItem> = app
        .xml_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let checkbox = if *app.selected_files.get(i).unwrap_or(&false) {
                "[x]"
            } else {
                "[ ]"
            };
            let filename = std::path::Path::new(file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file);
            ListItem::new(format!("{} {}", checkbox, filename))
        })
        .collect();

    let files_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("XML Files"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(files_list, chunks[0], &mut app.file_list_state);

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from(""),
        Line::from("↑/↓  - Navigate files"),
        Line::from("Space - Toggle selection"),
        Line::from("r    - Refresh file list"),
        Line::from("Tab  - Next tab"),
        Line::from("h/F1 - Help"),
        Line::from("q    - Quit"),
    ];

    let help_block = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(help_block, chunks[1]);
}

fn render_opcua_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Configuration fields
    let config_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunks[0]);

    // Render each field
    render_config_field(f, "OPC-UA Server IP", &app.config.ip, 
                       matches!(app.current_edit_field, Some(EditField::OpcUaIp)), 
                       &app.input_buffer, config_chunks[0]);

    let username_display = if app.anonymous_mode { 
        "[Anonymous Mode]".to_string() 
    } else { 
        app.config.username.clone() 
    };
    render_config_field(f, "Username", &username_display, 
                       matches!(app.current_edit_field, Some(EditField::OpcUaUsername)), 
                       &app.input_buffer, config_chunks[1]);

    let password_display = if app.anonymous_mode { 
        "[Anonymous Mode]".to_string() 
    } else { 
        "*".repeat(app.config.password.len()) 
    };
    render_config_field(f, "Password", &password_display, 
                       matches!(app.current_edit_field, Some(EditField::OpcUaPassword)), 
                       &app.input_buffer, config_chunks[2]);

    render_config_field(f, "Output Format", 
                       app.config.output_format.as_ref().unwrap_or(&"influxdb".to_string()), 
                       matches!(app.current_edit_field, Some(EditField::OutputFormat)), 
                       &app.input_buffer, config_chunks[3]);

    let anonymous_status = if app.anonymous_mode { "Enabled" } else { "Disabled" };
    render_config_field(f, "Anonymous Mode", anonymous_status, false, "", config_chunks[4]);

    let test_inputs_status = if app.config.include_test_inputs { "Enabled" } else { "Disabled" };
    render_config_field(f, "Test Inputs", test_inputs_status, false, "", config_chunks[5]);

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from(""),
        Line::from("i - Edit IP address"),
        Line::from("u - Edit username"),
        Line::from("p - Edit password"),
        Line::from("o - Edit output format"),
        Line::from("a - Toggle anonymous mode"),
        Line::from("t - Toggle test inputs"),
        Line::from(""),
        Line::from("Enter - Confirm edit"),
        Line::from("Esc   - Cancel edit"),
    ];

    let help_block = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(help_block, chunks[1]);
}

fn render_iot_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Configuration fields
    let config_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunks[0]);

    render_config_field(f, "IoT Device Host", &app.config.iot_host, 
                       matches!(app.current_edit_field, Some(EditField::IoTHost)), 
                       &app.input_buffer, config_chunks[0]);

    render_config_field(f, "IoT Username", &app.config.iot_username, 
                       matches!(app.current_edit_field, Some(EditField::IoTUsername)), 
                       &app.input_buffer, config_chunks[1]);

    let password_display = "*".repeat(app.config.iot_password.len());
    render_config_field(f, "IoT Password", &password_display, 
                       matches!(app.current_edit_field, Some(EditField::IoTPassword)), 
                       &app.input_buffer, config_chunks[2]);

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from(""),
        Line::from("h - Edit IoT host"),
        Line::from("u - Edit IoT username"),
        Line::from("p - Edit IoT password"),
        Line::from(""),
        Line::from("Enter - Confirm edit"),
        Line::from("Esc   - Cancel edit"),
    ];

    let help_block = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(help_block, chunks[1]);
}

fn render_actions_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Action buttons
    let actions = vec![
        Line::from("Available Actions:"),
        Line::from(""),
        Line::from("g - Generate Configuration"),
        Line::from("s - Send Config to IoT Device"),
        Line::from("c - Clear Status Messages"),
    ];

    let actions_block = Paragraph::new(actions)
        .block(Block::default().borders(Borders::ALL).title("Actions"));
    f.render_widget(actions_block, chunks[0]);

    // Current configuration summary
    let selected_count = app.selected_files.iter().filter(|&&x| x).count();
    let auth_mode = if app.anonymous_mode { "Anonymous" } else { "Username/Password" };
    
    let summary = vec![
        Line::from("Current Configuration:"),
        Line::from(""),
        Line::from(format!("Selected Files: {}", selected_count)),
        Line::from(format!("OPC-UA Server: {}", app.config.ip)),
        Line::from(format!("Authentication: {}", auth_mode)),
        Line::from(format!("IoT Device: {}", app.config.iot_host)),
        Line::from(format!("Output Format: {}", 
                          app.config.output_format.as_ref().unwrap_or(&"influxdb".to_string()))),
        Line::from(format!("Test Inputs: {}", 
                          if app.config.include_test_inputs { "Yes" } else { "No" })),
    ];

    let summary_block = Paragraph::new(summary)
        .block(Block::default().borders(Borders::ALL).title("Summary"));
    f.render_widget(summary_block, chunks[1]);
}

fn render_config_field(
    f: &mut Frame, 
    label: &str, 
    value: &str, 
    is_editing: bool, 
    input_buffer: &str, 
    area: Rect
) {
    let display_value = if is_editing {
        format!("{}_", input_buffer)
    } else {
        value.to_string()
    };

    let style = if is_editing {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(display_value)
        .block(Block::default().borders(Borders::ALL).title(label))
        .style(style);
    
    f.render_widget(paragraph, area);
}

fn render_status_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages: Vec<ListItem> = app
        .status_messages
        .iter()
        .map(|m| ListItem::new(m.as_str()))
        .collect();

    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Status Messages"));

    f.render_widget(messages_list, area);
}

fn render_help_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(60, 70, f.area());
    
    let help_text = vec![
        Line::from("IoT2050 Configuration TUI - Help"),
        Line::from(""),
        Line::from("Global Controls:"),
        Line::from("  Tab/1-4 - Switch between tabs"),
        Line::from("  h/F1    - Toggle this help"),
        Line::from("  q       - Quit application"),
        Line::from(""),
        Line::from("Files Tab:"),
        Line::from("  ↑/↓     - Navigate file list"),
        Line::from("  Space   - Toggle file selection"),
        Line::from("  r       - Refresh file list"),
        Line::from(""),
        Line::from("Config Tabs:"),
        Line::from("  Letters - Edit corresponding fields"),
        Line::from("  Enter   - Confirm edit"),
        Line::from("  Esc     - Cancel edit"),
        Line::from(""),
        Line::from("Actions Tab:"),
        Line::from("  g       - Generate configuration"),
        Line::from("  s       - Send config to IoT device"),
        Line::from("  c       - Clear status messages"),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    f.render_widget(Clear, popup_area);
    f.render_widget(help_paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
