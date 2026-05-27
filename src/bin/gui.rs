use eframe::egui;
use sie_generate_config::{gui_controller::GuiController, gui_renderer::GuiRenderer};

struct TelegrafApp {
    controller: GuiController,
}

impl TelegrafApp {
    fn new() -> Self {
        Self {
            controller: GuiController::new(),
        }
    }
}

impl eframe::App for TelegrafApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process worker responses and request repaint if needed
        if self.controller.process_worker_responses() {
            ctx.request_repaint();
        }

        // Main UI rendering
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Telegraf Configuration Generator");

            // Configuration Section
            GuiRenderer::render_configuration(ui, &mut self.controller);

            // XML Files Configuration
            GuiRenderer::render_xml_files_configuration(ui, &mut self.controller);

            // Main Action Buttons
            GuiRenderer::render_main_action_buttons(ui, &mut self.controller);

            // Other Commands Section
            GuiRenderer::render_other_commands(ui, &mut self.controller);

            // Status Messages
            GuiRenderer::render_status_messages(ui, &mut self.controller);

            // Selected Nodes
            GuiRenderer::render_selected_nodes(ui, &mut self.controller);
        });

        // OPC UA Browser Window
        GuiRenderer::render_opcua_browser(ctx, &mut self.controller);
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 1000.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Telegraf Config Generator",
        options,
        Box::new(|_cc| Ok(Box::new(TelegrafApp::new()))),
    )
}
