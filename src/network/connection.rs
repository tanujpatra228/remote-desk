//! Connection management for RemoteDesk
//!
//! This module handles individual P2P connections including state management,
//! lifecycle, and communication.

use crate::security::DeviceId;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::debug;

// Connection constants (avoiding magic numbers)
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const HEARTBEAT_TIMEOUT_SECS: u64 = 15;
const MAX_RECONNECT_ATTEMPTS: u32 = 3;

/// Heartbeat interval duration
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);

/// Heartbeat timeout duration
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(HEARTBEAT_TIMEOUT_SECS);

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Initial state, not connected
    Disconnected,

    /// Attempting to connect
    Connecting,

    /// Connected and authenticated
    Connected,

    /// Connection is being closed
    Disconnecting,

    /// Connection failed
    Failed,
}

/// Connection role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionRole {
    /// This device is the client (initiating connection)
    Client,

    /// This device is the host (receiving connection)
    Host,
}

/// Information about a connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Remote device ID
    pub remote_id: DeviceId,

    /// Remote device name
    pub remote_name: String,

    /// Remote address
    pub remote_addr: SocketAddr,

    /// Session ID
    pub session_id: Option<[u8; 16]>,

    /// Connection role
    pub role: ConnectionRole,

    /// Connection state
    pub state: ConnectionState,

    /// Time when connection was established
    pub connected_at: Option<Instant>,

    /// Last activity time
    pub last_activity: Instant,
}

/// Active connection
pub struct Connection {
    /// Connection information
    info: Arc<Mutex<ConnectionInfo>>,

    /// Last heartbeat received
    last_heartbeat: Arc<Mutex<Instant>>,
}

impl Connection {
    /// Creates a new connection
    pub fn new(
        remote_id: DeviceId,
        remote_name: String,
        remote_addr: SocketAddr,
        role: ConnectionRole,
    ) -> Self {
        let info = ConnectionInfo {
            remote_id,
            remote_name,
            remote_addr,
            session_id: None,
            role,
            state: ConnectionState::Disconnected,
            connected_at: None,
            last_activity: Instant::now(),
        };

        Self {
            info: Arc::new(Mutex::new(info)),
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Gets connection information
    pub async fn info(&self) -> ConnectionInfo {
        self.info.lock().await.clone()
    }

    /// Updates connection state
    pub async fn set_state(&self, state: ConnectionState) {
        let mut info = self.info.lock().await;
        info.state = state;

        if state == ConnectionState::Connected && info.connected_at.is_none() {
            info.connected_at = Some(Instant::now());
        }

        debug!(
            "Connection state changed: {:?} -> {:?}",
            info.remote_id, state
        );
    }

    /// Sets the session ID
    pub async fn set_session_id(&self, session_id: [u8; 16]) {
        let mut info = self.info.lock().await;
        info.session_id = Some(session_id);
    }

    /// Updates last activity timestamp
    pub async fn update_activity(&self) {
        let mut info = self.info.lock().await;
        info.last_activity = Instant::now();
    }

    /// Updates last heartbeat timestamp
    pub async fn update_heartbeat(&self) {
        let mut last_hb = self.last_heartbeat.lock().await;
        *last_hb = Instant::now();
    }

    /// Checks if heartbeat has timed out
    pub async fn is_heartbeat_timeout(&self) -> bool {
        let last_hb = self.last_heartbeat.lock().await;
        last_hb.elapsed() > HEARTBEAT_TIMEOUT
    }

    /// Gets connection duration
    pub async fn connection_duration(&self) -> Option<Duration> {
        let info = self.info.lock().await;
        info.connected_at.map(|t| t.elapsed())
    }

    /// Checks if connection is active
    pub async fn is_connected(&self) -> bool {
        let info = self.info.lock().await;
        info.state == ConnectionState::Connected
    }

    /// Gets the remote device ID
    pub async fn remote_id(&self) -> DeviceId {
        let info = self.info.lock().await;
        info.remote_id
    }
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total bytes sent
    pub bytes_sent: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// Messages sent
    pub messages_sent: u64,

    /// Messages received
    pub messages_received: u64,

    /// Failed send attempts
    pub send_failures: u64,

    /// Failed receive attempts
    pub receive_failures: u64,
}

impl ConnectionStats {
    /// Creates new statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Records bytes sent
    pub fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.messages_sent += 1;
    }

    /// Records bytes received
    pub fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.messages_received += 1;
    }

    /// Records send failure
    pub fn record_send_failure(&mut self) {
        self.send_failures += 1;
    }

    /// Records receive failure
    pub fn record_receive_failure(&mut self) {
        self.receive_failures += 1;
    }
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Disconnecting => write!(f, "Disconnecting"),
            ConnectionState::Failed => write!(f, "Failed"),
        }
    }
}

impl std::fmt::Display for ConnectionRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionRole::Client => write!(f, "Client"),
            ConnectionRole::Host => write!(f, "Host"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_creation() {
        let device_id = DeviceId::from_u32(123456789).unwrap();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let conn = Connection::new(
            device_id,
            "Test Device".to_string(),
            addr,
            ConnectionRole::Client,
        );

        let info = conn.info().await;
        assert_eq!(info.remote_id, device_id);
        assert_eq!(info.state, ConnectionState::Disconnected);
        assert_eq!(info.role, ConnectionRole::Client);
    }

    #[tokio::test]
    async fn test_connection_state_changes() {
        let device_id = DeviceId::from_u32(123456789).unwrap();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let conn = Connection::new(
            device_id,
            "Test Device".to_string(),
            addr,
            ConnectionRole::Client,
        );

        conn.set_state(ConnectionState::Connecting).await;
        assert_eq!(conn.info().await.state, ConnectionState::Connecting);

        conn.set_state(ConnectionState::Connected).await;
        assert!(conn.is_connected().await);
        assert!(conn.connection_duration().await.is_some());
    }

    #[tokio::test]
    async fn test_heartbeat_timeout() {
        let device_id = DeviceId::from_u32(123456789).unwrap();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let conn = Connection::new(
            device_id,
            "Test Device".to_string(),
            addr,
            ConnectionRole::Client,
        );

        // Should not timeout initially
        assert!(!conn.is_heartbeat_timeout().await);

        // Update heartbeat
        conn.update_heartbeat().await;
        assert!(!conn.is_heartbeat_timeout().await);
    }

    #[test]
    fn test_connection_stats() {
        let mut stats = ConnectionStats::new();

        stats.record_sent(1024);
        stats.record_received(2048);
        stats.record_send_failure();

        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 2048);
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.send_failures, 1);
    }
}
