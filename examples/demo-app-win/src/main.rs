use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting egui-mcp demo application");

    // Initialize egui-mcp client
    let client = egui_mcp_client_win::init().await.unwrap();
    let client = Arc::new(Mutex::new(client));

    // Spawn the IPC server
    let client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.lock().await.run().await {
            eprintln!("IPC server error: {}", e);
        }
    });

    info!("Demo application initialized, waiting for MCP server to connect");

    // Run the eframe application
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "egui-mcp Demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
    )
}

struct DemoApp {
    counter: i32,
    text_input: String,
    checkbox: bool,
    slider_value: f32,
    menu_action: String,
}

impl DemoApp {
    fn new() -> Self {
        Self {
            counter: 0,
            text_input: String::new(),
            checkbox: false,
            slider_value: 50.0,
            menu_action: "None".to_string(),
        }
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.enable_accesskit();

        egui::TopBottomPanel::top("demo_menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Actions", |ui| {
                    if ui.button("Action Alpha").clicked() {
                        self.menu_action = "Action Alpha".to_string();
                        ui.close_menu();
                    }
                    if ui.button("Action Beta").clicked() {
                        self.menu_action = "Action Beta".to_string();
                        ui.close_menu();
                    }
                    if ui.button("Reset Menu Action").clicked() {
                        self.menu_action = "None".to_string();
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui-mcp Demo");
            ui.separator();

            ui.label("This is a demo application for egui-mcp on Windows.");
            ui.label("The MCP server can connect to this application to:");
            ui.label("• Take screenshots");
            ui.label("• Send input events");
            ui.label("• Inspect UI elements");

            ui.separator();
            ui.heading("Interactive Elements");

            ui.add(egui::Slider::new(&mut self.slider_value, 0.0..=100.0).text("Slider"));
            ui.checkbox(&mut self.checkbox, "Checkbox");
            ui.add(
                egui::DragValue::new(&mut self.counter)
                    .speed(1.0)
                    .range(0..=100)
                    .prefix("Counter: "),
            );
            ui.text_edit_singleline(&mut self.text_input);

            ui.separator();
            ui.label(format!("Slider value: {:.1}", self.slider_value));
            ui.label(format!(
                "Checkbox: {}",
                if self.checkbox {
                    "checked"
                } else {
                    "unchecked"
                }
            ));
            ui.label(format!("Text: {}", self.text_input));
            ui.label(format!("Menu action: {}", self.menu_action));

            // Request repaint for animations
            ctx.request_repaint();
        });
    }
}
