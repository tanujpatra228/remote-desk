//! Transport abstraction for session communication
//!
//! This module provides channel-based transport that can be used for:
//! - Loopback testing (host and client in same process)
//! - QUIC-based networking over real network connections

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::desktop::FrameFormat;
use crate::input::InputEvent;
use crate::network::{
    BiStream, ConnectionRole, Message, QuicConnection, StreamReceiver, StreamSender,
};

/// Default channel buffer size
pub const DEFAULT_CHANNEL_BUFFER: usize = 32;

/// Frame channel buffer size (smaller to prevent memory bloat)
pub const FRAME_CHANNEL_BUFFER: usize = 4;

/// Frame data ready for transport
///
/// This is a serializable version of Frame/EncodedFrame for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportFrame {
    /// Frame sequence number for ordering
    pub sequence: u64,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Encoding format
    pub format: FrameFormat,
    /// Encoded frame data
    pub data: Vec<u8>,
    /// Original uncompressed size
    pub original_size: usize,
    /// Timestamp when frame was captured (millis since session start)
    pub timestamp_ms: u64,
}

impl TransportFrame {
    /// Creates a new transport frame
    pub fn new(
        sequence: u64,
        width: u32,
        height: u32,
        format: FrameFormat,
        data: Vec<u8>,
        original_size: usize,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            sequence,
            width,
            height,
            format,
            data,
            original_size,
            timestamp_ms,
        }
    }

    /// Returns the size of the encoded data
    pub fn encoded_size(&self) -> usize {
        self.data.len()
    }

    /// Returns the compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        self.data.len() as f64 / self.original_size as f64
    }
}

/// Input event with source coordinates for transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportInput {
    /// The input event
    pub event: InputEvent,
    /// Source window coordinates (for mouse events that need translation)
    pub source_coords: Option<(u32, u32)>,
    /// Sequence number for ordering
    pub sequence: u64,
}

impl TransportInput {
    /// Creates a new transport input
    pub fn new(event: InputEvent, sequence: u64) -> Self {
        Self {
            event,
            source_coords: None,
            sequence,
        }
    }

    /// Creates a transport input with source coordinates
    pub fn with_coords(event: InputEvent, sequence: u64, x: u32, y: u32) -> Self {
        Self {
            event,
            source_coords: Some((x, y)),
            sequence,
        }
    }
}

/// Clipboard content for transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportClipboard {
    /// Clipboard content type
    pub content_type: ClipboardContentType,
    /// Clipboard data
    pub data: Vec<u8>,
    /// Hash of content for deduplication
    pub content_hash: u64,
    /// Sequence number
    pub sequence: u64,
}

/// Type of clipboard content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardContentType {
    /// Plain text content
    Text,
    /// HTML content
    Html,
    /// Image content (PNG)
    Image,
}

/// Control messages for session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    /// Request to start session
    Start,
    /// Request to pause session
    Pause,
    /// Request to resume session
    Resume,
    /// Request to stop session
    Stop,
    /// Ping for latency measurement
    Ping { timestamp_ms: u64 },
    /// Pong response
    Pong { original_timestamp_ms: u64 },
    /// Quality adjustment request
    SetQuality { quality: u8 },
    /// FPS adjustment request
    SetFps { fps: u8 },
    /// Request display info
    RequestDisplayInfo,
    /// Display info response
    DisplayInfo { width: u32, height: u32, name: String },
}

/// Statistics for a transport channel
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Last latency measurement (ms)
    pub latency_ms: Option<u64>,
    /// Session start time
    pub started_at: Option<Instant>,
}

impl TransportStats {
    /// Updates latency based on ping/pong
    pub fn update_latency(&mut self, round_trip_ms: u64) {
        self.latency_ms = Some(round_trip_ms / 2);
    }

    /// Returns session duration in seconds
    pub fn duration_secs(&self) -> f64 {
        self.started_at
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Returns average bandwidth in bytes per second
    pub fn bandwidth_bps(&self) -> f64 {
        let duration = self.duration_secs();
        if duration == 0.0 {
            return 0.0;
        }
        (self.bytes_sent + self.bytes_received) as f64 / duration
    }
}

/// Channel pair for bidirectional communication
pub struct ChannelPair<T> {
    /// Sending half
    pub tx: mpsc::Sender<T>,
    /// Receiving half
    pub rx: mpsc::Receiver<T>,
}

/// Complete transport channels for a session
pub struct SessionTransport {
    /// Channel for sending/receiving frames
    pub frames: ChannelPair<TransportFrame>,
    /// Channel for sending/receiving input events
    pub input: ChannelPair<TransportInput>,
    /// Channel for clipboard synchronization
    pub clipboard: ChannelPair<TransportClipboard>,
    /// Channel for control messages
    pub control: ChannelPair<ControlMessage>,
}

/// Creates a loopback transport pair for testing
///
/// Returns (host_transport, client_transport) where:
/// - host_transport.frames.tx sends to client_transport.frames.rx
/// - client_transport.input.tx sends to host_transport.input.rx
/// - etc.
pub fn create_loopback_transport() -> (SessionTransport, SessionTransport) {
    // Create frame channels (host sends to client)
    let (host_frame_tx, client_frame_rx) = mpsc::channel(FRAME_CHANNEL_BUFFER);
    let (client_frame_tx, host_frame_rx) = mpsc::channel(FRAME_CHANNEL_BUFFER);

    // Create input channels (client sends to host)
    let (host_input_tx, client_input_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
    let (client_input_tx, host_input_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);

    // Create clipboard channels (bidirectional)
    let (host_clipboard_tx, client_clipboard_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
    let (client_clipboard_tx, host_clipboard_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);

    // Create control channels (bidirectional)
    let (host_control_tx, client_control_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
    let (client_control_tx, host_control_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);

    let host_transport = SessionTransport {
        frames: ChannelPair {
            tx: host_frame_tx,
            rx: host_frame_rx,
        },
        input: ChannelPair {
            tx: host_input_tx,
            rx: host_input_rx,
        },
        clipboard: ChannelPair {
            tx: host_clipboard_tx,
            rx: host_clipboard_rx,
        },
        control: ChannelPair {
            tx: host_control_tx,
            rx: host_control_rx,
        },
    };

    let client_transport = SessionTransport {
        frames: ChannelPair {
            tx: client_frame_tx,
            rx: client_frame_rx,
        },
        input: ChannelPair {
            tx: client_input_tx,
            rx: client_input_rx,
        },
        clipboard: ChannelPair {
            tx: client_clipboard_tx,
            rx: client_clipboard_rx,
        },
        control: ChannelPair {
            tx: client_control_tx,
            rx: client_control_rx,
        },
    };

    (host_transport, client_transport)
}

/// Result type for transport operations
pub type TransportResult<T> = Result<T, TransportError>;

/// Error type for transport operations
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("QUIC stream error: {0}")]
    StreamError(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Connection error: {0}")]
    ConnectionError(String),
}

/// Handle to the background tasks that bridge QUIC streams to channels
pub struct QuicTransportHandle {
    /// Task handles for the bridge tasks
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl QuicTransportHandle {
    /// Aborts all bridge tasks
    pub fn abort(&self) {
        for handle in &self.handles {
            handle.abort();
        }
    }

    /// Waits for all bridge tasks to complete
    pub async fn join(self) {
        for handle in self.handles {
            let _ = handle.await;
        }
    }
}

/// Creates a SessionTransport from a QUIC connection
///
/// This function opens the necessary QUIC streams and creates bridge tasks
/// that forward data between the streams and mpsc channels.
///
/// # Stream Layout
/// - Stream 1: Video frames (host → client, unidirectional)
/// - Stream 2: Input events (client → host, unidirectional)
/// - Stream 3: Clipboard (bidirectional)
/// - Control messages use the existing control stream from connection handshake
///
/// # Arguments
///
/// * `connection` - The established QUIC connection
/// * `role` - Whether this is the host or client side
/// * `control_stream` - The control stream from the connection handshake
///
/// # Returns
///
/// A tuple of (SessionTransport, QuicTransportHandle)
pub async fn create_quic_transport(
    connection: QuicConnection,
    role: ConnectionRole,
    control_stream: BiStream<Message>,
) -> TransportResult<(SessionTransport, QuicTransportHandle)> {
    info!("Creating QUIC transport for {:?} role", role);

    let mut handles = Vec::new();

    // Create channel pairs for the transport
    let (frame_out_tx, frame_out_rx) = mpsc::channel(FRAME_CHANNEL_BUFFER);
    let (frame_in_tx, frame_in_rx) = mpsc::channel(FRAME_CHANNEL_BUFFER);

    let (input_out_tx, input_out_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
    let (input_in_tx, input_in_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);

    let (clipboard_out_tx, clipboard_out_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
    let (clipboard_in_tx, clipboard_in_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);

    let (control_out_tx, control_out_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
    let (control_in_tx, control_in_rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);

    match role {
        ConnectionRole::Host => {
            // Host opens video stream (unidirectional send)
            let video_send = connection
                .open_uni()
                .await
                .map_err(|e| TransportError::StreamError(e.to_string()))?;

            // Host accepts input stream (unidirectional receive)
            let input_recv = connection
                .accept_uni()
                .await
                .map_err(|e| TransportError::StreamError(e.to_string()))?;

            // Host opens clipboard stream (bidirectional)
            let (clipboard_send, clipboard_recv) = connection
                .open_bi()
                .await
                .map_err(|e| TransportError::StreamError(e.to_string()))?;

            // Bridge video frames: channel → QUIC stream
            let sender: StreamSender<TransportFrame> = StreamSender::new(video_send);
            handles.push(spawn_channel_to_stream(frame_out_rx, sender));

            // Bridge input: QUIC stream → channel
            let receiver: StreamReceiver<TransportInput> = StreamReceiver::new(input_recv);
            handles.push(spawn_stream_to_channel(receiver, input_in_tx));

            // Bridge clipboard both directions
            let clip_sender: StreamSender<TransportClipboard> = StreamSender::new(clipboard_send);
            let clip_receiver: StreamReceiver<TransportClipboard> =
                StreamReceiver::new(clipboard_recv);
            handles.push(spawn_channel_to_stream(clipboard_out_rx, clip_sender));
            handles.push(spawn_stream_to_channel(clip_receiver, clipboard_in_tx));
        }
        ConnectionRole::Client => {
            // Client accepts video stream (unidirectional receive)
            let video_recv = connection
                .accept_uni()
                .await
                .map_err(|e| TransportError::StreamError(e.to_string()))?;

            // Client opens input stream (unidirectional send)
            let input_send = connection
                .open_uni()
                .await
                .map_err(|e| TransportError::StreamError(e.to_string()))?;

            // Client accepts clipboard stream (bidirectional)
            let (clipboard_send, clipboard_recv) = connection
                .accept_bi()
                .await
                .map_err(|e| TransportError::StreamError(e.to_string()))?;

            // Bridge video frames: QUIC stream → channel
            let receiver: StreamReceiver<TransportFrame> = StreamReceiver::new(video_recv);
            handles.push(spawn_stream_to_channel(receiver, frame_in_tx));

            // Bridge input: channel → QUIC stream
            let sender: StreamSender<TransportInput> = StreamSender::new(input_send);
            handles.push(spawn_channel_to_stream(input_out_rx, sender));

            // Bridge clipboard both directions
            let clip_sender: StreamSender<TransportClipboard> = StreamSender::new(clipboard_send);
            let clip_receiver: StreamReceiver<TransportClipboard> =
                StreamReceiver::new(clipboard_recv);
            handles.push(spawn_channel_to_stream(clipboard_out_rx, clip_sender));
            handles.push(spawn_stream_to_channel(clip_receiver, clipboard_in_tx));
        }
    }

    // Bridge control messages (using the existing control stream)
    // Note: Control messages use the protocol Message type, but we wrap them
    // in ControlMessage for the session layer
    let BiStream { sender, receiver } = control_stream;
    handles.push(spawn_control_sender(control_out_rx, sender));
    handles.push(spawn_control_receiver(receiver, control_in_tx));

    let transport = SessionTransport {
        frames: ChannelPair {
            tx: frame_out_tx,
            rx: frame_in_rx,
        },
        input: ChannelPair {
            tx: input_out_tx,
            rx: input_in_rx,
        },
        clipboard: ChannelPair {
            tx: clipboard_out_tx,
            rx: clipboard_in_rx,
        },
        control: ChannelPair {
            tx: control_out_tx,
            rx: control_in_rx,
        },
    };

    let handle = QuicTransportHandle { handles };

    info!("QUIC transport created successfully");
    Ok((transport, handle))
}

/// Spawns a task that reads from an mpsc channel and writes to a QUIC stream
fn spawn_channel_to_stream<T>(
    mut rx: mpsc::Receiver<T>,
    mut sender: StreamSender<T>,
) -> tokio::task::JoinHandle<()>
where
    T: Serialize + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = sender.send(msg).await {
                error!("Failed to send to QUIC stream: {}", e);
                break;
            }
        }
        debug!("Channel-to-stream bridge closed");
    })
}

/// Spawns a task that reads from a QUIC stream and writes to an mpsc channel
fn spawn_stream_to_channel<T>(
    mut receiver: StreamReceiver<T>,
    tx: mpsc::Sender<T>,
) -> tokio::task::JoinHandle<()>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(msg) => {
                    if tx.send(msg).await.is_err() {
                        debug!("Channel closed, stopping stream bridge");
                        break;
                    }
                }
                Err(e) => {
                    debug!("Stream closed or error: {}", e);
                    break;
                }
            }
        }
        debug!("Stream-to-channel bridge closed");
    })
}

/// Spawns a task that bridges control messages from channel to QUIC stream
fn spawn_control_sender(
    mut rx: mpsc::Receiver<ControlMessage>,
    mut sender: StreamSender<Message>,
) -> tokio::task::JoinHandle<()> {
    use crate::network::{MessagePayload, MessageType};

    tokio::spawn(async move {
        while let Some(ctrl) = rx.recv().await {
            // Wrap ControlMessage in protocol Message
            // For now, we'll use Heartbeat for ping/pong and custom handling
            let msg = match ctrl {
                ControlMessage::Ping { timestamp_ms } => Message::new(
                    MessageType::Heartbeat,
                    MessagePayload::Heartbeat(crate::network::Heartbeat { timestamp: timestamp_ms }),
                ),
                ControlMessage::Pong { original_timestamp_ms } => Message::new(
                    MessageType::Heartbeat,
                    MessagePayload::Heartbeat(crate::network::Heartbeat {
                        timestamp: original_timestamp_ms,
                    }),
                ),
                _ => {
                    // For other control messages, we'd need to extend the protocol
                    // For now, skip them
                    continue;
                }
            };

            if let Err(e) = sender.send(msg).await {
                error!("Failed to send control message: {}", e);
                break;
            }
        }
        debug!("Control sender bridge closed");
    })
}

/// Spawns a task that bridges control messages from QUIC stream to channel
fn spawn_control_receiver(
    mut receiver: StreamReceiver<Message>,
    tx: mpsc::Sender<ControlMessage>,
) -> tokio::task::JoinHandle<()> {
    use crate::network::MessagePayload;

    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(msg) => {
                    // Convert protocol Message to ControlMessage
                    let ctrl = match msg.payload {
                        MessagePayload::Heartbeat(hb) => {
                            // Could be ping or pong - use as ping for now
                            ControlMessage::Ping {
                                timestamp_ms: hb.timestamp,
                            }
                        }
                        _ => continue, // Skip other messages
                    };

                    if tx.send(ctrl).await.is_err() {
                        debug!("Control channel closed");
                        break;
                    }
                }
                Err(e) => {
                    debug!("Control stream closed or error: {}", e);
                    break;
                }
            }
        }
        debug!("Control receiver bridge closed");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{KeyboardEvent, Key};

    #[tokio::test]
    async fn test_loopback_frame_channel() {
        let (host, client) = create_loopback_transport();

        let frame = TransportFrame::new(
            1,
            1920,
            1080,
            FrameFormat::Jpeg,
            vec![0u8; 1000],
            8294400,
            0,
        );

        // Host sends frame
        host.frames.tx.send(frame.clone()).await.unwrap();

        // Client receives frame
        let mut client_frames_rx = client.frames.rx;
        let received = client_frames_rx.recv().await.unwrap();
        assert_eq!(received.sequence, 1);
        assert_eq!(received.width, 1920);
        assert_eq!(received.height, 1080);
    }

    #[tokio::test]
    async fn test_loopback_input_channel() {
        let (host, client) = create_loopback_transport();

        let input = TransportInput::new(
            InputEvent::Keyboard(KeyboardEvent::key_press(Key::A)),
            1,
        );

        // Client sends input
        client.input.tx.send(input).await.unwrap();

        // Host receives input
        let mut host_input_rx = host.input.rx;
        let received = host_input_rx.recv().await.unwrap();
        assert_eq!(received.sequence, 1);
    }

    #[tokio::test]
    async fn test_loopback_control_channel() {
        let (host, client) = create_loopback_transport();

        // Client sends control message
        client.control.tx.send(ControlMessage::Start).await.unwrap();

        // Host receives control message
        let mut host_control_rx = host.control.rx;
        let received = host_control_rx.recv().await.unwrap();
        assert!(matches!(received, ControlMessage::Start));
    }

    #[test]
    fn test_transport_frame_compression_ratio() {
        let frame = TransportFrame::new(
            1,
            1920,
            1080,
            FrameFormat::Jpeg,
            vec![0u8; 100_000],
            1_000_000,
            0,
        );

        assert!((frame.compression_ratio() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_transport_stats() {
        let mut stats = TransportStats::default();
        stats.started_at = Some(Instant::now());
        stats.bytes_sent = 1000;
        stats.bytes_received = 500;
        stats.update_latency(20);

        assert_eq!(stats.latency_ms, Some(10));
    }
}
