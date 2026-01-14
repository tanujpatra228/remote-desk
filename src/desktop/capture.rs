//! Screen capture functionality
//!
//! This module provides cross-platform screen capture using the scrap crate.

use crate::desktop::types::{CaptureConfig, CaptureStats, DisplayInfo, Frame};
use crate::error::{RemoteDeskError, Result};
use scrap::{Capturer, Display};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Default capture timeout in milliseconds
const CAPTURE_TIMEOUT_MS: u64 = 1000;

/// Maximum consecutive capture failures before stopping
const MAX_CONSECUTIVE_FAILURES: u32 = 10;

/// Screen capturer
pub struct ScreenCapturer {
    /// Capture configuration
    config: CaptureConfig,
    /// Current frame sequence number
    sequence: AtomicU64,
    /// Whether the capturer is running
    is_running: AtomicBool,
    /// Capture statistics
    stats: Arc<RwLock<CaptureStats>>,
    /// Display being captured
    display_info: DisplayInfo,
}

impl ScreenCapturer {
    /// Creates a new screen capturer
    ///
    /// # Errors
    ///
    /// Returns error if the display cannot be accessed or configuration is invalid
    pub fn new(config: CaptureConfig) -> Result<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| RemoteDeskError::Generic(format!("Invalid capture config: {}", e)))?;

        // Get display
        let display = Self::get_display(&config)?;
        let display_info = Self::get_display_info(&display)?;

        info!(
            "Created screen capturer for display '{}' ({}x{})",
            display_info.name, display_info.width, display_info.height
        );

        Ok(Self {
            config,
            sequence: AtomicU64::new(0),
            is_running: AtomicBool::new(false),
            stats: Arc::new(RwLock::new(CaptureStats::default())),
            display_info,
        })
    }

    /// Gets the display to capture based on configuration
    fn get_display(config: &CaptureConfig) -> Result<Display> {
        let displays = Display::all().map_err(|e| {
            RemoteDeskError::Generic(format!("Failed to enumerate displays: {}", e))
        })?;

        if displays.is_empty() {
            return Err(RemoteDeskError::Generic(
                "No displays available for capture".to_string(),
            ));
        }

        // Get specific display or primary
        let display = if let Some(display_id) = config.display_id {
            displays
                .into_iter()
                .nth(display_id as usize)
                .ok_or_else(|| {
                    RemoteDeskError::Generic(format!("Display {} not found", display_id))
                })?
        } else {
            Display::primary().map_err(|e| {
                RemoteDeskError::Generic(format!("Failed to get primary display: {}", e))
            })?
        };

        Ok(display)
    }

    /// Gets display information
    fn get_display_info(display: &Display) -> Result<DisplayInfo> {
        Ok(DisplayInfo {
            id: 0, // scrap doesn't provide IDs, use 0 for now
            name: "Primary Display".to_string(),
            width: display.width() as u32,
            height: display.height() as u32,
            is_primary: true,
            x: 0,
            y: 0,
        })
    }

    /// Lists all available displays
    ///
    /// # Errors
    ///
    /// Returns error if displays cannot be enumerated
    pub fn list_displays() -> Result<Vec<DisplayInfo>> {
        let displays = Display::all().map_err(|e| {
            RemoteDeskError::Generic(format!("Failed to enumerate displays: {}", e))
        })?;

        let mut display_infos = Vec::new();
        for (idx, display) in displays.iter().enumerate() {
            display_infos.push(DisplayInfo {
                id: idx as u32,
                name: format!("Display {}", idx),
                width: display.width() as u32,
                height: display.height() as u32,
                is_primary: idx == 0,
                x: 0,
                y: 0,
            });
        }

        Ok(display_infos)
    }

    /// Captures a single frame
    ///
    /// # Errors
    ///
    /// Returns error if frame capture fails
    pub async fn capture_frame(&self) -> Result<Frame> {
        let start = Instant::now();

        // Get display and create capturer
        let display = Self::get_display(&self.config)?;
        let mut capturer = Capturer::new(display).map_err(|e| {
            RemoteDeskError::Generic(format!("Failed to create capturer: {}", e))
        })?;

        let width = capturer.width();
        let height = capturer.height();

        // Try to capture frame with timeout
        let frame_data = self.capture_with_retry(&mut capturer, width, height)?;

        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        let frame = Frame::new(width as u32, height as u32, frame_data, sequence);

        // Update statistics
        let capture_time = start.elapsed().as_millis() as f64;
        let mut stats = self.stats.write().await;
        stats.frames_captured += 1;
        stats.bytes_captured += frame.size_bytes() as u64;

        // Update average capture time (moving average)
        if stats.avg_capture_time_ms == 0.0 {
            stats.avg_capture_time_ms = capture_time;
        } else {
            stats.avg_capture_time_ms = stats.avg_capture_time_ms * 0.9 + capture_time * 0.1;
        }

        debug!(
            "Captured frame {} ({}x{}, {} bytes) in {:.2}ms",
            sequence,
            frame.width,
            frame.height,
            frame.size_bytes(),
            capture_time
        );

        Ok(frame)
    }

    /// Captures frame with retry logic
    fn capture_with_retry(
        &self,
        capturer: &mut Capturer,
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>> {
        let start = Instant::now();
        let timeout = std::time::Duration::from_millis(CAPTURE_TIMEOUT_MS);

        loop {
            match capturer.frame() {
                Ok(frame) => {
                    // Convert to RGBA (scrap provides BGRA on some platforms)
                    let mut rgba_data = Vec::with_capacity(width * height * 4);

                    for chunk in frame.chunks_exact(4) {
                        // Convert BGRA to RGBA
                        rgba_data.push(chunk[2]); // R
                        rgba_data.push(chunk[1]); // G
                        rgba_data.push(chunk[0]); // B
                        rgba_data.push(chunk[3]); // A
                    }

                    return Ok(rgba_data);
                }
                Err(e) => {
                    // Check if the error is a "would block" type error
                    let error_msg = e.to_string();
                    if error_msg.contains("WouldBlock") || error_msg.contains("would block") {
                        // Frame not ready yet, check timeout
                        if start.elapsed() > timeout {
                            return Err(RemoteDeskError::Generic(
                                "Frame capture timeout".to_string(),
                            ));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    } else {
                        return Err(RemoteDeskError::Generic(format!(
                            "Failed to capture frame: {}",
                            e
                        )));
                    }
                }
            }
        }
    }

    /// Starts continuous frame capture
    ///
    /// Returns a channel receiver for captured frames
    pub fn start_capture(&self) -> tokio::sync::mpsc::Receiver<Frame> {
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("Capturer is already running");
            return rx;
        }

        let config = self.config.clone();
        let stats = Arc::clone(&self.stats);
        let sequence = Arc::new(AtomicU64::new(0));
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = Arc::clone(&is_running);

        // Use std::thread for blocking screen capture
        std::thread::spawn(move || {
            info!("Starting continuous screen capture at {} FPS", config.fps);

            let frame_interval = config.frame_interval();
            let mut consecutive_failures = 0;

            // Create capturer once
            let display = match Self::get_display(&config) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to get display: {}", e);
                    return;
                }
            };

            let mut capturer = match Capturer::new(display) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create capturer: {}", e);
                    return;
                }
            };

            let width = capturer.width();
            let height = capturer.height();

            while is_running_clone.load(Ordering::SeqCst) {
                let frame_start = Instant::now();

                // Capture frame
                match Self::capture_single_frame_blocking(
                    &mut capturer,
                    width,
                    height,
                    &sequence,
                    &stats,
                ) {
                    Ok(frame) => {
                        consecutive_failures = 0;

                        // Send frame (blocking)
                        if tx.blocking_send(frame).is_err() {
                            debug!("Frame receiver closed, stopping capture");
                            break;
                        }
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        error!("Failed to capture frame: {}", e);

                        if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                            error!(
                                "Too many consecutive failures ({}), stopping capture",
                                consecutive_failures
                            );
                            break;
                        }

                        // Update dropped frame stat
                        if let Ok(mut stats) = stats.try_write() {
                            stats.frames_dropped += 1;
                        }
                    }
                }

                // Sleep to maintain target FPS
                let elapsed = frame_start.elapsed();
                if elapsed < frame_interval {
                    std::thread::sleep(frame_interval - elapsed);
                }
            }

            info!("Screen capture stopped");
        });

        rx
    }

    /// Helper function to capture a single frame (blocking, for use in std::thread)
    fn capture_single_frame_blocking(
        capturer: &mut Capturer,
        width: usize,
        height: usize,
        sequence: &Arc<AtomicU64>,
        stats: &Arc<RwLock<CaptureStats>>,
    ) -> Result<Frame> {
        let start = Instant::now();

        // Capture frame
        let timeout = std::time::Duration::from_millis(CAPTURE_TIMEOUT_MS);
        let frame_start = Instant::now();

        let frame_data = loop {
            match capturer.frame() {
                Ok(frame) => {
                    let mut rgba_data = Vec::with_capacity(width * height * 4);

                    for chunk in frame.chunks_exact(4) {
                        rgba_data.push(chunk[2]); // R
                        rgba_data.push(chunk[1]); // G
                        rgba_data.push(chunk[0]); // B
                        rgba_data.push(chunk[3]); // A
                    }

                    break rgba_data;
                }
                Err(e) => {
                    // Check if the error is a "would block" type error
                    let error_msg = e.to_string();
                    if error_msg.contains("WouldBlock") || error_msg.contains("would block") {
                        if frame_start.elapsed() > timeout {
                            return Err(RemoteDeskError::Generic(
                                "Frame capture timeout".to_string(),
                            ));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    } else {
                        return Err(RemoteDeskError::Generic(format!(
                            "Failed to capture frame: {}",
                            e
                        )));
                    }
                }
            }
        };

        let seq = sequence.fetch_add(1, Ordering::SeqCst);
        let frame = Frame::new(width as u32, height as u32, frame_data, seq);

        // Update statistics (use try_write to avoid blocking)
        let capture_time = start.elapsed().as_millis() as f64;
        if let Ok(mut stats) = stats.try_write() {
            stats.frames_captured += 1;
            stats.bytes_captured += frame.size_bytes() as u64;

            if stats.avg_capture_time_ms == 0.0 {
                stats.avg_capture_time_ms = capture_time;
            } else {
                stats.avg_capture_time_ms = stats.avg_capture_time_ms * 0.9 + capture_time * 0.1;
            }
        }

        Ok(frame)
    }

    /// Stops continuous capture
    pub fn stop_capture(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        info!("Stopping screen capture");
    }

    /// Returns current capture statistics
    pub async fn get_stats(&self) -> CaptureStats {
        self.stats.read().await.clone()
    }

    /// Returns display information
    pub fn display_info(&self) -> &DisplayInfo {
        &self.display_info
    }

    /// Returns the capture configuration
    pub fn config(&self) -> &CaptureConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_displays() {
        // This test may fail in CI environments without displays
        match ScreenCapturer::list_displays() {
            Ok(displays) => {
                println!("Found {} display(s)", displays.len());
                for display in displays {
                    println!(
                        "  Display {}: {}x{}",
                        display.id, display.width, display.height
                    );
                }
            }
            Err(e) => {
                println!("Could not enumerate displays (expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_capture_config_validation() {
        let valid_config = CaptureConfig::new(30, 80);
        assert!(valid_config.validate().is_ok());
    }
}
