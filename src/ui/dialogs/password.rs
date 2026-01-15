//! Password entry dialog

use crate::security::DeviceId;
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Response from password dialog
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordResponse {
    /// User entered password
    Entered(String),
    /// User cancelled
    Cancelled,
    /// Dialog still open
    Pending,
}

/// Password entry dialog
pub struct PasswordDialog {
    remote_id: DeviceId,
    password: String,
    show_password: bool,
    response: PasswordResponse,
}

impl PasswordDialog {
    /// Create a new password dialog
    pub fn new(remote_id: DeviceId) -> Self {
        Self {
            remote_id,
            password: String::new(),
            show_password: false,
            response: PasswordResponse::Pending,
        }
    }

    /// Get the current response
    pub fn response(&self) -> &PasswordResponse {
        &self.response
    }

    /// Show the dialog window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut should_close = false;

        egui::Window::new("Password Required")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Enter Password");
                    ui.add_space(10.0);

                    ui.label("This connection requires a password.");
                    ui.label(format!(
                        "Remote device: {}",
                        self.remote_id.format_with_spaces()
                    ));

                    ui.add_space(15.0);

                    // Password input
                    let password_response = ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(!self.show_password)
                            .hint_text("Enter password")
                            .desired_width(300.0),
                    );

                    // Focus on password field when dialog opens
                    if self.password.is_empty() {
                        password_response.request_focus();
                    }

                    // Submit on Enter key
                    if password_response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && !self.password.is_empty()
                    {
                        self.response = PasswordResponse::Entered(self.password.clone());
                        should_close = true;
                    }

                    ui.checkbox(&mut self.show_password, "Show password");

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.password.is_empty(), egui::Button::new("✓ Connect"))
                            .clicked()
                        {
                            self.response = PasswordResponse::Entered(self.password.clone());
                            should_close = true;
                        }

                        if ui.button("✗ Cancel").clicked() {
                            self.response = PasswordResponse::Cancelled;
                            should_close = true;
                        }
                    });
                });
            });

        !should_close
    }
}

/// Run the password dialog as a standalone window
pub fn run_password_dialog(
    remote_id: DeviceId,
) -> Result<PasswordResponse, Box<dyn std::error::Error>> {
    let response = Arc::new(Mutex::new(PasswordResponse::Pending));
    let response_clone = response.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 250.0])
            .with_resizable(false)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Password Required",
        options,
        Box::new(move |_cc| Box::new(PasswordApp::new(remote_id, response_clone))),
    )?;

    let final_response = response.lock().unwrap().clone();
    Ok(final_response)
}

/// Standalone app for password dialog
struct PasswordApp {
    dialog: PasswordDialog,
    response: Arc<Mutex<PasswordResponse>>,
}

impl PasswordApp {
    fn new(remote_id: DeviceId, response: Arc<Mutex<PasswordResponse>>) -> Self {
        Self {
            dialog: PasswordDialog::new(remote_id),
            response,
        }
    }
}

impl eframe::App for PasswordApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.dialog.show(ctx) {
            *self.response.lock().unwrap() = self.dialog.response().clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
