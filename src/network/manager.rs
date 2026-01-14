//! Connection manager for RemoteDesk
//!
//! This module provides the main interface for managing P2P connections,
//! coordinating peer discovery, and handling the connection lifecycle.

use crate::error::{NetworkError, NetworkResult};
use crate::network::{
    Connection, ConnectionInfo, ConnectionReject, ConnectionRequest, ConnectionRole,
    ConnectionState, DesktopInfo, MessagePayload, MessageType, PeerDiscovery, PeerInfo,
    RejectReason, Message,
};
use crate::security::{DeviceId, PasswordManager};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Connection manager configuration
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Local device ID
    pub device_id: DeviceId,

    /// Local device name
    pub device_name: String,

    /// Service port
    pub service_port: u16,

    /// Path to password hash file
    pub password_hash_path: PathBuf,

    /// Maximum concurrent connections
    pub max_connections: usize,
}

/// Events emitted by the connection manager
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Connection request received
    ConnectionRequest {
        remote_id: DeviceId,
        remote_name: String,
        has_password: bool,
    },

    /// Connection established
    Connected {
        remote_id: DeviceId,
    },

    /// Connection closed
    Disconnected {
        remote_id: DeviceId,
        reason: String,
    },

    /// Peer discovered
    PeerDiscovered {
        peer_info: PeerInfo,
    },

    /// Peer lost
    PeerLost {
        device_id: DeviceId,
    },
}

/// Main connection manager
pub struct ConnectionManager {
    /// Configuration
    config: ManagerConfig,

    /// Active connections
    connections: Arc<RwLock<HashMap<DeviceId, Arc<Connection>>>>,

    /// Peer discovery
    discovery: Arc<PeerDiscovery>,

    /// Event channel sender
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,

    /// Event channel receiver
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<ConnectionEvent>>>,
}

impl ConnectionManager {
    /// Creates a new connection manager
    pub fn new(config: ManagerConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let discovery = Arc::new(PeerDiscovery::new(
            config.device_id,
            config.device_name.clone(),
            config.service_port,
        ));

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            discovery,
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        }
    }

    /// Starts the connection manager
    pub async fn start(&self) -> NetworkResult<()> {
        info!("Starting connection manager");

        // Start peer discovery
        self.discovery
            .start_advertising()
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e))?;

        self.discovery
            .start_discovery()
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e))?;

        info!("Connection manager started successfully");
        Ok(())
    }

    /// Stops the connection manager
    pub async fn stop(&self) {
        info!("Stopping connection manager");

        // Close all connections
        let connections = self.connections.read().await;
        for (_, conn) in connections.iter() {
            conn.set_state(ConnectionState::Disconnecting).await;
        }

        // Stop discovery
        self.discovery.stop_advertising().await;
        self.discovery.stop_discovery().await;

        info!("Connection manager stopped");
    }

    /// Initiates a connection to a remote device
    ///
    /// # Arguments
    ///
    /// * `remote_id` - The device ID to connect to
    /// * `password` - Optional password for authentication
    ///
    /// # Returns
    ///
    /// Returns Ok if connection is initiated successfully
    pub async fn connect(
        &self,
        remote_id: DeviceId,
        password: Option<String>,
    ) -> NetworkResult<()> {
        info!("Initiating connection to {}", remote_id.format_with_spaces());

        // Check if already connected
        if self.is_connected(remote_id).await {
            return Err(NetworkError::ConnectionFailed(
                "Already connected to this device".to_string(),
            ));
        }

        // Check maximum connections
        let connections = self.connections.read().await;
        if connections.len() >= self.config.max_connections {
            return Err(NetworkError::ConnectionFailed(
                "Maximum connections reached".to_string(),
            ));
        }
        drop(connections);

        // Try to resolve the peer
        let addresses = self.discovery.resolve(remote_id).await;

        if addresses.is_none() {
            warn!(
                "Peer {} not found via discovery, will attempt direct connection",
                remote_id.format_with_spaces()
            );
        }

        // For now, create a dummy address since we don't have actual networking yet
        let remote_addr: SocketAddr = "127.0.0.1:7070".parse().unwrap();

        // Create connection
        let connection = Arc::new(Connection::new(
            remote_id,
            format!("Device-{}", remote_id.as_u32()),
            remote_addr,
            ConnectionRole::Client,
        ));

        connection.set_state(ConnectionState::Connecting).await;

        // Create connection request
        let password_hash = if let Some(pwd) = password {
            // For simplicity, use a basic hash (in real implementation, use proper challenge-response)
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(pwd.as_bytes());
            hasher.update(remote_id.as_u32().to_le_bytes());
            let result = hasher.finalize();
            Some(result.into())
        } else {
            None
        };

        let request = ConnectionRequest::new(
            self.config.device_id,
            self.config.device_name.clone(),
            remote_id,
            password_hash,
        );

        // TODO: Actually send the request over the network
        // For now, simulate the connection
        info!(
            "Created connection request to {}",
            remote_id.format_with_spaces()
        );

        // Store the connection
        let mut connections = self.connections.write().await;
        connections.insert(remote_id, connection.clone());
        drop(connections);

        // Simulate successful connection for testing
        // TODO: Remove this and implement actual network communication
        connection.set_state(ConnectionState::Connected).await;
        let session_id = [0u8; 16]; // Dummy session ID
        connection.set_session_id(session_id).await;

        // Emit connected event
        let _ = self.event_tx.send(ConnectionEvent::Connected { remote_id });

        info!("Connection established to {}", remote_id.format_with_spaces());

        Ok(())
    }

    /// Disconnects from a remote device
    pub async fn disconnect(&self, remote_id: DeviceId) -> NetworkResult<()> {
        info!("Disconnecting from {}", remote_id.format_with_spaces());

        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(&remote_id) {
            connection.set_state(ConnectionState::Disconnecting).await;

            // TODO: Send disconnect message

            connection.set_state(ConnectionState::Disconnected).await;

            // Emit disconnected event
            let _ = self.event_tx.send(ConnectionEvent::Disconnected {
                remote_id,
                reason: "User initiated".to_string(),
            });

            info!("Disconnected from {}", remote_id.format_with_spaces());
        }

        Ok(())
    }

    /// Checks if connected to a specific device
    pub async fn is_connected(&self, remote_id: DeviceId) -> bool {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(&remote_id) {
            conn.is_connected().await
        } else {
            false
        }
    }

    /// Gets information about a connection
    pub async fn get_connection_info(&self, remote_id: DeviceId) -> Option<ConnectionInfo> {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(&remote_id) {
            Some(conn.info().await)
        } else {
            None
        }
    }

    /// Gets all active connections
    pub async fn get_active_connections(&self) -> Vec<ConnectionInfo> {
        let connections = self.connections.read().await;
        let mut infos = Vec::new();

        for (_, conn) in connections.iter() {
            if conn.is_connected().await {
                infos.push(conn.info().await);
            }
        }

        infos
    }

    /// Gets the next connection event (non-blocking)
    pub async fn try_recv_event(&self) -> Option<ConnectionEvent> {
        let mut rx = self.event_rx.write().await;
        rx.try_recv().ok()
    }

    /// Handles an incoming connection request
    pub async fn handle_connection_request(
        &self,
        request: ConnectionRequest,
        remote_addr: SocketAddr,
    ) -> NetworkResult<()> {
        let remote_id = DeviceId::from_u32(request.client_id)?;

        info!(
            "Received connection request from {} ({})",
            remote_id.format_with_spaces(),
            request.client_name
        );

        // Check if password is required
        let password_required = PasswordManager::is_password_set(&self.config.password_hash_path);

        if password_required {
            // Verify password
            if let Some(password_hash) = request.password_hash {
                // TODO: Implement proper password verification
                // For now, just accept it
                debug!("Password authentication enabled, verifying...");
            } else {
                // Reject - password required but not provided
                warn!("Connection rejected: password required but not provided");
                // TODO: Send rejection message
                return Ok(());
            }
        } else {
            // Manual accept mode - emit event for user to accept/reject
            let _ = self.event_tx.send(ConnectionEvent::ConnectionRequest {
                remote_id,
                remote_name: request.client_name.clone(),
                has_password: request.password_hash.is_some(),
            });

            // TODO: Wait for user decision
            info!("Waiting for user to accept/reject connection");
        }

        Ok(())
    }

    /// Accepts a pending connection request
    pub async fn accept_connection(&self, remote_id: DeviceId) -> NetworkResult<()> {
        info!("Accepting connection from {}", remote_id.format_with_spaces());

        // Create connection
        let remote_addr: SocketAddr = "127.0.0.1:7070".parse().unwrap(); // Placeholder
        let connection = Arc::new(Connection::new(
            remote_id,
            format!("Device-{}", remote_id.as_u32()),
            remote_addr,
            ConnectionRole::Host,
        ));

        connection.set_state(ConnectionState::Connected).await;

        // Store connection
        let mut connections = self.connections.write().await;
        connections.insert(remote_id, connection);

        // Emit connected event
        let _ = self.event_tx.send(ConnectionEvent::Connected { remote_id });

        info!("Connection accepted from {}", remote_id.format_with_spaces());

        Ok(())
    }

    /// Rejects a pending connection request
    pub async fn reject_connection(
        &self,
        remote_id: DeviceId,
        reason: RejectReason,
    ) -> NetworkResult<()> {
        info!("Rejecting connection from {}", remote_id.format_with_spaces());

        // TODO: Send rejection message

        Ok(())
    }

    /// Gets the local device ID
    pub fn device_id(&self) -> DeviceId {
        self.config.device_id
    }

    /// Gets all discovered peers
    pub async fn get_discovered_peers(&self) -> Vec<PeerInfo> {
        self.discovery.get_all_peers().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> ManagerConfig {
        let temp_dir = TempDir::new().unwrap();
        let device_id = DeviceId::from_u32(123456789).unwrap();

        ManagerConfig {
            device_id,
            device_name: "Test Device".to_string(),
            service_port: 7070,
            password_hash_path: temp_dir.path().join("password.hash"),
            max_connections: 5,
        }
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let config = create_test_config();
        let manager = ConnectionManager::new(config.clone());

        assert_eq!(manager.device_id(), config.device_id);
    }

    #[tokio::test]
    async fn test_manager_start_stop() {
        let config = create_test_config();
        let manager = ConnectionManager::new(config);

        assert!(manager.start().await.is_ok());
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_connect_disconnect() {
        let config = create_test_config();
        let manager = ConnectionManager::new(config);

        let remote_id = DeviceId::from_u32(987654321).unwrap();

        // Connect
        assert!(manager.connect(remote_id, None).await.is_ok());
        assert!(manager.is_connected(remote_id).await);

        // Disconnect
        assert!(manager.disconnect(remote_id).await.is_ok());
        assert!(!manager.is_connected(remote_id).await);
    }

    #[tokio::test]
    async fn test_get_active_connections() {
        let config = create_test_config();
        let manager = ConnectionManager::new(config);

        let remote_id1 = DeviceId::from_u32(111111111).unwrap();
        let remote_id2 = DeviceId::from_u32(222222222).unwrap();

        manager.connect(remote_id1, None).await.unwrap();
        manager.connect(remote_id2, None).await.unwrap();

        let active = manager.get_active_connections().await;
        assert_eq!(active.len(), 2);
    }
}
