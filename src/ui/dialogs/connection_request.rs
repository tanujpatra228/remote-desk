//! Connection request dialog

use crate::security::DeviceId;
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Response from connection request dialog
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionResponse {
    /// User accepted the connection
    Accepted { password: Option<String> },
    /// User rejected the connection
    Rejected,
    /// Dialog still open, no response yet
    Pending,
}

/// Connection request dialog
pub struct ConnectionRequestDialog {
    remote_id: DeviceId,
    remote_name: String,
    requires_password: bool,
    password: String,
    response: ConnectionResponse,
}

impl ConnectionRequestDialog {
    /// Create a new connection request dialog
    pub fn new(remote_id: DeviceId, remote_name: String, requires_password: bool) -> Self {
        Self {
            remote_id,
            remote_name,
            requires_password,
            password: String::new(),
            response: ConnectionResponse::Pending,
        }
    }

    /// Get the current response
    pub fn response(&self) -> &ConnectionResponse {
        &self.response
    }

    /// Show the dialog window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut should_close = false;

        egui::Window::new("Incoming Connection Request")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Someone wants to connect to your computer");
                    ui.add_space(10.0);

                    // Device information
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Remote Device").strong());
                        ui.label(format!("Name: {}", self.remote_name));
                        ui.label(format!(
                            "ID: {}",
                            self.remote_id.format_with_spaces()
                        ));
                    });

                    ui.add_space(10.0);

                    // Password input if required
                    if self.requires_password {
                        ui.label("Enter password to allow this connection:");
                        let password_response = ui.add(
                            egui::TextEdit::singleline(&mut self.password)
                                .password(true)
                                .hint_text("Enter password"),
                        );

                        // Submit on Enter key
                        if password_response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            if !self.password.is_empty() {
                                self.response = ConnectionResponse::Accepted {
                                    password: Some(self.password.clone()),
                                };
                                should_close = true;
                            }
                        }

                        ui.add_space(10.0);
                    }

                    // Warning
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 150, 50),
                        "⚠️  This will allow remote control of your computer.",
                    );

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        let accept_enabled = !self.requires_password || !self.password.is_empty();

                        if ui
                            .add_enabled(accept_enabled, egui::Button::new("✓ Accept"))
                            .clicked()
                        {
                            self.response = ConnectionResponse::Accepted {
                                password: if self.requires_password {
                                    Some(self.password.clone())
                                } else {
                                    None
                                },
                            };
                            should_close = true;
                        }

                        if ui.button("✗ Reject").clicked() {
                            self.response = ConnectionResponse::Rejected;
                            should_close = true;
                        }
                    });
                });
            });

        !should_close
    }
}

/// Run the connection request dialog as a standalone window
pub fn run_connection_request_dialog(
    remote_id: DeviceId,
    remote_name: String,
    requires_password: bool,
) -> Result<ConnectionResponse, Box<dyn std::error::Error>> {
    let response = Arc::new(Mutex::new(ConnectionResponse::Pending));
    let response_clone = response.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 300.0])
            .with_resizable(false)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Incoming Connection Request",
        options,
        Box::new(move |_cc| {
            Box::new(ConnectionRequestApp::new(
                remote_id,
                remote_name,
                requires_password,
                response_clone,
            ))
        }),
    )?;

    let final_response = response.lock().unwrap().clone();
    Ok(final_response)
}

/// Standalone app for connection request dialog
struct ConnectionRequestApp {
    dialog: ConnectionRequestDialog,
    response: Arc<Mutex<ConnectionResponse>>,
}

impl ConnectionRequestApp {
    fn new(
        remote_id: DeviceId,
        remote_name: String,
        requires_password: bool,
        response: Arc<Mutex<ConnectionResponse>>,
    ) -> Self {
        Self {
            dialog: ConnectionRequestDialog::new(remote_id, remote_name, requires_password),
            response,
        }
    }
}

impl eframe::App for ConnectionRequestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.dialog.show(ctx) {
            *self.response.lock().unwrap() = self.dialog.response().clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
