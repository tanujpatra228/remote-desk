//! Viewer window for displaying remote desktop frames
//!
//! This module provides an egui-based window that displays received frames
//! and captures user input to send to the remote host.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::desktop::{Frame, FrameDecoder};
use crate::input::{InputEvent, Key, KeyboardEvent, MouseButton, MouseEvent};
use crate::session::transport::{TransportFrame, TransportInput};
use crate::ui::overlay::StatusOverlay;

/// Viewer window configuration
#[derive(Debug, Clone)]
pub struct ViewerConfig {
    /// Window title
    pub title: String,
    /// Initial window width
    pub width: u32,
    /// Initial window height
    pub height: u32,
    /// Whether to show the status overlay
    pub show_overlay: bool,
    /// Whether to capture input
    pub capture_input: bool,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            title: "RemoteDesk Viewer".to_string(),
            width: 1280,
            height: 720,
            show_overlay: true,
            capture_input: true,
        }
    }
}

/// Statistics for the viewer
#[derive(Debug, Clone, Default)]
pub struct ViewerStats {
    /// Frames received
    pub frames_received: u64,
    /// Frames displayed
    pub frames_displayed: u64,
    /// Frames dropped
    pub frames_dropped: u64,
    /// Input events sent
    pub input_events_sent: u64,
    /// Current FPS
    pub current_fps: f64,
    /// Last latency measurement (ms)
    pub latency_ms: Option<u64>,
    /// Bandwidth in bytes/sec
    pub bandwidth_bps: f64,
}

/// Viewer window for remote desktop display
pub struct ViewerWindow {
    /// Configuration
    config: ViewerConfig,
    /// Current texture for display
    texture: Option<egui::TextureHandle>,
    /// Current frame dimensions
    frame_size: (u32, u32),
    /// Frame decoder
    decoder: FrameDecoder,
    /// Frame receiver channel
    frame_rx: Option<mpsc::Receiver<TransportFrame>>,
    /// Input sender channel
    input_tx: Option<mpsc::Sender<TransportInput>>,
    /// Input sequence counter
    input_sequence: Arc<AtomicU64>,
    /// Status overlay
    overlay: StatusOverlay,
    /// Statistics
    stats: ViewerStats,
    /// FPS calculation
    fps_counter: FpsCounter,
    /// Last received frame timestamp
    last_frame_time: Option<Instant>,
    /// Whether the window has focus for input capture
    has_focus: bool,
    /// Mouse position relative to remote screen
    mouse_pos: Option<(f32, f32)>,
}

/// Helper struct for FPS calculation
#[derive(Debug, Clone)]
struct FpsCounter {
    frame_times: Vec<Instant>,
    max_samples: usize,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            frame_times: Vec::new(),
            max_samples: 60,
        }
    }
}

impl FpsCounter {
    fn add_frame(&mut self) {
        let now = Instant::now();
        self.frame_times.push(now);

        // Keep only recent samples
        while self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }
    }

    fn fps(&self) -> f64 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }

        let oldest = self.frame_times.first().unwrap();
        let newest = self.frame_times.last().unwrap();
        let duration = newest.duration_since(*oldest).as_secs_f64();

        if duration == 0.0 {
            return 0.0;
        }

        (self.frame_times.len() - 1) as f64 / duration
    }
}

impl ViewerWindow {
    /// Creates a new viewer window
    pub fn new(
        config: ViewerConfig,
        frame_rx: mpsc::Receiver<TransportFrame>,
        input_tx: mpsc::Sender<TransportInput>,
    ) -> Self {
        Self {
            config,
            texture: None,
            frame_size: (0, 0),
            decoder: FrameDecoder::new(),
            frame_rx: Some(frame_rx),
            input_tx: Some(input_tx),
            input_sequence: Arc::new(AtomicU64::new(0)),
            overlay: StatusOverlay::default(),
            stats: ViewerStats::default(),
            fps_counter: FpsCounter::default(),
            last_frame_time: None,
            has_focus: true,
            mouse_pos: None,
        }
    }

    /// Creates a viewer window with default config
    pub fn with_channels(
        frame_rx: mpsc::Receiver<TransportFrame>,
        input_tx: mpsc::Sender<TransportInput>,
    ) -> Self {
        Self::new(ViewerConfig::default(), frame_rx, input_tx)
    }

    /// Runs the viewer window (blocking)
    pub fn run(self) -> Result<(), eframe::Error> {
        let title = self.config.title.clone();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([self.config.width as f32, self.config.height as f32])
                .with_title(&title),
            ..Default::default()
        };

        eframe::run_native(
            &title,
            options,
            Box::new(move |_cc| Box::new(self)),
        )
    }

    /// Processes any pending frames
    fn process_pending_frames(&mut self, ctx: &egui::Context) {
        if let Some(ref mut rx) = self.frame_rx {
            // Try to receive all pending frames, keeping only the latest
            let mut latest_frame: Option<TransportFrame> = None;
            let mut frames_received = 0;

            while let Ok(frame) = rx.try_recv() {
                frames_received += 1;
                latest_frame = Some(frame);
            }

            self.stats.frames_received += frames_received;
            if frames_received > 1 {
                self.stats.frames_dropped += frames_received - 1;
            }

            // Process the latest frame
            if let Some(transport_frame) = latest_frame {
                self.process_frame(ctx, transport_frame);
            }
        }
    }

    /// Processes a single frame
    fn process_frame(&mut self, ctx: &egui::Context, transport_frame: TransportFrame) {
        match self.decoder.decode_transport(&transport_frame) {
            Ok(frame) => {
                self.update_texture(ctx, &frame);
                self.fps_counter.add_frame();
                self.stats.frames_displayed += 1;
                self.stats.current_fps = self.fps_counter.fps();
                self.last_frame_time = Some(Instant::now());

                debug!(
                    "Displayed frame {} ({}x{})",
                    transport_frame.sequence, frame.width, frame.height
                );
            }
            Err(e) => {
                warn!("Failed to decode frame: {}", e);
                self.stats.frames_dropped += 1;
            }
        }
    }

    /// Updates the texture with a new frame
    fn update_texture(&mut self, ctx: &egui::Context, frame: &Frame) {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.data,
        );

        self.frame_size = (frame.width, frame.height);

        match &mut self.texture {
            Some(texture) => {
                // Update existing texture
                texture.set(image, egui::TextureOptions::LINEAR);
            }
            None => {
                // Create new texture
                self.texture = Some(ctx.load_texture("remote_frame", image, egui::TextureOptions::LINEAR));
            }
        }
    }

    /// Handles keyboard input
    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        if !self.config.capture_input || !self.has_focus {
            return;
        }

        ctx.input(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Key {
                        key,
                        pressed,
                        modifiers: _,
                        ..
                    } => {
                        if let Some(our_key) = self.convert_egui_key(*key) {
                            let kb_event = if *pressed {
                                KeyboardEvent::key_press(our_key)
                            } else {
                                KeyboardEvent::key_release(our_key)
                            };

                            self.send_input(InputEvent::Keyboard(kb_event));
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// Handles mouse input
    fn handle_mouse_input(&mut self, ctx: &egui::Context, image_rect: egui::Rect) {
        if !self.config.capture_input || !self.has_focus {
            return;
        }

        ctx.input(|input| {
            // Handle mouse position
            if let Some(pos) = input.pointer.hover_pos() {
                if image_rect.contains(pos) {
                    // Calculate position relative to the remote screen
                    let rel_x = (pos.x - image_rect.left()) / image_rect.width();
                    let rel_y = (pos.y - image_rect.top()) / image_rect.height();

                    let remote_x = (rel_x * self.frame_size.0 as f32) as i32;
                    let remote_y = (rel_y * self.frame_size.1 as f32) as i32;

                    // Check if position changed
                    let new_pos = (rel_x, rel_y);
                    if self.mouse_pos != Some(new_pos) {
                        self.mouse_pos = Some(new_pos);
                        self.send_input(InputEvent::Mouse(MouseEvent::move_to(remote_x, remote_y)));
                    }
                }
            }

            // Handle mouse buttons
            if input.pointer.button_pressed(egui::PointerButton::Primary) {
                self.send_input(InputEvent::Mouse(MouseEvent::button_press(MouseButton::Left)));
            }
            if input.pointer.button_released(egui::PointerButton::Primary) {
                self.send_input(InputEvent::Mouse(MouseEvent::button_release(MouseButton::Left)));
            }
            if input.pointer.button_pressed(egui::PointerButton::Secondary) {
                self.send_input(InputEvent::Mouse(MouseEvent::button_press(MouseButton::Right)));
            }
            if input.pointer.button_released(egui::PointerButton::Secondary) {
                self.send_input(InputEvent::Mouse(MouseEvent::button_release(MouseButton::Right)));
            }
            if input.pointer.button_pressed(egui::PointerButton::Middle) {
                self.send_input(InputEvent::Mouse(MouseEvent::button_press(MouseButton::Middle)));
            }
            if input.pointer.button_released(egui::PointerButton::Middle) {
                self.send_input(InputEvent::Mouse(MouseEvent::button_release(MouseButton::Middle)));
            }

            // Handle scroll
            let scroll = input.smooth_scroll_delta;
            if scroll.x != 0.0 || scroll.y != 0.0 {
                self.send_input(InputEvent::Mouse(MouseEvent::wheel(
                    scroll.x as i32,
                    scroll.y as i32,
                )));
            }
        });
    }

    /// Sends an input event to the host
    fn send_input(&mut self, event: InputEvent) {
        if let Some(ref tx) = self.input_tx {
            let sequence = self.input_sequence.fetch_add(1, Ordering::SeqCst);
            let transport_input = TransportInput::new(event, sequence);

            if tx.try_send(transport_input).is_ok() {
                self.stats.input_events_sent += 1;
            }
        }
    }

    /// Converts egui key to our Key type
    fn convert_egui_key(&self, key: egui::Key) -> Option<Key> {
        Some(match key {
            egui::Key::A => Key::A,
            egui::Key::B => Key::B,
            egui::Key::C => Key::C,
            egui::Key::D => Key::D,
            egui::Key::E => Key::E,
            egui::Key::F => Key::F,
            egui::Key::G => Key::G,
            egui::Key::H => Key::H,
            egui::Key::I => Key::I,
            egui::Key::J => Key::J,
            egui::Key::K => Key::K,
            egui::Key::L => Key::L,
            egui::Key::M => Key::M,
            egui::Key::N => Key::N,
            egui::Key::O => Key::O,
            egui::Key::P => Key::P,
            egui::Key::Q => Key::Q,
            egui::Key::R => Key::R,
            egui::Key::S => Key::S,
            egui::Key::T => Key::T,
            egui::Key::U => Key::U,
            egui::Key::V => Key::V,
            egui::Key::W => Key::W,
            egui::Key::X => Key::X,
            egui::Key::Y => Key::Y,
            egui::Key::Z => Key::Z,
            egui::Key::Num0 => Key::Num0,
            egui::Key::Num1 => Key::Num1,
            egui::Key::Num2 => Key::Num2,
            egui::Key::Num3 => Key::Num3,
            egui::Key::Num4 => Key::Num4,
            egui::Key::Num5 => Key::Num5,
            egui::Key::Num6 => Key::Num6,
            egui::Key::Num7 => Key::Num7,
            egui::Key::Num8 => Key::Num8,
            egui::Key::Num9 => Key::Num9,
            egui::Key::Escape => Key::Escape,
            egui::Key::Tab => Key::Tab,
            egui::Key::Backspace => Key::Backspace,
            egui::Key::Enter => Key::Return,
            egui::Key::Space => Key::Space,
            egui::Key::Insert => Key::Insert,
            egui::Key::Delete => Key::Delete,
            egui::Key::Home => Key::Home,
            egui::Key::End => Key::End,
            egui::Key::PageUp => Key::PageUp,
            egui::Key::PageDown => Key::PageDown,
            egui::Key::ArrowLeft => Key::Left,
            egui::Key::ArrowRight => Key::Right,
            egui::Key::ArrowUp => Key::Up,
            egui::Key::ArrowDown => Key::Down,
            egui::Key::F1 => Key::F1,
            egui::Key::F2 => Key::F2,
            egui::Key::F3 => Key::F3,
            egui::Key::F4 => Key::F4,
            egui::Key::F5 => Key::F5,
            egui::Key::F6 => Key::F6,
            egui::Key::F7 => Key::F7,
            egui::Key::F8 => Key::F8,
            egui::Key::F9 => Key::F9,
            egui::Key::F10 => Key::F10,
            egui::Key::F11 => Key::F11,
            egui::Key::F12 => Key::F12,
            egui::Key::Minus => Key::Minus,
            egui::Key::Comma => Key::Comma,
            egui::Key::Period => Key::Period,
            _ => return None,
        })
    }

    /// Returns the current statistics
    pub fn stats(&self) -> &ViewerStats {
        &self.stats
    }
}

impl eframe::App for ViewerWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any pending frames
        self.process_pending_frames(ctx);

        // Request continuous repaint for smooth updates
        ctx.request_repaint();

        // Main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Calculate image rect for input translation
            let available_size = ui.available_size();
            let image_rect = if let Some(ref texture) = self.texture {
                // Calculate aspect-ratio-preserving size
                let tex_size = texture.size_vec2();
                let scale = (available_size.x / tex_size.x).min(available_size.y / tex_size.y);
                let scaled_size = tex_size * scale;

                // Center the image
                let offset = (available_size - scaled_size) / 2.0;
                let min = ui.min_rect().min + offset;

                // Display the image
                let image = egui::Image::new(texture)
                    .fit_to_exact_size(scaled_size);
                ui.put(egui::Rect::from_min_size(min, scaled_size), image);

                egui::Rect::from_min_size(min, scaled_size)
            } else {
                // No frame yet, show placeholder
                ui.centered_and_justified(|ui| {
                    ui.label("Waiting for frames...");
                });
                ui.min_rect()
            };

            // Handle input
            self.handle_keyboard_input(ctx);
            self.handle_mouse_input(ctx, image_rect);

            // Show overlay if enabled
            if self.config.show_overlay {
                self.overlay.show(ui, &self.stats);
            }
        });
    }
}
