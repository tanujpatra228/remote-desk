//! Remote desktop session module
//!
//! This module handles the integration of screen capture, input simulation,
//! and network communication for remote desktop sessions.

pub mod client;
pub mod host;
pub mod manager;
pub mod state;
pub mod transport;
pub mod types;

pub use client::{ClientSession, ClientSessionConfig, ClientSessionStats};
pub use host::{HostSession, HostSessionConfig, HostSessionStats};
pub use manager::{ManagedSession, SessionId, SessionInfo, SessionManager, SessionType};
pub use state::{SessionState, SessionStateMachine, StateTransition};
pub use transport::{
    create_loopback_transport, ChannelPair, ClipboardContentType, ControlMessage,
    SessionTransport, TransportClipboard, TransportFrame, TransportInput, TransportStats,
};
pub use types::{Session, SessionConfig, SessionMode, SessionStats};
