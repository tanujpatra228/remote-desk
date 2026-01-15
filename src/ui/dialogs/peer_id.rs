//! Peer ID display dialog with QR code

use crate::security::DeviceId;
use eframe::egui;
use qrcode::QrCode;

/// Peer ID display dialog
pub struct PeerIdDialog {
    device_id: DeviceId,
    qr_texture: Option<egui::TextureHandle>,
    copied: bool,
    copy_timer: f32,
}

impl PeerIdDialog {
    /// Create a new peer ID dialog
    pub fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            qr_texture: None,
            copied: false,
            copy_timer: 0.0,
        }
    }

    /// Show the dialog window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut should_close = false;

        egui::Window::new("Your Device ID")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Share this ID to receive connections");
                    ui.add_space(10.0);

                    // Display device ID
                    let formatted_id = self.device_id.format_with_spaces();
                    ui.label(
                        egui::RichText::new(&formatted_id)
                            .size(24.0)
                            .monospace()
                            .strong(),
                    );

                    ui.add_space(10.0);

                    // Copy button
                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.output_mut(|o| o.copied_text = formatted_id.clone());
                        self.copied = true;
                        self.copy_timer = 2.0; // Show "Copied!" for 2 seconds
                    }

                    if self.copied && self.copy_timer > 0.0 {
                        ui.colored_label(egui::Color32::GREEN, "✓ Copied to clipboard!");
                        self.copy_timer -= ui.input(|i| i.stable_dt);
                        if self.copy_timer <= 0.0 {
                            self.copied = false;
                        }
                        ctx.request_repaint(); // Keep updating for timer
                    }

                    ui.add_space(20.0);

                    // QR Code
                    ui.label("Scan QR Code:");
                    ui.add_space(5.0);

                    if self.qr_texture.is_none() {
                        self.qr_texture = Some(Self::generate_qr_texture(ctx, &self.device_id));
                    }

                    if let Some(texture) = &self.qr_texture {
                        ui.image((texture.id(), egui::vec2(200.0, 200.0)));
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                });
            });

        !should_close
    }

    /// Generate QR code texture
    fn generate_qr_texture(ctx: &egui::Context, device_id: &DeviceId) -> egui::TextureHandle {
        let id_string = device_id.format_with_spaces();

        // Generate QR code
        let qr = QrCode::new(id_string.as_bytes()).expect("Failed to generate QR code");
        let colors = qr.to_colors();
        let size = (colors.len() as f64).sqrt() as usize;

        // Convert to ColorImage
        let pixels: Vec<egui::Color32> = colors
            .iter()
            .map(|c| {
                if *c == qrcode::Color::Light {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::BLACK
                }
            })
            .collect();

        let color_image = egui::ColorImage {
            size: [size, size],
            pixels,
        };

        ctx.load_texture("qr_code", color_image, egui::TextureOptions::NEAREST)
    }
}

/// Run the peer ID dialog as a standalone window
pub fn run_peer_id_dialog(device_id: DeviceId) -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 550.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Your Device ID",
        options,
        Box::new(move |_cc| Box::new(PeerIdApp::new(device_id))),
    )?;

    Ok(())
}

/// Standalone app for peer ID dialog
struct PeerIdApp {
    dialog: PeerIdDialog,
}

impl PeerIdApp {
    fn new(device_id: DeviceId) -> Self {
        Self {
            dialog: PeerIdDialog::new(device_id),
        }
    }
}

impl eframe::App for PeerIdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.dialog.show(ctx) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
