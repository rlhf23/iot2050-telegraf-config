use iced::widget::{button, checkbox, column, container, scrollable, text, text_input, Space};
use iced::{Element, Task, Length, Color, Background, Border, Shadow, Vector};

fn main() -> iced::Result {
    iced::run("IoT2050 Config Tool", ConfigApp::update, ConfigApp::view)
}

#[derive(Debug, Clone)]
pub enum Message {
    // File operations
    RefreshFiles,
    ToggleFileSelection(usize),
    
    // OPC-UA config
    IpChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    ToggleAnonymous,
    
    // IoT config
    IoTIpChanged(String),
    IoTUsernameChanged(String),
    IoTPasswordChanged(String),
    
    // Actions
    GenerateConfig,
    SendConfig,
    OpenOpcUaBrowser,
    
    // Modal/popup
    CloseModal,
    ModalAction(String),
}

#[derive(Default)]
struct ConfigApp {
    // File management
    xml_files: Vec<String>,
    selected_files: Vec<bool>,
    
    // OPC-UA config
    ip: String,
    username: String,
    password: String,
    anonymous: bool,
    
    // IoT config
    iot_ip: String,
    iot_username: String,
    iot_password: String,
    
    // UI state
    status: String,
    show_modal: bool,
    modal_content: String,
}

impl ConfigApp {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // File operations
            Message::RefreshFiles => {
                // Mock file discovery for now
                self.xml_files = vec![
                    "config1.xml".to_string(),
                    "config2.xml".to_string(),
                    "sensors.xml".to_string(),
                ];
                self.selected_files = vec![false; self.xml_files.len()];
                self.status = format!("Found {} XML files", self.xml_files.len());
                Task::none()
            }
            Message::ToggleFileSelection(index) => {
                if let Some(selected) = self.selected_files.get_mut(index) {
                    *selected = !*selected;
                }
                Task::none()
            }
            
            // OPC-UA config
            Message::IpChanged(value) => {
                self.ip = value;
                Task::none()
            }
            Message::UsernameChanged(value) => {
                self.username = value;
                Task::none()
            }
            Message::PasswordChanged(value) => {
                self.password = value;
                Task::none()
            }
            Message::ToggleAnonymous => {
                self.anonymous = !self.anonymous;
                if self.anonymous {
                    self.username.clear();
                    self.password.clear();
                }
                Task::none()
            }
            
            // IoT config
            Message::IoTIpChanged(value) => {
                self.iot_ip = value;
                Task::none()
            }
            Message::IoTUsernameChanged(value) => {
                self.iot_username = value;
                Task::none()
            }
            Message::IoTPasswordChanged(value) => {
                self.iot_password = value;
                Task::none()
            }
            
            // Actions
            Message::GenerateConfig => {
                let selected_count = self.selected_files.iter().filter(|&&x| x).count();
                self.status = format!("✅ Config generated for {} files, IP: {} ({})", 
                    selected_count,
                    self.ip, 
                    if self.anonymous { "Anonymous" } else { "Authenticated" }
                );
                Task::none()
            }
            Message::SendConfig => {
                self.status = "📤 Sending configuration to IoT device...".to_string();
                Task::none()
            }
            Message::OpenOpcUaBrowser => {
                self.show_modal = true;
                self.modal_content = "OPC-UA Browser\n\nConnecting to server...\n\nRoot\n  Objects\n    Temperature\n    Pressure\n    Flow Rate".to_string();
                Task::none()
            }
            
            // Modal/popup
            Message::CloseModal => {
                self.show_modal = false;
                Task::none()
            }
            Message::ModalAction(action) => {
                self.status = format!("Modal action: {}", action);
                self.show_modal = false;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let main_content = self.view_main_content();
        
        if self.show_modal {
            // Stack modal on top of main content
            container(
                column![
                    // Background content (dimmed)
                    container(main_content)
                        .style(|_theme| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.3))),
                            ..Default::default()
                        }),
                    // Modal overlay positioned on top
                    container(self.view_modal_content())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .style(|_theme| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                ]
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            main_content
        }
    }
    
    fn view_main_content(&self) -> Element<Message> {
        let mut content = vec![
            // Header
            text("IoT2050 Config Tool").size(20).into(),
            text("Configure OPC-UA monitoring for industrial devices").size(12).into(),
            Space::with_height(15).into(),
            
            // File Selection Section
            text("XML Configuration Files").size(14).into(),
            Space::with_height(5).into(),
        ];
        
        // Add file list or refresh button
        if self.xml_files.is_empty() {
            content.push(
                button("Discover XML Files")
                    .on_press(Message::RefreshFiles)
                    .padding([6, 12])
                    .width(Length::Fixed(200.0))
                    .into()
            );
        } else {
            for (i, file) in self.xml_files.iter().enumerate() {
                let is_selected = self.selected_files.get(i).copied().unwrap_or(false);
                content.push(
                    checkbox(file, is_selected)
                        .on_toggle(move |_| Message::ToggleFileSelection(i))
                        .into()
                );
            }
        }
        
        content.extend([
            Space::with_height(15).into(),
            
            // OPC-UA Configuration Section
            text("OPC-UA Server Configuration").size(14).into(),
            Space::with_height(5).into(),
            text("Server IP Address:").size(11).into(),
            text_input("Enter OPC-UA Server IP", &self.ip)
                .on_input(Message::IpChanged)
                .padding(6)
                .into(),
            Space::with_height(8).into(),
            checkbox("Anonymous Authentication", self.anonymous)
                .on_toggle(|_| Message::ToggleAnonymous)
                .into(),
        ]);

        if !self.anonymous {
            content.extend([
                Space::with_height(8).into(),
                text("Username:").size(11).into(),
                text_input("Username", &self.username)
                    .on_input(Message::UsernameChanged)
                    .padding(6)
                    .into(),
                Space::with_height(8).into(),
                text("Password:").size(11).into(),
                text_input("Password", &self.password)
                    .on_input(Message::PasswordChanged)
                    .secure(true)
                    .padding(6)
                    .into(),
            ]);
        }
        
        content.extend([
            Space::with_height(8).into(),
            button("Browse OPC-UA Nodes")
                .on_press(Message::OpenOpcUaBrowser)
                .padding([6, 12])
                .width(Length::Fixed(200.0))
                .into(),
            Space::with_height(15).into(),
            
            // IoT Device Configuration Section
            text("IoT Device Configuration").size(14).into(),
            Space::with_height(5).into(),
            text("IoT Device IP:").size(11).into(),
            text_input("Enter IoT device IP", &self.iot_ip)
                .on_input(Message::IoTIpChanged)
                .padding(6)
                .into(),
            Space::with_height(8).into(),
            text("IoT Username:").size(11).into(),
            text_input("IoT Username", &self.iot_username)
                .on_input(Message::IoTUsernameChanged)
                .padding(6)
                .into(),
            Space::with_height(8).into(),
            text("IoT Password:").size(11).into(),
            text_input("IoT Password", &self.iot_password)
                .on_input(Message::IoTPasswordChanged)
                .secure(true)
                .padding(6)
                .into(),
            Space::with_height(20).into(),
            
            // Actions Section
            text("Actions").size(14).into(),
            Space::with_height(5).into(),
            button("Generate Configuration")
                .on_press(Message::GenerateConfig)
                .padding([8, 16])
                .width(Length::Fixed(200.0))
                .into(),
            Space::with_height(8).into(),
            button("Send to IoT Device")
                .on_press(Message::SendConfig)
                .padding([8, 16])
                .width(Length::Fixed(200.0))
                .into(),
            Space::with_height(15).into(),
            text(if self.status.is_empty() { 
                "Ready to configure..." 
            } else { 
                &self.status 
            }).size(12).into(),
        ]);

        let main_column = column(content)
            .spacing(3)
            .padding(20)
            .max_width(450);

        // Wrap in scrollable container
        let scrollable_content = scrollable(main_column);

        container(scrollable_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }
    
    fn view_modal_content(&self) -> Element<Message> {
        // Create modal content box
        let modal_box = column![
            text("OPC-UA Browser").size(16),
            Space::with_height(10),
            text(&self.modal_content).size(11),
            Space::with_height(15),
            button("Select Node")
                .on_press(Message::ModalAction("select".to_string()))
                .padding([6, 12])
                .width(Length::Fixed(150.0)),
            Space::with_height(8),
            button("Close")
                .on_press(Message::CloseModal)
                .padding([6, 12])
                .width(Length::Fixed(150.0)),
        ]
        .spacing(5)
        .padding(20)
        .max_width(350);
        
        // White modal box with shadow effect
        container(modal_box)
            .padding(0)
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                border: Border::default(),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 10.0,
                },
                ..Default::default()
            })
            .into()
    }
}
