//! Settings dialog

use eframe::egui;

/// Settings dialog
pub struct SettingsDialog {
    auto_accept: bool,
    require_password: bool,
    password: String,
    show_password: bool,
}

impl SettingsDialog {
    /// Create a new settings dialog
    pub fn new() -> Self {
        Self {
            auto_accept: false,
            require_password: true,
            password: String::new(),
            show_password: false,
        }
    }

    /// Show the dialog window and return whether it should stay open
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut should_close = false;

        egui::Window::new("Settings")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Connection Settings");
                    ui.add_space(10.0);

                    ui.checkbox(&mut self.auto_accept, "Auto-accept incoming connections");

                    if self.auto_accept {
                        ui.indent("password_settings", |ui| {
                            ui.checkbox(&mut self.require_password, "Require password");

                            if self.require_password {
                                ui.horizontal(|ui| {
                                    ui.label("Password:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(!self.show_password)
                                            .desired_width(150.0),
                                    );
                                });
                                ui.checkbox(&mut self.show_password, "Show password");
                            }
                        });
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            // TODO: Save settings to config
                            should_close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            open = false;
        }

        open
    }
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}
