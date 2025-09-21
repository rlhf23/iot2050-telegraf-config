use iced::widget::{button, checkbox, column, container, text, text_input, Space};
use iced::{Element, Task, Length, Color, Background, Border, Shadow, Vector, Theme};

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
                self.status = format!("Config would be generated for IP: {}", self.ip);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let title = text("🏭 IoT2050 Config Tool")
            .size(28);
        
        let subtitle = text("Configure OPC-UA monitoring for industrial devices")
            .size(14);

        let ip_input = text_input("Enter OPC-UA Server IP", &self.ip)
            .on_input(Message::IpChanged)
            .padding(12);
            
        let username_input = text_input("Username", &self.username)
            .on_input(Message::UsernameChanged)
            .padding(12);
            
        let password_input = text_input("Password", &self.password)
            .on_input(Message::PasswordChanged)
            .secure(true)
            .padding(12);
            
        let anonymous_checkbox = checkbox("🔓 Anonymous Authentication", self.anonymous)
            .on_toggle(|_| Message::ToggleAnonymous);
            
        let generate_button = button("🚀 Generate Configuration")
            .on_press(Message::GenerateConfig)
            .padding([12, 24]);
            
        let status_text = if !self.status.is_empty() {
            text(&self.status).size(14)
        } else {
            text("Ready to generate configuration...").size(14)
        };

        let content = column![
            title,
            subtitle,
            Space::with_height(20),
            text("🔧 OPC-UA Configuration").size(18),
            Space::with_height(10),
            text("Server IP Address").size(12),
            ip_input,
            Space::with_height(15),
            text("Authentication").size(12),
            anonymous_checkbox,
            Space::with_height(10),
            if !self.anonymous {
                column![
                    username_input,
                    Space::with_height(10),
                    password_input,
                ].into()
            } else {
                Space::with_height(0).into()
            },
            Space::with_height(30),
            generate_button,
            Space::with_height(20),
            status_text,
        ]
        .spacing(10)
        .padding(40)
        .max_width(500);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
