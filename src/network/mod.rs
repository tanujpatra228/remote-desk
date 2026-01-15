//! Network module for RemoteDesk
//!
//! This module handles all networking functionality including:
//! - P2P connection management
//! - Peer discovery (mDNS)
//! - Protocol implementation
//! - Connection lifecycle management

pub mod connection;
pub mod discovery;
pub mod manager;
pub mod protocol;

// Re-export commonly used types
pub use connection::{Connection, ConnectionInfo, ConnectionRole, ConnectionState, ConnectionStats};
pub use discovery::{PeerDiscovery, PeerInfo, DEFAULT_SERVICE_PORT};
pub use manager::{ConnectionEvent, ConnectionManager, ManagerConfig};
pub use protocol::{
    Capability, ConnectionAccept, ConnectionReject, ConnectionRequest, DesktopInfo, Disconnect,
    DisconnectReason, ErrorCode, ErrorMessage, FrameFormat, Heartbeat, KeyboardEventData,
    KeyboardEventTypeData, Message, MessagePayload, MessageType, MouseEventData,
    MouseEventTypeData, RejectReason, ScreenFrameData, CURRENT_PROTOCOL_VERSION,
};
