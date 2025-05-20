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
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame, Terminal,
};
use sie_generate_config::{error, TelegrafConfig};
use std::{
    collections::HashMap,
    env, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(PartialEq)]
enum AppTab {
    Configuration,
    XmlFiles,
    Generation,
    Sending,
    Backup,
}

struct App {
    tabs: Vec<String>,
    active_tab: usize,
    input_mode: InputMode,
    config: TelegrafConfig,
    edit_field: usize,
    field_values: Vec<String>,
    field_names: Vec<String>,
    field_descriptions: Vec<String>,
    message: String,
    xml_files: Vec<String>,
    selected_xml_file: Option<usize>,
    file_configs: HashMap<String, error::XmlFileValidation>,
    show_help: bool,
}

impl App {
    fn new() -> Self {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let default_folder = {
            let mut path = current_exe.clone();
            path.pop();
            path
        };

        let config = TelegrafConfig {
            folder: default_folder.clone(),
            ip: env!("DEFAULT_IP").to_string(),
            username: env!("DEFAULT_USERNAME").to_string(),
            password: env!("DEFAULT_PASSWORD").to_string(),
            iot_host: env!("DEFAULT_IOT_IP").to_string(),
            iot_username: env!("DEFAULT_IOT_USERNAME").to_string(),
            iot_password: env!("DEFAULT_IOT_PASSWORD").to_string(),
            token_folder: default_folder,
            bucket_name: "telegraf".to_string(),
            influx_token: None,
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
        };

        // Field names and values for editing
        let field_names = vec![
            "Folder".to_string(),
            "IP".to_string(),
            "Username".to_string(),
            "Password".to_string(),
            "IoT Host".to_string(),
            "IoT Username".to_string(),
            "IoT Password".to_string(),
            "Bucket Name".to_string(),
            "Output Format".to_string(),
            "Include Test Inputs".to_string(),
        ];

        let field_values = vec![
            config.folder.to_string_lossy().to_string(),
            config.ip.clone(),
            config.username.clone(),
            config.password.clone(),
            config.iot_host.clone(),
            config.iot_username.clone(),
            config.iot_password.clone(),
            config.bucket_name.clone(),
            config.output_format.clone().unwrap_or_default(),
            config.include_test_inputs.to_string(),
        ];

        let field_descriptions = vec![
            "Folder containing XML files".to_string(),
            "OPC IP address".to_string(),
            "OPC username".to_string(),
            "OPC password".to_string(),
            "IoT-2050 host address and port".to_string(),
            "IoT-2050 username".to_string(),
            "IoT-2050 password".to_string(),
            "InfluxDB bucket name".to_string(),
            "Output format (influxdb or prometheus)".to_string(),
            "Include test inputs (true/false)".to_string(),
        ];

        // Load XML files
        let xml_files = App::discover_xml_files(&config.folder);

        App {
            tabs: vec![
                "Configuration".to_string(),
                "XML Files".to_string(),
                "Generate Config".to_string(),
                "Send Config".to_string(),
                "Backup".to_string(),
            ],
            active_tab: 0,
            input_mode: InputMode::Normal,
            config,
            edit_field: 0,
            field_values,
            field_names,
            field_descriptions,
            message: "Welcome to IOT2050 Telegraf Config TUI".to_string(),
            xml_files,
            selected_xml_file: None,
            file_configs: HashMap::new(),
            show_help: false,
        }
    }

    fn update_config(&mut self) {
        self.config.folder = PathBuf::from(&self.field_values[0]);
        self.config.ip = self.field_values[1].clone();
        self.config.username = self.field_values[2].clone();
        self.config.password = self.field_values[3].clone();
        self.config.iot_host = self.field_values[4].clone();
        self.config.iot_username = self.field_values[5].clone();
        self.config.iot_password = self.field_values[6].clone();
        self.config.bucket_name = self.field_values[7].clone();
        self.config.output_format = Some(self.field_values[8].clone());
        self.config.include_test_inputs = self.field_values[9].parse().unwrap_or(false);
    }

    fn discover_xml_files(folder: &Path) -> Vec<String> {
        match std::fs::read_dir(folder) {
            Ok(entries) => {
                let mut xml_files = Vec::new();
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "xml" {
                            if let Some(file_name) = path.file_name() {
                                if let Some(file_name_str) = file_name.to_str() {
                                    xml_files.push(file_name_str.to_string());
                                }
                            }
                        }
                    }
                }
                xml_files.sort();
                xml_files
            }
            Err(_) => Vec::new(),
        }
    }

    fn refresh_xml_files(&mut self) {
        self.xml_files = App::discover_xml_files(&self.config.folder);
        if self.selected_xml_file.is_some()
            && self.selected_xml_file.unwrap() >= self.xml_files.len()
        {
            self.selected_xml_file = None;
        }
    }

    fn handle_key_press(&mut self, key: KeyCode) {
        match self.input_mode {
            InputMode::Normal => match key {
                KeyCode::Char('q') => {
                    if self.show_help {
                        self.show_help = false;
                    } else {
                        // Exit the application
                        // This will be handled in the run function
                    }
                }
                KeyCode::Char('h') => {
                    self.show_help = !self.show_help;
                }
                KeyCode::Char('e') => {
                    if !self.show_help {
                        self.input_mode = InputMode::Editing;
                    }
                }
                KeyCode::Tab => {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    self.selected_xml_file = None;
                }
                KeyCode::BackTab => {
                    self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
                    self.selected_xml_file = None;
                }
                KeyCode::Char('1') => {
                    self.active_tab = 0;
                    self.selected_xml_file = None;
                }
                KeyCode::Char('2') => {
                    self.active_tab = 1;
                    self.refresh_xml_files();
                }
                KeyCode::Char('3') => {
                    self.active_tab = 2;
                    self.message = "Generating config...".to_string();
                    self.update_config();
                    // Code to generate config would go here
                }
                KeyCode::Char('4') => {
                    self.active_tab = 3;
                    self.update_config();
                    // Code to prepare for sending config would go here
                }
                KeyCode::Char('5') => {
                    self.active_tab = 4;
                    self.update_config();
                    // Code to prepare for backup would go here
                }
                KeyCode::Down => match self.active_tab {
                    0 => {
                        self.edit_field = (self.edit_field + 1) % self.field_names.len();
                    }
                    1 => {
                        if !self.xml_files.is_empty() {
                            let new_index = match self.selected_xml_file {
                                Some(idx) => (idx + 1) % self.xml_files.len(),
                                None => 0,
                            };
                            self.selected_xml_file = Some(new_index);
                        }
                    }
                    _ => {}
                },
                KeyCode::Up => match self.active_tab {
                    0 => {
                        self.edit_field =
                            (self.edit_field + self.field_names.len() - 1) % self.field_names.len();
                    }
                    1 => {
                        if !self.xml_files.is_empty() {
                            let new_index = match self.selected_xml_file {
                                Some(idx) => {
                                    (idx + self.xml_files.len() - 1) % self.xml_files.len()
                                }
                                None => self.xml_files.len() - 1,
                            };
                            self.selected_xml_file = Some(new_index);
                        }
                    }
                    _ => {}
                },
                KeyCode::Enter => {
                    match self.active_tab {
                        0 => {
                            self.input_mode = InputMode::Editing;
                        }
                        1 => {
                            // View details of selected XML file
                            if let Some(idx) = self.selected_xml_file {
                                if idx < self.xml_files.len() {
                                    let filename = &self.xml_files[idx];
                                    self.message = format!("Selected XML file: {}", filename);
                                    // Code to parse and display XML details would go here
                                }
                            }
                        }
                        2 => {
                            // Generate config
                            self.update_config();
                            self.message = "Generating Telegraf config...".to_string();
                            // Code to generate config would go here
                        }
                        3 => {
                            // Send config
                            self.update_config();
                            self.message = "Sending Telegraf config to IoT device...".to_string();
                            // Code to send config would go here
                        }
                        4 => {
                            // Backup
                            self.update_config();
                            self.message = "Starting backup...".to_string();
                            // Code to perform backup would go here
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            InputMode::Editing => match key {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                    self.update_config();
                    self.message = "Configuration updated".to_string();
                }
                KeyCode::Char(c) => {
                    self.field_values[self.edit_field].push(c);
                }
                KeyCode::Backspace => {
                    self.field_values[self.edit_field].pop();
                }
                KeyCode::Down => {
                    self.edit_field = (self.edit_field + 1) % self.field_names.len();
                }
                KeyCode::Up => {
                    self.edit_field =
                        (self.edit_field + self.field_names.len() - 1) % self.field_names.len();
                }
                _ => {}
            },
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Create layout for tabs and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(size);

    // Create tabs
    let tab_titles = app
        .tabs
        .iter()
        .map(|t| {
            let (first, rest) = t.split_at(1);
            Line::from(vec![
                Span::styled(first, Style::default().fg(Color::Yellow)),
                Span::styled(rest, Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .select(app.active_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, chunks[0]);

    // Create a block for the status area
    let status_block = Block::default().borders(Borders::ALL).title("Status");

    let status_text = Paragraph::new(app.message.clone())
        .block(status_block)
        .style(Style::default().fg(Color::White));

    f.render_widget(status_text, chunks[2]);

    // Content
    match app.active_tab {
        0 => {
            // Configuration tab
            let config_chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[1]);

            let mut list_items = Vec::new();
            for i in 0..app.field_names.len() {
                let style = if i == app.edit_field {
                    match app.input_mode {
                        InputMode::Normal => Style::default().fg(Color::Yellow),
                        InputMode::Editing => Style::default().fg(Color::Green),
                    }
                } else {
                    Style::default()
                };

                let field_name = &app.field_names[i];
                let field_value = &app.field_values[i];
                let field_desc = &app.field_descriptions[i];

                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("{}: ", field_name), style),
                    Span::styled(field_value.clone(), style),
                    Span::styled(
                        format!(" ({})", field_desc),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])));
            }

            let title = match app.input_mode {
                InputMode::Normal => "Configuration (Press 'e' to edit)",
                InputMode::Editing => "Editing Configuration (Esc to stop, Enter to save)",
            };

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            f.render_widget(list, config_chunk[0]);
        }
        1 => {
            // XML Files tab
            let xml_chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[1]);

            let mut list_items = Vec::new();
            for (i, file) in app.xml_files.iter().enumerate() {
                let style = if Some(i) == app.selected_xml_file {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                list_items.push(ListItem::new(Line::from(vec![Span::styled(
                    file.clone(),
                    style,
                )])));
            }

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title("XML Files"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            f.render_widget(list, xml_chunk[0]);
        }
        2 => {
            // Generate Config tab
            let gen_chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[1]);

            let text = Paragraph::new(
                "Press Enter to generate Telegraf configuration from XML files.\n\
                This will read all XML files from the configured folder and generate a telegraf.conf file."
            )
            .block(Block::default().borders(Borders::ALL).title("Generate Config"))
            .style(Style::default().fg(Color::White));

            f.render_widget(text, gen_chunk[0]);
        }
        3 => {
            // Send Config tab
            let send_chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[1]);

            let text = Paragraph::new(
                "Press Enter to send the Telegraf configuration to the IoT device.\n\
                This will upload the telegraf.conf file to the configured IoT device using SSH.",
            )
            .block(Block::default().borders(Borders::ALL).title("Send Config"))
            .style(Style::default().fg(Color::White));

            f.render_widget(text, send_chunk[0]);
        }
        4 => {
            // Backup tab
            let backup_chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[1]);

            let text = Paragraph::new(
                "Press Enter to back up the InfluxDB and Grafana configurations from the IoT device.\n\
                This will download the configurations and save them to the current working directory."
            )
            .block(Block::default().borders(Borders::ALL).title("Backup"))
            .style(Style::default().fg(Color::White));

            f.render_widget(text, backup_chunk[0]);
        }
        _ => {}
    }

    // Show help overlay if requested
    if app.show_help {
        let help_area = centered_rect(70, 70, size);
        let help_text = vec![
            Line::from("HELP - Key Bindings"),
            Line::from(""),
            Line::from("q - Quit (close help if open)"),
            Line::from("h - Toggle help"),
            Line::from("Tab/1-5 - Switch tabs"),
            Line::from("↑/↓ - Navigate items"),
            Line::from("Enter - Select/Edit"),
            Line::from("e - Edit selected field"),
            Line::from("Esc - Cancel editing"),
            Line::from(""),
            Line::from("Press q to close this help window"),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White));

        f.render_widget(help, help_area);
    }
}

// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q')
                            if app.input_mode == InputMode::Normal && !app.show_help =>
                        {
                            break;
                        }
                        _ => app.handle_key_press(key.code),
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let tick_rate = Duration::from_millis(100);
    let res = run_app(&mut terminal, app, tick_rate);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
        return Err(err);
    }

    Ok(())
}
