//! Desktop types and data structures
//!
//! This module defines common types used across the desktop layer.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Frame quality setting (0-100)
pub type Quality = u8;

/// Frames per second setting
pub type Fps = u8;

/// Valid quality range constants
pub const MIN_QUALITY: Quality = 1;
pub const MAX_QUALITY: Quality = 100;
pub const DEFAULT_QUALITY: Quality = 80;

/// Valid FPS range constants
pub const MIN_FPS: Fps = 1;
pub const MAX_FPS: Fps = 60;
pub const DEFAULT_FPS: Fps = 30;

/// Display information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// Display identifier
    pub id: u32,
    /// Display name
    pub name: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Whether this is the primary display
    pub is_primary: bool,
    /// X position in virtual screen coordinate
    pub x: i32,
    /// Y position in virtual screen coordinate
    pub y: i32,
}

/// A captured frame from the screen
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Raw RGBA pixel data (4 bytes per pixel)
    pub data: Vec<u8>,
    /// Frame sequence number for ordering
    pub sequence: u64,
    /// Timestamp when frame was captured
    pub timestamp: std::time::Instant,
}

impl Frame {
    /// Creates a new frame
    pub fn new(width: u32, height: u32, data: Vec<u8>, sequence: u64) -> Self {
        Self {
            width,
            height,
            data,
            sequence,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Returns the size of the frame data in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// Validates that the frame data matches the dimensions
    pub fn is_valid(&self) -> bool {
        let expected_size = (self.width * self.height * 4) as usize;
        self.data.len() == expected_size
    }
}

/// Encoded frame ready for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedFrame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Compressed frame data
    pub data: Vec<u8>,
    /// Frame sequence number
    pub sequence: u64,
    /// Encoding format used
    pub format: FrameFormat,
    /// Original uncompressed size (for stats)
    pub original_size: usize,
}

impl EncodedFrame {
    /// Returns the compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        self.data.len() as f64 / self.original_size as f64
    }

    /// Returns the compression percentage
    pub fn compression_percentage(&self) -> f64 {
        (1.0 - self.compression_ratio()) * 100.0
    }
}

/// Frame encoding format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameFormat {
    /// Raw RGBA (no compression)
    Raw = 0,
    /// JPEG compression
    Jpeg = 1,
    /// PNG compression (lossless)
    Png = 2,
    /// WebP compression
    WebP = 3,
}

/// Capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target frames per second
    pub fps: Fps,
    /// Image quality (1-100, higher is better)
    pub quality: Quality,
    /// Specific display to capture (None = primary display)
    pub display_id: Option<u32>,
    /// Frame encoding format
    pub format: FrameFormat,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            fps: DEFAULT_FPS,
            quality: DEFAULT_QUALITY,
            display_id: None,
            format: FrameFormat::Jpeg,
        }
    }
}

impl CaptureConfig {
    /// Creates a new capture configuration
    pub fn new(fps: Fps, quality: Quality) -> Self {
        Self {
            fps: fps.clamp(MIN_FPS, MAX_FPS),
            quality: quality.clamp(MIN_QUALITY, MAX_QUALITY),
            display_id: None,
            format: FrameFormat::Jpeg,
        }
    }

    /// Sets the display to capture
    pub fn with_display(mut self, display_id: u32) -> Self {
        self.display_id = Some(display_id);
        self
    }

    /// Sets the frame format
    pub fn with_format(mut self, format: FrameFormat) -> Self {
        self.format = format;
        self
    }

    /// Returns the frame interval duration
    pub fn frame_interval(&self) -> Duration {
        Duration::from_millis(1000 / self.fps as u64)
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.fps < MIN_FPS || self.fps > MAX_FPS {
            return Err(format!(
                "FPS must be between {} and {}",
                MIN_FPS, MAX_FPS
            ));
        }

        if self.quality < MIN_QUALITY || self.quality > MAX_QUALITY {
            return Err(format!(
                "Quality must be between {} and {}",
                MIN_QUALITY, MAX_QUALITY
            ));
        }

        Ok(())
    }
}

/// Capture statistics
#[derive(Debug, Default, Clone)]
pub struct CaptureStats {
    /// Total frames captured
    pub frames_captured: u64,
    /// Total frames dropped
    pub frames_dropped: u64,
    /// Total bytes captured (uncompressed)
    pub bytes_captured: u64,
    /// Total bytes encoded (compressed)
    pub bytes_encoded: u64,
    /// Average capture time in milliseconds
    pub avg_capture_time_ms: f64,
    /// Average encoding time in milliseconds
    pub avg_encoding_time_ms: f64,
}

impl CaptureStats {
    /// Returns the average compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_captured == 0 {
            return 0.0;
        }
        self.bytes_encoded as f64 / self.bytes_captured as f64
    }

    /// Returns the drop rate percentage
    pub fn drop_rate(&self) -> f64 {
        let total = self.frames_captured + self.frames_dropped;
        if total == 0 {
            return 0.0;
        }
        (self.frames_dropped as f64 / total as f64) * 100.0
    }

    /// Returns the actual FPS
    pub fn actual_fps(&self) -> f64 {
        if self.avg_capture_time_ms == 0.0 {
            return 0.0;
        }
        1000.0 / self.avg_capture_time_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_validation() {
        let data = vec![0u8; 1920 * 1080 * 4];
        let frame = Frame::new(1920, 1080, data, 1);
        assert!(frame.is_valid());

        let invalid_data = vec![0u8; 100];
        let invalid_frame = Frame::new(1920, 1080, invalid_data, 1);
        assert!(!invalid_frame.is_valid());
    }

    #[test]
    fn test_capture_config_validation() {
        let config = CaptureConfig::new(30, 80);
        assert!(config.validate().is_ok());

        let invalid_config = CaptureConfig {
            fps: 0,
            quality: 150,
            display_id: None,
            format: FrameFormat::Jpeg,
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_compression_ratio() {
        let encoded = EncodedFrame {
            width: 1920,
            height: 1080,
            data: vec![0u8; 100_000],
            sequence: 1,
            format: FrameFormat::Jpeg,
            original_size: 1_000_000,
        };

        assert_eq!(encoded.compression_ratio(), 0.1);
        assert_eq!(encoded.compression_percentage(), 90.0);
    }

    #[test]
    fn test_frame_interval() {
        let config = CaptureConfig::new(30, 80);
        assert_eq!(config.frame_interval(), Duration::from_millis(33));

        let config = CaptureConfig::new(60, 80);
        assert_eq!(config.frame_interval(), Duration::from_millis(16));
    }

    #[test]
    fn test_capture_stats() {
        let mut stats = CaptureStats::default();
        stats.frames_captured = 90;
        stats.frames_dropped = 10;
        stats.bytes_captured = 1_000_000;
        stats.bytes_encoded = 100_000;

        assert_eq!(stats.drop_rate(), 10.0);
        assert_eq!(stats.compression_ratio(), 0.1);
    }
}
