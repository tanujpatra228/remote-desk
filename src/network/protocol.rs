//! Network protocol definitions for RemoteDesk
//!
//! This module defines all message types used in the P2P protocol.
//! Based on the specification in docs/PROTOCOL.md

use crate::security::DeviceId;
use serde::{Deserialize, Serialize};

// Protocol constants (avoiding magic numbers)
const PROTOCOL_VERSION: u8 = 1;
const MAX_MESSAGE_SIZE: usize = 10_485_760; // 10 MB

/// Protocol version
pub const CURRENT_PROTOCOL_VERSION: u8 = PROTOCOL_VERSION;

/// Maximum message size in bytes
pub const MAX_MESSAGE_SIZE_BYTES: usize = MAX_MESSAGE_SIZE;

/// Message type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    // Connection Management (0x00 - 0x0F)
    ConnectionRequest = 0x00,
    ConnectionAccept = 0x01,
    ConnectionReject = 0x02,
    Disconnect = 0x03,
    Heartbeat = 0x04,

    // Authentication (0x10 - 0x1F)
    AuthChallenge = 0x10,
    AuthResponse = 0x11,

    // Desktop Control (0x20 - 0x3F)
    ScreenFrame = 0x20,
    ScreenFrameDiff = 0x21,
    KeyboardEvent = 0x22,
    MouseEvent = 0x23,
    ResolutionChange = 0x24,

    // Clipboard (0x40 - 0x4F)
    ClipboardUpdate = 0x40,

    // Metadata (0x50 - 0x5F)
    QualityUpdate = 0x50,
    Statistics = 0x51,

    // Error (0xF0 - 0xFF)
    Error = 0xF0,
}

/// Base message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID for request/response matching
    pub message_id: u32,

    /// Type of message
    pub message_type: MessageType,

    /// Message payload
    pub payload: MessagePayload,
}

/// Message payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    ConnectionRequest(ConnectionRequest),
    ConnectionAccept(ConnectionAccept),
    ConnectionReject(ConnectionReject),
    Disconnect(Disconnect),
    Heartbeat(Heartbeat),
    Error(ErrorMessage),
    ScreenFrame(ScreenFrameData),
    KeyboardEvent(KeyboardEventData),
    MouseEvent(MouseEventData),
}

/// Connection request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    /// Protocol version
    pub protocol_version: u8,

    /// Client's device ID
    pub client_id: u32,

    /// Client device name
    pub client_name: String,

    /// Host's device ID (to verify)
    pub host_id: u32,

    /// Password hash (if host requires password)
    pub password_hash: Option<[u8; 32]>,

    /// Requested capabilities
    pub requested_capabilities: Vec<Capability>,
}

/// Connection accept message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionAccept {
    /// Session ID
    pub session_id: [u8; 16],

    /// Host name
    pub host_name: String,

    /// Host capabilities
    pub host_capabilities: Vec<Capability>,

    /// Desktop information
    pub desktop_info: DesktopInfo,
}

/// Connection reject message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionReject {
    /// Reason for rejection
    pub reason: RejectReason,

    /// Optional message
    pub message: Option<String>,
}

/// Disconnect message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disconnect {
    /// Reason for disconnect
    pub reason: DisconnectReason,
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Error code
    pub error_code: ErrorCode,

    /// Error message
    pub message: String,
}

/// Screen frame data message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenFrameData {
    /// Frame sequence number
    pub sequence: u64,

    /// Frame width in pixels
    pub width: u32,

    /// Frame height in pixels
    pub height: u32,

    /// Frame format
    pub format: FrameFormat,

    /// Compressed frame data
    pub data: Vec<u8>,

    /// Timestamp when frame was captured (milliseconds since epoch)
    pub timestamp: u64,
}

/// Frame encoding format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameFormat {
    /// JPEG compression
    Jpeg = 1,
    /// PNG compression (lossless)
    Png = 2,
    /// Raw RGBA (no compression)
    Raw = 3,
}

/// Keyboard event data message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEventData {
    /// Event type (press or release)
    pub event_type: KeyboardEventTypeData,

    /// Key code
    pub key: u16,

    /// Timestamp when event occurred (milliseconds since epoch)
    pub timestamp: u64,
}

/// Keyboard event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum KeyboardEventTypeData {
    /// Key pressed
    KeyPress = 1,
    /// Key released
    KeyRelease = 2,
}

/// Mouse event data message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEventData {
    /// Event type
    pub event_type: MouseEventTypeData,

    /// Timestamp when event occurred (milliseconds since epoch)
    pub timestamp: u64,
}

/// Mouse event type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseEventTypeData {
    /// Mouse moved to absolute position
    Move {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
    },
    /// Mouse button pressed
    ButtonPress {
        /// Button identifier
        button: u8,
    },
    /// Mouse button released
    ButtonRelease {
        /// Button identifier
        button: u8,
    },
    /// Mouse wheel scrolled
    Wheel {
        /// Horizontal scroll delta
        delta_x: i32,
        /// Vertical scroll delta
        delta_y: i32,
    },
}

/// Reason for connection rejection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    /// User manually rejected
    UserDenied,

    /// Wrong password
    InvalidPassword,

    /// ID doesn't match
    InvalidId,

    /// Already connected
    AlreadyConnected,

    /// Account locked (too many failed attempts)
    AccountLocked,

    /// Protocol version mismatch
    UnsupportedVersion,
}

/// Reason for disconnection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// User initiated disconnect
    UserInitiated,

    /// Session timeout
    SessionTimeout,

    /// Error occurred
    Error,
}

/// Error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    UnknownError = 0,
    ProtocolViolation = 1,
    UnsupportedMessage = 2,
    InvalidPayload = 3,
    PermissionDenied = 4,
    ResourceExhausted = 5,
}

/// Device capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// Remote desktop control
    RemoteControl,

    /// Clipboard synchronization
    ClipboardSync,

    /// File transfer (future)
    FileTransfer,
}

/// Desktop information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopInfo {
    /// Screen width in pixels
    pub screen_width: u16,

    /// Screen height in pixels
    pub screen_height: u16,

    /// Number of screens
    pub screen_count: u8,
}

impl Message {
    /// Creates a new message
    pub fn new(message_type: MessageType, payload: MessagePayload) -> Self {
        use rand::Rng;
        let message_id = rand::thread_rng().gen();

        Self {
            message_id,
            message_type,
            payload,
        }
    }

    /// Serializes the message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserializes a message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Validates message size
    pub fn validate_size(size: usize) -> bool {
        size <= MAX_MESSAGE_SIZE_BYTES
    }
}

impl ConnectionRequest {
    /// Creates a new connection request
    pub fn new(
        client_id: DeviceId,
        client_name: String,
        host_id: DeviceId,
        password_hash: Option<[u8; 32]>,
    ) -> Self {
        Self {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            client_id: client_id.as_u32(),
            client_name,
            host_id: host_id.as_u32(),
            password_hash,
            requested_capabilities: vec![Capability::RemoteControl, Capability::ClipboardSync],
        }
    }
}

impl ConnectionAccept {
    /// Creates a new connection accept message
    pub fn new(
        host_name: String,
        desktop_info: DesktopInfo,
    ) -> Self {
        use rand::Rng;
        let mut session_id = [0u8; 16];
        rand::thread_rng().fill(&mut session_id);

        Self {
            session_id,
            host_name,
            host_capabilities: vec![Capability::RemoteControl, Capability::ClipboardSync],
            desktop_info,
        }
    }
}

impl ConnectionReject {
    /// Creates a new connection reject message
    pub fn new(reason: RejectReason, message: Option<String>) -> Self {
        Self { reason, message }
    }
}

impl Disconnect {
    /// Creates a new disconnect message
    pub fn new(reason: DisconnectReason) -> Self {
        Self { reason }
    }
}

impl Heartbeat {
    /// Creates a new heartbeat message with current timestamp
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self { timestamp }
    }
}

impl Default for Heartbeat {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorMessage {
    /// Creates a new error message
    pub fn new(error_code: ErrorCode, message: String) -> Self {
        Self { error_code, message }
    }
}

impl DesktopInfo {
    /// Creates desktop info from current system (placeholder)
    pub fn current() -> Self {
        // TODO: Get actual screen dimensions
        Self {
            screen_width: 1920,
            screen_height: 1080,
            screen_count: 1,
        }
    }
}

impl ScreenFrameData {
    /// Creates a new screen frame data message
    pub fn new(
        sequence: u64,
        width: u32,
        height: u32,
        format: FrameFormat,
        data: Vec<u8>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            sequence,
            width,
            height,
            format,
            data,
            timestamp,
        }
    }
}

impl KeyboardEventData {
    /// Creates a new keyboard event message
    pub fn new(event_type: KeyboardEventTypeData, key: u16) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            event_type,
            key,
            timestamp,
        }
    }
}

impl MouseEventData {
    /// Creates a new mouse event message
    pub fn new(event_type: MouseEventTypeData) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            event_type,
            timestamp,
        }
    }

    /// Creates a mouse move event
    pub fn move_to(x: i32, y: i32) -> Self {
        Self::new(MouseEventTypeData::Move { x, y })
    }

    /// Creates a mouse button press event
    pub fn button_press(button: u8) -> Self {
        Self::new(MouseEventTypeData::ButtonPress { button })
    }

    /// Creates a mouse button release event
    pub fn button_release(button: u8) -> Self {
        Self::new(MouseEventTypeData::ButtonRelease { button })
    }

    /// Creates a mouse wheel event
    pub fn wheel(delta_x: i32, delta_y: i32) -> Self {
        Self::new(MouseEventTypeData::Wheel { delta_x, delta_y })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let request = ConnectionRequest::new(
            DeviceId::from_u32(123456789).unwrap(),
            "Test Client".to_string(),
            DeviceId::from_u32(987654321).unwrap(),
            None,
        );

        let message = Message::new(
            MessageType::ConnectionRequest,
            MessagePayload::ConnectionRequest(request.clone()),
        );

        let bytes = message.to_bytes().unwrap();
        let deserialized = Message::from_bytes(&bytes).unwrap();

        assert_eq!(message.message_type, deserialized.message_type);
    }

    #[test]
    fn test_heartbeat_timestamp() {
        let heartbeat1 = Heartbeat::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let heartbeat2 = Heartbeat::new();

        assert!(heartbeat2.timestamp > heartbeat1.timestamp);
    }

    #[test]
    fn test_message_size_validation() {
        assert!(Message::validate_size(1024));
        assert!(Message::validate_size(MAX_MESSAGE_SIZE_BYTES));
        assert!(!Message::validate_size(MAX_MESSAGE_SIZE_BYTES + 1));
    }

    #[test]
    fn test_connection_request_creation() {
        let client_id = DeviceId::from_u32(123456789).unwrap();
        let host_id = DeviceId::from_u32(987654321).unwrap();

        let request = ConnectionRequest::new(
            client_id,
            "Test".to_string(),
            host_id,
            None,
        );

        assert_eq!(request.protocol_version, CURRENT_PROTOCOL_VERSION);
        assert_eq!(request.client_id, 123456789);
        assert_eq!(request.host_id, 987654321);
    }
}
