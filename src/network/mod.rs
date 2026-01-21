//! Network module for RemoteDesk
//!
//! This module handles all networking functionality including:
//! - QUIC-based P2P connection management
//! - Peer discovery via mDNS
//! - Protocol implementation
//! - Connection lifecycle management
//! - TLS certificate management

pub mod cert;
pub mod connection;
pub mod discovery;
pub mod listener;
pub mod manager;
pub mod protocol;
pub mod quic;
pub mod stream;

// Re-export commonly used types
pub use connection::{Connection, ConnectionInfo, ConnectionRole, ConnectionState, ConnectionStats};
pub use discovery::{PeerDiscovery, PeerEvent, PeerInfo, DEFAULT_SERVICE_PORT};
pub use manager::{ConnectionEvent, ConnectionManager, EstablishedConnection, ManagerConfig};
pub use protocol::{
    Capability, ConnectionAccept, ConnectionReject, ConnectionRequest, DesktopInfo, Disconnect,
    DisconnectReason, ErrorCode, ErrorMessage, FrameFormat, Heartbeat, KeyboardEventData,
    KeyboardEventTypeData, Message, MessagePayload, MessageType, MouseEventData,
    MouseEventTypeData, RejectReason, ScreenFrameData, CURRENT_PROTOCOL_VERSION,
};
pub use quic::{QuicConfig, QuicConnection, QuicEndpoint, QuicError, StreamType, DEFAULT_QUIC_PORT};
pub use stream::{BiStream, StreamError, StreamReceiver, StreamSender};
pub use cert::{CertError, CertPair};
pub use listener::{AcceptedConnection, ConnectionListener, IncomingConnection, PendingConnection};
