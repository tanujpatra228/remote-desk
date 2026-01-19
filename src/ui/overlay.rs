//! Status overlay for the viewer window
//!
//! Displays FPS, latency, bandwidth, and other statistics.

use eframe::egui;

use crate::ui::viewer::ViewerStats;

/// Position for the overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayPosition {
    /// Top-left corner
    #[default]
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    BottomRight,
}

/// Status overlay configuration
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Whether the overlay is visible
    pub visible: bool,
    /// Overlay position
    pub position: OverlayPosition,
    /// Background opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Font size
    pub font_size: f32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            visible: true,
            position: OverlayPosition::TopLeft,
            opacity: 0.7,
            font_size: 12.0,
        }
    }
}

/// Status overlay widget
#[derive(Debug, Clone, Default)]
pub struct StatusOverlay {
    /// Overlay configuration
    config: OverlayConfig,
}

impl StatusOverlay {
    /// Creates a new status overlay
    pub fn new(config: OverlayConfig) -> Self {
        Self { config }
    }

    /// Sets visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.config.visible = visible;
    }

    /// Returns whether the overlay is visible
    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    /// Sets the position
    pub fn set_position(&mut self, position: OverlayPosition) {
        self.config.position = position;
    }

    /// Shows the overlay
    pub fn show(&self, ui: &mut egui::Ui, stats: &ViewerStats) {
        if !self.config.visible {
            return;
        }

        let available = ui.available_rect_before_wrap();
        let padding = 10.0;
        let overlay_width = 150.0;
        let overlay_height = 100.0;

        // Calculate position based on setting
        let pos = match self.config.position {
            OverlayPosition::TopLeft => egui::pos2(
                available.left() + padding,
                available.top() + padding,
            ),
            OverlayPosition::TopRight => egui::pos2(
                available.right() - overlay_width - padding,
                available.top() + padding,
            ),
            OverlayPosition::BottomLeft => egui::pos2(
                available.left() + padding,
                available.bottom() - overlay_height - padding,
            ),
            OverlayPosition::BottomRight => egui::pos2(
                available.right() - overlay_width - padding,
                available.bottom() - overlay_height - padding,
            ),
        };

        // Draw overlay background
        let rect = egui::Rect::from_min_size(pos, egui::vec2(overlay_width, overlay_height));

        // Draw semi-transparent background
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(4.0),
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, (self.config.opacity * 255.0) as u8),
        );

        // Draw stats text
        let text_color = egui::Color32::WHITE;
        let line_height = self.config.font_size + 4.0;
        let mut y = pos.y + 8.0;

        // FPS
        let fps_text = format!("FPS: {:.1}", stats.current_fps);
        ui.painter().text(
            egui::pos2(pos.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            fps_text,
            egui::FontId::proportional(self.config.font_size),
            text_color,
        );
        y += line_height;

        // Latency
        let latency_text = match stats.latency_ms {
            Some(ms) => format!("Latency: {}ms", ms),
            None => "Latency: --".to_string(),
        };
        ui.painter().text(
            egui::pos2(pos.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            latency_text,
            egui::FontId::proportional(self.config.font_size),
            text_color,
        );
        y += line_height;

        // Bandwidth
        let bandwidth_text = format_bandwidth(stats.bandwidth_bps);
        ui.painter().text(
            egui::pos2(pos.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            format!("BW: {}", bandwidth_text),
            egui::FontId::proportional(self.config.font_size),
            text_color,
        );
        y += line_height;

        // Frames stats
        let frames_text = format!(
            "Frames: {} ({} dropped)",
            stats.frames_displayed, stats.frames_dropped
        );
        ui.painter().text(
            egui::pos2(pos.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            frames_text,
            egui::FontId::proportional(self.config.font_size),
            text_color,
        );
        y += line_height;

        // Input events
        let input_text = format!("Input: {} events", stats.input_events_sent);
        ui.painter().text(
            egui::pos2(pos.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            input_text,
            egui::FontId::proportional(self.config.font_size),
            text_color,
        );
    }
}

/// Formats bandwidth in human-readable form
fn format_bandwidth(bps: f64) -> String {
    if bps >= 1_000_000_000.0 {
        format!("{:.1} Gbps", bps / 1_000_000_000.0)
    } else if bps >= 1_000_000.0 {
        format!("{:.1} Mbps", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.1} Kbps", bps / 1_000.0)
    } else {
        format!("{:.0} bps", bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bandwidth() {
        assert_eq!(format_bandwidth(500.0), "500 bps");
        assert_eq!(format_bandwidth(1500.0), "1.5 Kbps");
        assert_eq!(format_bandwidth(1_500_000.0), "1.5 Mbps");
        assert_eq!(format_bandwidth(1_500_000_000.0), "1.5 Gbps");
    }

    #[test]
    fn test_overlay_config_default() {
        let config = OverlayConfig::default();
        assert!(config.visible);
        assert_eq!(config.position, OverlayPosition::TopLeft);
    }

    #[test]
    fn test_overlay_visibility() {
        let mut overlay = StatusOverlay::default();
        assert!(overlay.is_visible());

        overlay.set_visible(false);
        assert!(!overlay.is_visible());
    }
}
