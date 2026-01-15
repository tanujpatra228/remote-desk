//! Session types and implementation
//!
//! This module defines the remote desktop session types and logic.

use crate::desktop::{CaptureConfig, FrameEncoder, FrameFormat, ScreenCapturer};
use crate::error::{RemoteDeskError, Result};
use crate::input::{InputEvent, InputSimulator, Key, KeyboardEvent, KeyboardEventType, MouseButton, MouseEvent, MouseEventType};
use crate::network::{
    FrameFormat as NetFrameFormat, KeyboardEventData, KeyboardEventTypeData, MouseEventData,
    MouseEventTypeData, ScreenFrameData,
};
use crate::security::DeviceId;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Session mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Host mode - being controlled (captures screen, receives input)
    Host,
    /// Client mode - controlling (receives frames, sends input)
    Client,
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Session mode
    pub mode: SessionMode,
    /// Remote device ID
    pub remote_id: DeviceId,
    /// Local device ID
    pub local_id: DeviceId,
    /// Capture configuration (for host mode)
    pub capture_config: CaptureConfig,
}

impl SessionConfig {
    /// Creates a new host mode configuration
    pub fn host(local_id: DeviceId, remote_id: DeviceId, capture_config: CaptureConfig) -> Self {
        Self {
            mode: SessionMode::Host,
            remote_id,
            local_id,
            capture_config,
        }
    }

    /// Creates a new client mode configuration
    pub fn client(local_id: DeviceId, remote_id: DeviceId) -> Self {
        Self {
            mode: SessionMode::Client,
            remote_id,
            local_id,
            capture_config: CaptureConfig::default(), // Not used in client mode
        }
    }
}

/// Session statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    /// Total frames sent (host) or received (client)
    pub frames_processed: u64,
    /// Total input events sent (client) or received (host)
    pub input_events_processed: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Session start time (milliseconds since epoch)
    pub session_start_time: u64,
    /// Last activity time (milliseconds since epoch)
    pub last_activity_time: u64,
}

impl SessionStats {
    /// Returns session duration in seconds
    pub fn duration_secs(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let start = self.session_start_time / 1000;
        now.saturating_sub(start)
    }

    /// Returns average FPS (frames per second)
    pub fn average_fps(&self) -> f64 {
        let duration = self.duration_secs();
        if duration == 0 {
            return 0.0;
        }
        self.frames_processed as f64 / duration as f64
    }

    /// Returns average bandwidth in bytes per second
    pub fn average_bandwidth_bps(&self) -> f64 {
        let duration = self.duration_secs();
        if duration == 0 {
            return 0.0;
        }
        (self.bytes_sent + self.bytes_received) as f64 / duration as f64
    }
}

/// Remote desktop session
pub struct Session {
    /// Session configuration
    config: SessionConfig,
    /// Whether the session is active
    is_active: Arc<AtomicBool>,
    /// Session statistics
    stats: Arc<RwLock<SessionStats>>,
    /// Frame sequence counter
    frame_sequence: Arc<AtomicU64>,
    /// Screen capturer (host mode only)
    capturer: Option<ScreenCapturer>,
    /// Frame encoder (host mode only)
    encoder: Option<FrameEncoder>,
    /// Input simulator (host mode only)
    input_simulator: Option<InputSimulator>,
}

impl Session {
    /// Creates a new session
    ///
    /// # Errors
    ///
    /// Returns error if session initialization fails
    pub fn new(config: SessionConfig) -> Result<Self> {
        let mut session = Self {
            config: config.clone(),
            is_active: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(SessionStats::default())),
            frame_sequence: Arc::new(AtomicU64::new(0)),
            capturer: None,
            encoder: None,
            input_simulator: None,
        };

        // Initialize mode-specific components
        match config.mode {
            SessionMode::Host => {
                // Create screen capturer
                let capturer = ScreenCapturer::new(config.capture_config.clone())?;
                session.capturer = Some(capturer);

                // Create frame encoder
                let encoder =
                    FrameEncoder::new(config.capture_config.format, config.capture_config.quality);
                session.encoder = Some(encoder);

                // Create input simulator
                let input_simulator = InputSimulator::new();
                session.input_simulator = Some(input_simulator);

                info!(
                    "Created host session for remote device {}",
                    config.remote_id.format_with_spaces()
                );
            }
            SessionMode::Client => {
                info!(
                    "Created client session to remote device {}",
                    config.remote_id.format_with_spaces()
                );
            }
        }

        Ok(session)
    }

    /// Starts the session
    ///
    /// # Errors
    ///
    /// Returns error if session start fails
    pub async fn start(&mut self) -> Result<()> {
        if self
            .is_active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(RemoteDeskError::Generic(
                "Session already active".to_string(),
            ));
        }

        // Initialize session stats
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut stats = self.stats.write().await;
        stats.session_start_time = now;
        stats.last_activity_time = now;
        drop(stats);

        info!("Session started in {:?} mode", self.config.mode);
        Ok(())
    }

    /// Stops the session
    pub async fn stop(&mut self) {
        self.is_active.store(false, Ordering::SeqCst);
        info!("Session stopped");
    }

    /// Returns whether the session is active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Returns session statistics
    pub async fn stats(&self) -> SessionStats {
        self.stats.read().await.clone()
    }

    /// Captures and encodes a single frame (host mode)
    ///
    /// # Errors
    ///
    /// Returns error if frame capture or encoding fails
    pub async fn capture_frame(&self) -> Result<ScreenFrameData> {
        if self.config.mode != SessionMode::Host {
            return Err(RemoteDeskError::Generic(
                "Frame capture only available in host mode".to_string(),
            ));
        }

        let capturer = self.capturer.as_ref().ok_or_else(|| {
            RemoteDeskError::Generic("Screen capturer not initialized".to_string())
        })?;

        let encoder = self.encoder.as_ref().ok_or_else(|| {
            RemoteDeskError::Generic("Frame encoder not initialized".to_string())
        })?;

        // Capture frame
        let frame = capturer.capture_frame().await?;
        let frame_size = frame.size_bytes();

        // Encode frame
        let encoded = encoder.encode(&frame)?;
        let encoded_size = encoded.data.len();

        // Convert to network message
        let net_format = match encoded.format {
            FrameFormat::Jpeg => NetFrameFormat::Jpeg,
            FrameFormat::Png => NetFrameFormat::Png,
            FrameFormat::Raw => NetFrameFormat::Raw,
            FrameFormat::WebP => NetFrameFormat::Jpeg, // Fallback
        };

        let frame_data = ScreenFrameData::new(
            encoded.sequence,
            encoded.width,
            encoded.height,
            net_format,
            encoded.data,
        );

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.frames_processed += 1;
        stats.bytes_sent += encoded_size as u64;
        stats.last_activity_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        debug!(
            "Captured and encoded frame {} ({} bytes -> {} bytes, {:.1}% compression)",
            frame_data.sequence,
            frame_size,
            encoded_size,
            (1.0 - (encoded_size as f64 / frame_size as f64)) * 100.0
        );

        Ok(frame_data)
    }

    /// Starts continuous frame capture (host mode)
    ///
    /// Returns a channel receiver for encoded frames
    pub fn start_frame_stream(&self) -> Result<mpsc::Receiver<ScreenFrameData>> {
        if self.config.mode != SessionMode::Host {
            return Err(RemoteDeskError::Generic(
                "Frame streaming only available in host mode".to_string(),
            ));
        }

        let capturer = self.capturer.as_ref().ok_or_else(|| {
            RemoteDeskError::Generic("Screen capturer not initialized".to_string())
        })?;

        let encoder = self.encoder.as_ref().ok_or_else(|| {
            RemoteDeskError::Generic("Frame encoder not initialized".to_string())
        })?;

        let (tx, rx) = mpsc::channel(10);

        // Start capture
        let mut frame_rx = capturer.start_capture();
        let encoder_format = encoder.format();
        let encoder_quality = encoder.quality();
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            let encoder = FrameEncoder::new(encoder_format, encoder_quality);

            while let Some(frame) = frame_rx.recv().await {
                let frame_size = frame.size_bytes();

                match encoder.encode(&frame) {
                    Ok(encoded) => {
                        let encoded_size = encoded.data.len();

                        let net_format = match encoded.format {
                            FrameFormat::Jpeg => NetFrameFormat::Jpeg,
                            FrameFormat::Png => NetFrameFormat::Png,
                            FrameFormat::Raw => NetFrameFormat::Raw,
                            FrameFormat::WebP => NetFrameFormat::Jpeg,
                        };

                        let frame_data = ScreenFrameData::new(
                            encoded.sequence,
                            encoded.width,
                            encoded.height,
                            net_format,
                            encoded.data,
                        );

                        // Update statistics
                        if let Ok(mut stats) = stats.try_write() {
                            stats.frames_processed += 1;
                            stats.bytes_sent += encoded_size as u64;
                            stats.last_activity_time = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;
                        }

                        debug!(
                            "Encoded frame {} ({} -> {} bytes, {:.1}% compression)",
                            frame_data.sequence,
                            frame_size,
                            encoded_size,
                            (1.0 - (encoded_size as f64 / frame_size as f64)) * 100.0
                        );

                        if tx.send(frame_data).await.is_err() {
                            debug!("Frame stream receiver closed");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to encode frame: {}", e);
                    }
                }
            }

            info!("Frame streaming stopped");
        });

        Ok(rx)
    }

    /// Processes a received input event (host mode)
    ///
    /// # Errors
    ///
    /// Returns error if input simulation fails
    pub async fn process_input(&self, event: &InputEvent) -> Result<()> {
        if self.config.mode != SessionMode::Host {
            return Err(RemoteDeskError::Generic(
                "Input processing only available in host mode".to_string(),
            ));
        }

        let simulator = self.input_simulator.as_ref().ok_or_else(|| {
            RemoteDeskError::Generic("Input simulator not initialized".to_string())
        })?;

        // Simulate the input event
        simulator.simulate(event)?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.input_events_processed += 1;
        stats.last_activity_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        debug!("Processed input event: {:?}", event);

        Ok(())
    }

    /// Processes a received keyboard event from network (host mode)
    pub async fn process_keyboard_event(&self, event: &KeyboardEventData) -> Result<()> {
        let event_type = match event.event_type {
            KeyboardEventTypeData::KeyPress => KeyboardEventType::KeyPress,
            KeyboardEventTypeData::KeyRelease => KeyboardEventType::KeyRelease,
        };

        // Convert key code to Key enum
        // This is a simplified mapping - in production you'd want a proper conversion
        let key = Self::key_from_code(event.key)?;

        let kb_event = KeyboardEvent {
            event_type,
            key,
            timestamp: event.timestamp,
        };

        self.process_input(&InputEvent::Keyboard(kb_event)).await
    }

    /// Processes a received mouse event from network (host mode)
    pub async fn process_mouse_event(&self, event: &MouseEventData) -> Result<()> {
        let event_type = match &event.event_type {
            MouseEventTypeData::Move { x, y } => MouseEventType::Move { x: *x, y: *y },
            MouseEventTypeData::ButtonPress { button } => {
                let btn = Self::button_from_code(*button)?;
                MouseEventType::ButtonPress { button: btn }
            }
            MouseEventTypeData::ButtonRelease { button } => {
                let btn = Self::button_from_code(*button)?;
                MouseEventType::ButtonRelease { button: btn }
            }
            MouseEventTypeData::Wheel { delta_x, delta_y } => MouseEventType::Wheel {
                delta_x: *delta_x,
                delta_y: *delta_y,
            },
        };

        let mouse_event = MouseEvent {
            event_type,
            timestamp: event.timestamp,
        };

        self.process_input(&InputEvent::Mouse(mouse_event)).await
    }

    /// Converts key code to Key enum (simplified mapping)
    fn key_from_code(code: u16) -> Result<Key> {
        let key = match code {
            0x41 => Key::A,
            0x42 => Key::B,
            0x43 => Key::C,
            0x44 => Key::D,
            0x45 => Key::E,
            0x46 => Key::F,
            0x47 => Key::G,
            0x48 => Key::H,
            0x49 => Key::I,
            0x4A => Key::J,
            0x4B => Key::K,
            0x4C => Key::L,
            0x4D => Key::M,
            0x4E => Key::N,
            0x4F => Key::O,
            0x50 => Key::P,
            0x51 => Key::Q,
            0x52 => Key::R,
            0x53 => Key::S,
            0x54 => Key::T,
            0x55 => Key::U,
            0x56 => Key::V,
            0x57 => Key::W,
            0x58 => Key::X,
            0x59 => Key::Y,
            0x5A => Key::Z,
            0x30 => Key::Num0,
            0x31 => Key::Num1,
            0x32 => Key::Num2,
            0x33 => Key::Num3,
            0x34 => Key::Num4,
            0x35 => Key::Num5,
            0x36 => Key::Num6,
            0x37 => Key::Num7,
            0x38 => Key::Num8,
            0x39 => Key::Num9,
            0x0D => Key::Return,
            0x20 => Key::Space,
            0x1B => Key::Escape,
            0x08 => Key::Backspace,
            0x09 => Key::Tab,
            0x10 => Key::Shift,
            0x11 => Key::Control,
            0x12 => Key::Alt,
            _ => {
                warn!("Unknown key code: 0x{:04X}", code);
                Key::Unknown
            }
        };

        Ok(key)
    }

    /// Converts button code to MouseButton enum
    fn button_from_code(code: u8) -> Result<MouseButton> {
        let button = match code {
            1 => MouseButton::Left,
            2 => MouseButton::Right,
            3 => MouseButton::Middle,
            4 => MouseButton::Button4,
            5 => MouseButton::Button5,
            _ => {
                warn!("Unknown mouse button code: {}", code);
                MouseButton::Left // Fallback
            }
        };

        Ok(button)
    }

    /// Returns the session configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::FrameFormat;

    #[test]
    fn test_session_config_creation() {
        let local_id = DeviceId::from_u32(123456789).unwrap();
        let remote_id = DeviceId::from_u32(987654321).unwrap();

        let host_config = SessionConfig::host(
            local_id,
            remote_id,
            CaptureConfig::new(30, 80).with_format(FrameFormat::Jpeg),
        );
        assert_eq!(host_config.mode, SessionMode::Host);

        let client_config = SessionConfig::client(local_id, remote_id);
        assert_eq!(client_config.mode, SessionMode::Client);
    }

    #[tokio::test]
    async fn test_session_stats() {
        let mut stats = SessionStats::default();
        stats.session_start_time = 0;
        stats.frames_processed = 100;

        assert!(stats.duration_secs() > 0);
    }

    #[test]
    fn test_key_conversion() {
        assert!(matches!(Session::key_from_code(0x41), Ok(Key::A)));
        assert!(matches!(Session::key_from_code(0x0D), Ok(Key::Return)));
        assert!(matches!(Session::key_from_code(0x20), Ok(Key::Space)));
    }

    #[test]
    fn test_button_conversion() {
        assert!(matches!(
            Session::button_from_code(1),
            Ok(MouseButton::Left)
        ));
        assert!(matches!(
            Session::button_from_code(2),
            Ok(MouseButton::Right)
        ));
        assert!(matches!(
            Session::button_from_code(3),
            Ok(MouseButton::Middle)
        ));
    }
}
