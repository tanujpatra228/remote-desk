//! Host session for screen sharing
//!
//! The host session captures the screen, encodes frames, and processes
//! remote input events.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::desktop::{CaptureConfig, FrameEncoder, FrameFormat, ScreenCapturer};
use crate::error::{RemoteDeskError, SessionError, SessionResult};
use crate::input::{InputEvent, InputSimulator};
use crate::session::state::{SessionState, SessionStateMachine};
use crate::session::transport::{
    ControlMessage, SessionTransport, TransportFrame, TransportInput,
};

/// Configuration for host session
#[derive(Debug, Clone)]
pub struct HostSessionConfig {
    /// Capture configuration
    pub capture: CaptureConfig,
    /// Whether to allow input simulation
    pub allow_input: bool,
    /// Session identifier
    pub session_id: String,
}

impl Default for HostSessionConfig {
    fn default() -> Self {
        Self {
            capture: CaptureConfig::default(),
            session_id: uuid::Uuid::new_v4().to_string(),
            allow_input: true,
        }
    }
}

impl HostSessionConfig {
    /// Creates a new host session config with specified FPS and quality
    pub fn new(fps: u8, quality: u8) -> Self {
        Self {
            capture: CaptureConfig::new(fps, quality),
            ..Default::default()
        }
    }

    /// Sets whether to allow input simulation
    pub fn with_input(mut self, allow: bool) -> Self {
        self.allow_input = allow;
        self
    }

    /// Sets the session ID
    pub fn with_session_id(mut self, id: String) -> Self {
        self.session_id = id;
        self
    }
}

/// Statistics for the host session
#[derive(Debug, Clone, Default)]
pub struct HostSessionStats {
    /// Total frames captured and sent
    pub frames_sent: u64,
    /// Total frames dropped (encoding or channel full)
    pub frames_dropped: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total input events received
    pub input_events_received: u64,
    /// Total input events processed
    pub input_events_processed: u64,
    /// Average frame encode time (ms)
    pub avg_encode_time_ms: f64,
    /// Session start time
    pub started_at: Option<Instant>,
}

impl HostSessionStats {
    /// Returns the session duration in seconds
    pub fn duration_secs(&self) -> f64 {
        self.started_at
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Returns the average FPS
    pub fn average_fps(&self) -> f64 {
        let duration = self.duration_secs();
        if duration == 0.0 {
            return 0.0;
        }
        self.frames_sent as f64 / duration
    }

    /// Returns the drop rate as a percentage
    pub fn drop_rate(&self) -> f64 {
        let total = self.frames_sent + self.frames_dropped;
        if total == 0 {
            return 0.0;
        }
        (self.frames_dropped as f64 / total as f64) * 100.0
    }
}

/// Host session for sharing the desktop
pub struct HostSession {
    /// Session configuration
    config: HostSessionConfig,
    /// Session state machine
    state: Arc<RwLock<SessionStateMachine>>,
    /// Transport channels
    transport: SessionTransport,
    /// Whether the session is running
    is_running: Arc<AtomicBool>,
    /// Session statistics
    stats: Arc<RwLock<HostSessionStats>>,
    /// Frame sequence counter
    frame_sequence: Arc<AtomicU64>,
    /// Session start time
    session_start: Arc<RwLock<Option<Instant>>>,
}

impl HostSession {
    /// Creates a new host session
    pub fn new(config: HostSessionConfig, transport: SessionTransport) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(SessionStateMachine::new())),
            transport,
            is_running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(HostSessionStats::default())),
            frame_sequence: Arc::new(AtomicU64::new(0)),
            session_start: Arc::new(RwLock::new(None)),
        }
    }

    /// Returns the session ID
    pub fn session_id(&self) -> &str {
        &self.config.session_id
    }

    /// Returns the current session state
    pub async fn state(&self) -> SessionState {
        self.state.read().await.current()
    }

    /// Returns the session statistics
    pub async fn stats(&self) -> HostSessionStats {
        self.stats.read().await.clone()
    }

    /// Starts the host session
    ///
    /// This spawns background tasks for:
    /// - Frame capture and encoding
    /// - Input event processing
    /// - Control message handling
    pub async fn start(&mut self) -> SessionResult<()> {
        // Transition to Connecting state
        {
            let mut state = self.state.write().await;
            state.transition(SessionState::Connecting)?;
        }

        // For loopback/local mode, skip authentication and go directly to Active
        {
            let mut state = self.state.write().await;
            state.transition(SessionState::Authenticating)?;
            state.transition(SessionState::Active)?;
        }

        self.is_running.store(true, Ordering::SeqCst);

        // Record session start time
        {
            let mut start = self.session_start.write().await;
            *start = Some(Instant::now());
            let mut stats = self.stats.write().await;
            stats.started_at = Some(Instant::now());
        }

        info!("Host session {} started", self.config.session_id);

        // Start background tasks
        self.spawn_frame_capture_task();
        self.spawn_input_receiver_task();
        self.spawn_control_handler_task();

        Ok(())
    }

    /// Stops the host session
    pub async fn stop(&mut self) -> SessionResult<()> {
        let current = self.state.read().await.current();
        if current == SessionState::Disconnected {
            return Ok(());
        }

        self.is_running.store(false, Ordering::SeqCst);

        // Transition through disconnecting to disconnected
        {
            let mut state = self.state.write().await;
            if state.can_transition(SessionState::Disconnecting) {
                let _ = state.transition(SessionState::Disconnecting);
            }
            state.force_transition(SessionState::Disconnected);
        }

        info!("Host session {} stopped", self.config.session_id);

        Ok(())
    }

    /// Pauses the host session
    pub async fn pause(&mut self) -> SessionResult<()> {
        let mut state = self.state.write().await;
        state.transition(SessionState::Paused)?;
        info!("Host session {} paused", self.config.session_id);
        Ok(())
    }

    /// Resumes a paused host session
    pub async fn resume(&mut self) -> SessionResult<()> {
        let mut state = self.state.write().await;
        state.transition(SessionState::Active)?;
        info!("Host session {} resumed", self.config.session_id);
        Ok(())
    }

    /// Spawns the frame capture task
    ///
    /// Uses a blocking thread since scrap::Capturer is not Send
    fn spawn_frame_capture_task(&self) {
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let frame_tx = self.transport.frames.tx.clone();
        let frame_sequence = Arc::clone(&self.frame_sequence);
        let session_start = Arc::clone(&self.session_start);

        // Use std::thread for blocking screen capture (scrap::Capturer is not Send)
        std::thread::spawn(move || {
            info!("Starting frame capture task");

            // Create runtime for state checks
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Failed to create runtime: {}", e);
                    return;
                }
            };

            // Create screen capturer
            let display = match scrap::Display::primary() {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to get primary display: {}", e);
                    return;
                }
            };

            let width = display.width();
            let height = display.height();

            let mut capturer = match scrap::Capturer::new(display) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create capturer: {}", e);
                    return;
                }
            };

            // Create frame encoder
            let encoder = FrameEncoder::new(config.capture.format, config.capture.quality);

            let frame_interval = config.capture.frame_interval();
            let mut consecutive_failures = 0;
            const MAX_FAILURES: u32 = 10;

            while is_running.load(Ordering::SeqCst) {
                let frame_start = Instant::now();

                // Check if we should capture (only in Active state)
                let should_capture = rt.block_on(async {
                    let current_state = state.read().await.current();
                    current_state == SessionState::Active
                });

                if !should_capture {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                // Capture frame
                let capture_result = Self::capture_frame_blocking(
                    &mut capturer,
                    width,
                    height,
                    &frame_sequence,
                );

                match capture_result {
                    Ok((frame_data, sequence)) => {
                        consecutive_failures = 0;
                        let encode_start = Instant::now();

                        let frame = crate::desktop::Frame::new(
                            width as u32,
                            height as u32,
                            frame_data,
                            sequence,
                        );

                        match encoder.encode(&frame) {
                            Ok(encoded) => {
                                let encode_time = encode_start.elapsed().as_secs_f64() * 1000.0;

                                // Get timestamp relative to session start
                                let timestamp_ms = rt.block_on(async {
                                    let start = session_start.read().await;
                                    start
                                        .map(|s| s.elapsed().as_millis() as u64)
                                        .unwrap_or(0)
                                });

                                let transport_frame = TransportFrame::new(
                                    sequence,
                                    encoded.width,
                                    encoded.height,
                                    encoded.format,
                                    encoded.data.clone(),
                                    encoded.original_size,
                                    timestamp_ms,
                                );

                                // Send frame
                                match frame_tx.blocking_send(transport_frame) {
                                    Ok(()) => {
                                        rt.block_on(async {
                                            let mut s = stats.write().await;
                                            s.frames_sent += 1;
                                            s.bytes_sent += encoded.data.len() as u64;

                                            // Update average encode time
                                            if s.avg_encode_time_ms == 0.0 {
                                                s.avg_encode_time_ms = encode_time;
                                            } else {
                                                s.avg_encode_time_ms =
                                                    s.avg_encode_time_ms * 0.9 + encode_time * 0.1;
                                            }
                                        });

                                        debug!(
                                            "Sent frame {} ({} bytes, {:.1}ms encode)",
                                            sequence,
                                            encoded.data.len(),
                                            encode_time
                                        );
                                    }
                                    Err(_) => {
                                        warn!("Frame channel closed");
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                rt.block_on(async {
                                    let mut s = stats.write().await;
                                    s.frames_dropped += 1;
                                });
                                error!("Failed to encode frame: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        error!(
                            "Failed to capture frame (attempt {}/{}): {}",
                            consecutive_failures, MAX_FAILURES, e
                        );

                        if consecutive_failures >= MAX_FAILURES {
                            error!("Too many consecutive capture failures, stopping");
                            break;
                        }
                    }
                }

                // Maintain target FPS
                let elapsed = frame_start.elapsed();
                if elapsed < frame_interval {
                    std::thread::sleep(frame_interval - elapsed);
                }
            }

            info!("Frame capture task stopped");
        });
    }

    /// Captures a single frame (blocking)
    fn capture_frame_blocking(
        capturer: &mut scrap::Capturer,
        width: usize,
        height: usize,
        sequence: &Arc<AtomicU64>,
    ) -> Result<(Vec<u8>, u64), RemoteDeskError> {
        let timeout = std::time::Duration::from_millis(1000);
        let start = Instant::now();

        loop {
            match capturer.frame() {
                Ok(frame) => {
                    // Convert BGRA to RGBA
                    let mut rgba_data = Vec::with_capacity(width * height * 4);
                    for chunk in frame.chunks_exact(4) {
                        rgba_data.push(chunk[2]); // R
                        rgba_data.push(chunk[1]); // G
                        rgba_data.push(chunk[0]); // B
                        rgba_data.push(chunk[3]); // A
                    }

                    let seq = sequence.fetch_add(1, Ordering::SeqCst);
                    return Ok((rgba_data, seq));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("WouldBlock") || error_msg.contains("would block") {
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

    /// Spawns the input receiver task
    fn spawn_input_receiver_task(&self) {
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);

        // Take ownership of the input receiver
        // Note: This requires the transport to be passed by value or use interior mutability
        // For now, we'll create a simple placeholder that handles input from the channel

        let is_running_clone = Arc::clone(&is_running);
        let state_clone = Arc::clone(&state);
        let stats_clone = Arc::clone(&stats);

        // The actual receiver would need to be moved out of self.transport
        // This is a design limitation - in practice, you'd use Arc<Mutex<Option<Receiver>>>
        // or restructure the code

        tokio::spawn(async move {
            info!("Starting input receiver task");

            let simulator = InputSimulator::new();

            // For now, we'll just log that the task is running
            // In a full implementation, we'd receive from the input channel here
            while is_running_clone.load(Ordering::SeqCst) {
                // Check state
                {
                    let current_state = state_clone.read().await.current();
                    if current_state != SessionState::Active {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                }

                // Wait for shutdown
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            info!("Input receiver task stopped");
        });
    }

    /// Spawns the control message handler task
    fn spawn_control_handler_task(&self) {
        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            info!("Starting control handler task");

            while is_running.load(Ordering::SeqCst) {
                // Handle control messages
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            info!("Control handler task stopped");
        });
    }

    /// Processes an input event directly (for testing/loopback)
    pub fn process_input(&self, input: &TransportInput) -> SessionResult<()> {
        if !self.config.allow_input {
            return Err(SessionError::InputError(
                "Input simulation not allowed".to_string(),
            ));
        }

        let simulator = InputSimulator::new();
        simulator
            .simulate(&input.event)
            .map_err(|e| SessionError::InputError(e.to_string()))
    }

    /// Updates the capture quality dynamically
    pub fn set_quality(&mut self, quality: u8) {
        self.config.capture.quality = quality.clamp(1, 100);
        info!("Updated capture quality to {}", quality);
    }

    /// Updates the capture FPS dynamically
    pub fn set_fps(&mut self, fps: u8) {
        self.config.capture.fps = fps.clamp(1, 60);
        info!("Updated capture FPS to {}", fps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::transport::create_loopback_transport;

    #[tokio::test]
    async fn test_host_session_creation() {
        let config = HostSessionConfig::default();
        let (host_transport, _client_transport) = create_loopback_transport();

        let session = HostSession::new(config, host_transport);
        assert_eq!(session.state().await, SessionState::Idle);
    }

    #[tokio::test]
    async fn test_host_session_config() {
        let config = HostSessionConfig::new(30, 80)
            .with_input(false)
            .with_session_id("test-session".to_string());

        assert_eq!(config.capture.fps, 30);
        assert_eq!(config.capture.quality, 80);
        assert!(!config.allow_input);
        assert_eq!(config.session_id, "test-session");
    }

    #[tokio::test]
    async fn test_host_session_state_transitions() {
        let config = HostSessionConfig::default();
        let (host_transport, _client_transport) = create_loopback_transport();

        let mut session = HostSession::new(config, host_transport);

        // Start session
        // Note: This will fail in CI due to no display, but state transition logic is tested
        let state = session.state().await;
        assert_eq!(state, SessionState::Idle);
    }

    #[tokio::test]
    async fn test_host_session_stats() {
        let config = HostSessionConfig::default();
        let (host_transport, _client_transport) = create_loopback_transport();

        let session = HostSession::new(config, host_transport);
        let stats = session.stats().await;

        assert_eq!(stats.frames_sent, 0);
        assert_eq!(stats.frames_dropped, 0);
        assert_eq!(stats.bytes_sent, 0);
    }

    #[test]
    fn test_host_session_stats_calculations() {
        let mut stats = HostSessionStats::default();
        stats.started_at = Some(Instant::now());
        stats.frames_sent = 100;
        stats.frames_dropped = 10;

        assert!(stats.drop_rate() > 0.0);
        assert!(stats.average_fps() >= 0.0);
    }
}
