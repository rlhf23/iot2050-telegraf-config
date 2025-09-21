use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Element, Task, Theme};

fn main() -> iced::Result {
    iced::run("IoT2050 Config Tool", ConfigApp::update, ConfigApp::view)
}

#[derive(Debug, Clone)]
pub enum Message {
    IpChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    ToggleAnonymous,
    GenerateConfig,
}

#[derive(Default)]
struct ConfigApp {
    ip: String,
    username: String,
    password: String,
    anonymous: bool,
    status: String,
}

impl ConfigApp {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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
            Message::GenerateConfig => {
                self.status = "Config generated!".to_string();
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let title = text("IoT2050 Config Tool").size(24);
        
        let ip_input = text_input("OPC-UA Server IP", &self.ip)
            .on_input(Message::IpChanged);
            
        let username_input = text_input("Username", &self.username)
            .on_input(Message::UsernameChanged);
            
        let password_input = text_input("Password", &self.password)
            .on_input(Message::PasswordChanged)
            .secure(true);
            
        let anonymous_checkbox = checkbox("Anonymous mode", self.anonymous)
            .on_toggle(|_| Message::ToggleAnonymous);
            
        let generate_button = button("Generate Config")
            .on_press(Message::GenerateConfig);
            
        let status_text = text(&self.status);

        column![
            title,
            ip_input,
            username_input,
            password_input,
            anonymous_checkbox,
            generate_button,
            status_text
        ]
        .spacing(20)
        .padding(20)
                ip: String::new(),
                interval: String::new(),
            })
            .collect();
        
        self.status_message = format!("Found {} XML files", self.xml_files.len());
    }

    fn view_files_section(&self) -> Element<Message> {
        let title = text("XML Files").size(18);
        
        let refresh_btn = button("Refresh Files")
            .on_press(Message::RefreshFiles)
            .style(button::Style {
                background: Some(Background::Color(Color::from_rgb(0.2, 0.6, 0.9))),
                text_color: Color::WHITE,
                border: Border::rounded(4),
                ..Default::default()
            });

        let files_list = if self.xml_files.is_empty() {
            column![text("No XML files found").style(Color::from_rgb(0.6, 0.6, 0.6))]
        } else {
            self.xml_files
                .iter()
                .enumerate()
                .fold(column![], |col, (i, file)| {
                    let file_row = row![
                        checkbox("", file.selected)
                            .on_toggle(move |_| Message::ToggleFileSelection(i)),
                        checkbox("Listener", file.is_listener)
                            .on_toggle(move |_| Message::ToggleListenerMode(i)),
                        text(&file.name).width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_items(iced::Alignment::Center);
                    
                    let config_row = row![
                        text("NS:").size(12),
                        text_input("2", &file.namespace)
                            .on_input(move |v| Message::FileNamespaceChanged(i, v))
                            .width(60),
                        text("IP:").size(12),
                        text_input("default", &file.ip)
                            .on_input(move |v| Message::FileIpChanged(i, v))
                            .width(120),
                        text("INT:").size(12),
                        text_input(
                            if file.is_listener { "500ms" } else { "1000ms" },
                            &file.interval
                        )
                        .on_input(move |v| Message::FileIntervalChanged(i, v))
                        .width(80),
                    ]
                    .spacing(5)
                    .align_items(iced::Alignment::Center);
                    
                    col.push(
                        container(
                            column![file_row, config_row]
                                .spacing(5)
                                .padding(10)
                        )
                        .style(container::Style {
                            background: Some(Background::Color(Color::from_rgb(0.95, 0.95, 0.95))),
                            border: Border::rounded(4),
                            ..Default::default()
                        })
                    )
                    .push(vertical_space(5))
                })
        };

        column![
            title,
            refresh_btn,
            vertical_space(10),
            scrollable(files_list).height(300)
        ]
        .spacing(10)
    }

    fn view_config_section(&self) -> Element<Message> {
        let opcua_section = column![
            text("OPC-UA Configuration").size(16),
            text_input("OPC-UA Server IP", &self.config.ip)
                .on_input(Message::OpcUaIpChanged),
            text_input("Username", &self.config.username)
                .on_input(Message::OpcUaUsernameChanged)
                .style(if self.anonymous_mode {
                    text_input::Style {
                        background: Background::Color(Color::from_rgb(0.9, 0.9, 0.9)),
                        ..Default::default()
                    }
                } else {
                    text_input::Style::default()
                }),
            text_input("Password", &self.config.password)
                .on_input(Message::OpcUaPasswordChanged)
                .password()
                .style(if self.anonymous_mode {
                    text_input::Style {
                        background: Background::Color(Color::from_rgb(0.9, 0.9, 0.9)),
                        ..Default::default()
                    }
                } else {
                    text_input::Style::default()
                }),
            checkbox("Anonymous Mode", self.anonymous_mode)
                .on_toggle(|_| Message::ToggleAnonymousMode),
            checkbox("Include Test Inputs", self.config.include_test_inputs)
                .on_toggle(|_| Message::ToggleTestInputs),
        ]
        .spacing(10);

        let iot_section = column![
            text("IoT Device Configuration").size(16),
            text_input("IoT Device IP", &self.config.iot_host)
                .on_input(Message::IoTIpChanged),
            text_input("IoT Username", &self.config.iot_username)
                .on_input(Message::IoTUsernameChanged),
            text_input("IoT Password", &self.config.iot_password)
                .on_input(Message::IoTPasswordChanged)
                .password(),
        ]
        .spacing(10);

        column![opcua_section, vertical_space(20), iot_section].spacing(10)
    }

    fn view_actions_section(&self) -> Element<Message> {
        let generate_btn = button(if self.is_generating {
            "Generating..."
        } else {
            "Generate Config"
        })
        .on_press_maybe(if self.is_generating {
            None
        } else {
            Some(Message::GenerateConfig)
        })
        .style(button::Style {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.7, 0.2))),
            text_color: Color::WHITE,
            border: Border::rounded(4),
            ..Default::default()
        });

        let status_btn = button("Telegraf Status")
            .on_press(Message::TelegrafStatus)
            .style(button::Style {
                background: Some(Background::Color(Color::from_rgb(0.8, 0.4, 0.1))),
                text_color: Color::WHITE,
                border: Border::rounded(4),
                ..Default::default()
            });

        column![
            text("Actions").size(16),
            generate_btn,
            status_btn,
        ]
        .spacing(10)
    }

    fn view_status_section(&self) -> Element<Message> {
        container(
            text(&self.status_message)
                .style(Color::from_rgb(0.3, 0.3, 0.3))
        )
        .padding(10)
        .style(container::Style {
            background: Some(Background::Color(Color::from_rgb(0.98, 0.98, 0.98))),
            border: Border::rounded(4),
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }
}
