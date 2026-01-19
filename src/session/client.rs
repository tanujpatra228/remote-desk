//! Client session for viewing remote desktop
//!
//! The client session receives frames from the host and sends input events.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::desktop::FrameDecoder;
use crate::error::{SessionError, SessionResult};
use crate::session::state::{SessionState, SessionStateMachine};
use crate::session::transport::{
    ControlMessage, SessionTransport, TransportFrame, TransportInput,
};

/// Configuration for client session
#[derive(Debug, Clone)]
pub struct ClientSessionConfig {
    /// Session identifier
    pub session_id: String,
    /// Whether to send input events
    pub send_input: bool,
    /// Buffer size for frames
    pub frame_buffer_size: usize,
}

impl Default for ClientSessionConfig {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            send_input: true,
            frame_buffer_size: 4,
        }
    }
}

impl ClientSessionConfig {
    /// Creates a new client session config
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the session ID
    pub fn with_session_id(mut self, id: String) -> Self {
        self.session_id = id;
        self
    }

    /// Sets whether to send input
    pub fn with_input(mut self, send: bool) -> Self {
        self.send_input = send;
        self
    }
}

/// Statistics for the client session
#[derive(Debug, Clone, Default)]
pub struct ClientSessionStats {
    /// Total frames received
    pub frames_received: u64,
    /// Total frames decoded
    pub frames_decoded: u64,
    /// Total frames dropped
    pub frames_dropped: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total input events sent
    pub input_events_sent: u64,
    /// Average decode time (ms)
    pub avg_decode_time_ms: f64,
    /// Current FPS
    pub current_fps: f64,
    /// Session start time
    pub started_at: Option<Instant>,
    /// Last latency measurement (ms)
    pub latency_ms: Option<u64>,
}

impl ClientSessionStats {
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
        self.frames_decoded as f64 / duration
    }
}

/// Client session for receiving remote desktop
pub struct ClientSession {
    /// Session configuration
    config: ClientSessionConfig,
    /// Session state machine
    state: Arc<RwLock<SessionStateMachine>>,
    /// Transport channels
    transport: SessionTransport,
    /// Whether the session is running
    is_running: Arc<AtomicBool>,
    /// Session statistics
    stats: Arc<RwLock<ClientSessionStats>>,
    /// Frame decoder
    decoder: Arc<FrameDecoder>,
    /// Input sequence counter
    input_sequence: Arc<AtomicU64>,
}

impl ClientSession {
    /// Creates a new client session
    pub fn new(config: ClientSessionConfig, transport: SessionTransport) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(SessionStateMachine::new())),
            transport,
            is_running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(ClientSessionStats::default())),
            decoder: Arc::new(FrameDecoder::new()),
            input_sequence: Arc::new(AtomicU64::new(0)),
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
    pub async fn stats(&self) -> ClientSessionStats {
        self.stats.read().await.clone()
    }

    /// Returns the frame decoder
    pub fn decoder(&self) -> &FrameDecoder {
        &self.decoder
    }

    /// Starts the client session
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
            let mut stats = self.stats.write().await;
            stats.started_at = Some(Instant::now());
        }

        info!("Client session {} started", self.config.session_id);

        // Start background tasks
        self.spawn_frame_receiver_task();
        self.spawn_control_handler_task();

        Ok(())
    }

    /// Stops the client session
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

        info!("Client session {} stopped", self.config.session_id);

        Ok(())
    }

    /// Pauses the client session
    pub async fn pause(&mut self) -> SessionResult<()> {
        let mut state = self.state.write().await;
        state.transition(SessionState::Paused)?;
        info!("Client session {} paused", self.config.session_id);
        Ok(())
    }

    /// Resumes a paused client session
    pub async fn resume(&mut self) -> SessionResult<()> {
        let mut state = self.state.write().await;
        state.transition(SessionState::Active)?;
        info!("Client session {} resumed", self.config.session_id);
        Ok(())
    }

    /// Spawns the frame receiver task
    fn spawn_frame_receiver_task(&self) {
        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let decoder = Arc::clone(&self.decoder);

        tokio::spawn(async move {
            info!("Starting frame receiver task");

            while is_running.load(Ordering::SeqCst) {
                // Check if we should process frames (only in Active state)
                {
                    let current_state = state.read().await.current();
                    if current_state != SessionState::Active {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                }

                // For now, just sleep - actual frame processing happens via the transport
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }

            info!("Frame receiver task stopped");
        });
    }

    /// Spawns the control message handler task
    fn spawn_control_handler_task(&self) {
        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            info!("Starting control handler task");

            while is_running.load(Ordering::SeqCst) {
                // Handle control messages
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            info!("Control handler task stopped");
        });
    }

    /// Sends an input event to the host
    pub fn send_input(&self, event: crate::input::InputEvent) -> SessionResult<()> {
        if !self.config.send_input {
            return Err(SessionError::InputError(
                "Input sending not enabled".to_string(),
            ));
        }

        let sequence = self.input_sequence.fetch_add(1, Ordering::SeqCst);
        let transport_input = TransportInput::new(event, sequence);

        match self.transport.input.tx.try_send(transport_input) {
            Ok(()) => {
                debug!("Sent input event {}", sequence);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!("Input channel full, dropping event");
                Err(SessionError::ChannelClosed)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!("Input channel closed");
                Err(SessionError::ChannelClosed)
            }
        }
    }

    /// Returns a receiver for frames (for use with viewer)
    pub fn take_frame_receiver(&mut self) -> Option<mpsc::Receiver<TransportFrame>> {
        // This is a design pattern to allow the viewer to own the receiver
        // In practice, you'd use Arc<Mutex<Option<Receiver>>> or restructure
        None // Placeholder - transport ownership is complex
    }

    /// Returns a sender for input events (for use with viewer)
    pub fn input_sender(&self) -> mpsc::Sender<TransportInput> {
        self.transport.input.tx.clone()
    }

    /// Measures latency by sending a ping
    pub async fn measure_latency(&self) -> SessionResult<()> {
        let timestamp_ms = Instant::now().elapsed().as_millis() as u64;
        let ping = ControlMessage::Ping { timestamp_ms };

        match self.transport.control.tx.try_send(ping) {
            Ok(()) => Ok(()),
            Err(_) => Err(SessionError::ChannelClosed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::transport::create_loopback_transport;

    #[tokio::test]
    async fn test_client_session_creation() {
        let config = ClientSessionConfig::default();
        let (_host_transport, client_transport) = create_loopback_transport();

        let session = ClientSession::new(config, client_transport);
        assert_eq!(session.state().await, SessionState::Idle);
    }

    #[tokio::test]
    async fn test_client_session_config() {
        let config = ClientSessionConfig::new()
            .with_input(false)
            .with_session_id("test-client".to_string());

        assert!(!config.send_input);
        assert_eq!(config.session_id, "test-client");
    }

    #[tokio::test]
    async fn test_client_session_stats() {
        let config = ClientSessionConfig::default();
        let (_host_transport, client_transport) = create_loopback_transport();

        let session = ClientSession::new(config, client_transport);
        let stats = session.stats().await;

        assert_eq!(stats.frames_received, 0);
        assert_eq!(stats.frames_decoded, 0);
        assert_eq!(stats.input_events_sent, 0);
    }
}
