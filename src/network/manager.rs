//! Connection manager for RemoteDesk
//!
//! This module provides the main interface for managing P2P connections,
//! coordinating peer discovery, and handling the connection lifecycle with
//! real QUIC networking.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::error::{NetworkError, NetworkResult};
use crate::network::cert::{self, CertPair};
use crate::network::discovery::{PeerDiscovery, PeerEvent, PeerInfo};
use crate::network::listener::{AcceptedConnection, ConnectionListener, IncomingConnection, PendingConnection};
use crate::network::protocol::{
    ConnectionAccept, ConnectionRequest, DesktopInfo, Message, MessagePayload, MessageType,
    RejectReason, CURRENT_PROTOCOL_VERSION,
};
use crate::network::quic::{QuicConfig, QuicConnection, QuicEndpoint, QuicError, DEFAULT_QUIC_PORT};
use crate::network::stream::BiStream;
use crate::network::{Connection, ConnectionInfo, ConnectionRole, ConnectionState};
use crate::security::{DeviceId, PasswordManager};

/// Connection manager configuration
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Local device ID
    pub device_id: DeviceId,
    /// Local device name
    pub device_name: String,
    /// Service port
    pub service_port: u16,
    /// Path to configuration directory (for certificates)
    pub config_dir: PathBuf,
    /// Path to password hash file
    pub password_hash_path: PathBuf,
    /// Maximum concurrent connections
    pub max_connections: usize,
}

impl ManagerConfig {
    /// Creates a new manager config with defaults
    pub fn new(device_id: DeviceId, device_name: String, config_dir: PathBuf) -> Self {
        Self {
            device_id,
            device_name,
            service_port: DEFAULT_QUIC_PORT,
            password_hash_path: config_dir.join("password.hash"),
            config_dir,
            max_connections: 5,
        }
    }

    /// Sets the service port
    pub fn with_port(mut self, port: u16) -> Self {
        self.service_port = port;
        self
    }
}

/// Events emitted by the connection manager
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Connection request received (awaiting accept/reject)
    ConnectionRequest {
        remote_id: DeviceId,
        remote_name: String,
        has_password: bool,
        connection_id: u64,
    },
    /// Connection established
    Connected { remote_id: DeviceId },
    /// Connection closed
    Disconnected { remote_id: DeviceId, reason: String },
    /// Peer discovered via mDNS
    PeerDiscovered { peer_info: PeerInfo },
    /// Peer lost
    PeerLost { device_id: DeviceId },
}

/// Result of a successful connection (for session creation)
pub struct EstablishedConnection {
    /// The QUIC connection
    pub connection: QuicConnection,
    /// Control stream for the session
    pub control_stream: BiStream<Message>,
    /// Remote device ID
    pub remote_device_id: DeviceId,
    /// Remote device name
    pub remote_name: String,
    /// Session ID
    pub session_id: [u8; 16],
    /// Connection role
    pub role: ConnectionRole,
}

/// Main connection manager
pub struct ConnectionManager {
    /// Configuration
    config: ManagerConfig,
    /// Certificate pair for QUIC
    cert_pair: CertPair,
    /// QUIC endpoint
    endpoint: Option<Arc<QuicEndpoint>>,
    /// Active connections
    connections: Arc<RwLock<HashMap<DeviceId, Arc<Connection>>>>,
    /// Established QUIC connections ready for session use
    quic_connections: Arc<RwLock<HashMap<DeviceId, EstablishedConnection>>>,
    /// Peer discovery
    discovery: Arc<RwLock<PeerDiscovery>>,
    /// Pending incoming connections awaiting accept/reject
    pending_connections: Arc<RwLock<HashMap<u64, PendingConnection>>>,
    /// Event channel sender
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    /// Event channel receiver
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<ConnectionEvent>>>,
    /// Listener task handle
    listener_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConnectionManager {
    /// Creates a new connection manager
    pub fn new(config: ManagerConfig) -> NetworkResult<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Load or create certificate
        let cert_pair = cert::load_or_create_cert(&config.config_dir, config.device_id.as_u32())
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let discovery = PeerDiscovery::new(
            config.device_id,
            config.device_name.clone(),
            config.service_port,
        );

        Ok(Self {
            config,
            cert_pair,
            endpoint: None,
            connections: Arc::new(RwLock::new(HashMap::new())),
            quic_connections: Arc::new(RwLock::new(HashMap::new())),
            discovery: Arc::new(RwLock::new(discovery)),
            pending_connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            listener_handle: None,
        })
    }

    /// Starts the connection manager (QUIC endpoint, listener, and discovery)
    pub async fn start(&mut self) -> NetworkResult<()> {
        info!("Starting connection manager");

        // Create QUIC endpoint
        let quic_config = QuicConfig::default()
            .with_bind_addr(SocketAddr::from(([0, 0, 0, 0], self.config.service_port)))
            .with_cert_pair(self.cert_pair.clone());

        let endpoint = Arc::new(
            QuicEndpoint::new(quic_config)
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?,
        );

        self.endpoint = Some(endpoint.clone());

        // Start connection listener
        let (listener, mut incoming_rx) = ConnectionListener::new(
            endpoint.clone(),
            self.config.device_id,
            self.config.device_name.clone(),
        );

        // Spawn listener task
        let listener_handle = tokio::spawn(async move {
            listener.run().await;
        });
        self.listener_handle = Some(listener_handle);

        // Spawn task to handle incoming connections
        let event_tx = self.event_tx.clone();
        let pending_connections = self.pending_connections.clone();
        let password_hash_path = self.config.password_hash_path.clone();

        tokio::spawn(async move {
            while let Some((incoming, pending)) = incoming_rx.recv().await {
                // Check if password is required
                let password_required = PasswordManager::is_password_set(&password_hash_path);

                if password_required && !incoming.has_password {
                    // Reject - password required but not provided
                    warn!(
                        "Rejecting connection from {}: password required",
                        incoming.remote_device_id.format_with_spaces()
                    );
                    let _ = pending.reject(RejectReason::InvalidPassword, None).await;
                    continue;
                }

                // Store pending connection
                pending_connections
                    .write()
                    .await
                    .insert(incoming.connection_id, pending);

                // Emit event for UI to handle
                let _ = event_tx.send(ConnectionEvent::ConnectionRequest {
                    remote_id: incoming.remote_device_id,
                    remote_name: incoming.remote_name,
                    has_password: incoming.has_password,
                    connection_id: incoming.connection_id,
                });
            }
        });

        // Start peer discovery
        let mut discovery = self.discovery.write().await;
        discovery
            .start_advertising()
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e))?;

        let peer_event_rx = discovery
            .start_discovery()
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e))?;

        // Forward peer events to main event channel
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let mut peer_event_rx = peer_event_rx;
            while let Some(event) = peer_event_rx.recv().await {
                let conn_event = match event {
                    PeerEvent::Discovered(info) => ConnectionEvent::PeerDiscovered { peer_info: info },
                    PeerEvent::Updated(_) => continue, // Skip updates
                    PeerEvent::Lost(id) => ConnectionEvent::PeerLost { device_id: id },
                };
                let _ = event_tx.send(conn_event);
            }
        });

        info!("Connection manager started successfully");
        Ok(())
    }

    /// Stops the connection manager
    pub async fn stop(&mut self) {
        info!("Stopping connection manager");

        // Stop listener
        if let Some(handle) = self.listener_handle.take() {
            handle.abort();
        }

        // Close all connections
        let connections = self.connections.read().await;
        for (_, conn) in connections.iter() {
            conn.set_state(ConnectionState::Disconnecting).await;
        }

        // Close QUIC connections
        let mut quic_conns = self.quic_connections.write().await;
        for (_, conn) in quic_conns.drain() {
            conn.connection.close("manager shutdown");
        }

        // Stop discovery
        let mut discovery = self.discovery.write().await;
        discovery.stop_advertising().await;
        discovery.stop_discovery().await;

        // Close endpoint
        if let Some(endpoint) = &self.endpoint {
            endpoint.close();
        }

        info!("Connection manager stopped");
    }

    /// Initiates a connection to a remote device
    pub async fn connect(
        &self,
        remote_id: DeviceId,
        password: Option<String>,
    ) -> NetworkResult<EstablishedConnection> {
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

        // Resolve peer address
        let discovery = self.discovery.read().await;
        let addresses = discovery.resolve(remote_id).await;
        drop(discovery);

        let remote_addr = addresses
            .and_then(|addrs| addrs.into_iter().next())
            .ok_or_else(|| {
                NetworkError::ConnectionFailed(format!(
                    "Could not resolve address for device {}",
                    remote_id.format_with_spaces()
                ))
            })?;

        // Get endpoint
        let endpoint = self.endpoint.as_ref().ok_or_else(|| {
            NetworkError::ConnectionFailed("Connection manager not started".to_string())
        })?;

        // Connect via QUIC
        let quic_conn = endpoint
            .connect(remote_addr, "localhost")
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        // Open control stream and perform handshake
        let (send, recv) = quic_conn
            .open_bi()
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let mut control_stream: BiStream<Message> = BiStream::new(send, recv);

        // Create and send connection request
        let password_hash = password.map(|pwd| {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(pwd.as_bytes());
            hasher.update(remote_id.as_u32().to_le_bytes());
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            hash
        });

        let request = ConnectionRequest::new(
            self.config.device_id,
            self.config.device_name.clone(),
            remote_id,
            password_hash,
        );

        let request_msg = Message::new(
            MessageType::ConnectionRequest,
            MessagePayload::ConnectionRequest(request),
        );

        control_stream
            .send(request_msg)
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        // Wait for response
        let response = control_stream
            .recv()
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        // Handle response
        match response.payload {
            MessagePayload::ConnectionAccept(accept) => {
                info!(
                    "Connection accepted by {} ({})",
                    remote_id.format_with_spaces(),
                    accept.host_name
                );

                // Create connection info
                let connection = Arc::new(Connection::new(
                    remote_id,
                    accept.host_name.clone(),
                    remote_addr,
                    ConnectionRole::Client,
                ));
                connection.set_state(ConnectionState::Connected).await;
                connection.set_session_id(accept.session_id).await;

                // Store connection
                let mut connections = self.connections.write().await;
                connections.insert(remote_id, connection);

                // Emit connected event
                let _ = self.event_tx.send(ConnectionEvent::Connected { remote_id });

                Ok(EstablishedConnection {
                    connection: quic_conn,
                    control_stream,
                    remote_device_id: remote_id,
                    remote_name: accept.host_name,
                    session_id: accept.session_id,
                    role: ConnectionRole::Client,
                })
            }
            MessagePayload::ConnectionReject(reject) => {
                warn!(
                    "Connection rejected by {}: {:?}",
                    remote_id.format_with_spaces(),
                    reject.reason
                );
                Err(NetworkError::ConnectionRejected(format!(
                    "{:?}: {}",
                    reject.reason,
                    reject.message.unwrap_or_default()
                )))
            }
            _ => Err(NetworkError::ProtocolError(
                "Unexpected response to connection request".to_string(),
            )),
        }
    }

    /// Accepts a pending connection request
    pub async fn accept_connection(
        &self,
        connection_id: u64,
    ) -> NetworkResult<EstablishedConnection> {
        let pending = {
            let mut pending_conns = self.pending_connections.write().await;
            pending_conns.remove(&connection_id)
        };

        let pending = pending.ok_or_else(|| {
            NetworkError::ConnectionFailed("Pending connection not found".to_string())
        })?;

        let remote_id = DeviceId::from_u32(pending.request().client_id)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        info!("Accepting connection from {}", remote_id.format_with_spaces());

        // Accept the connection
        let accepted = pending
            .accept(
                self.config.device_name.clone(),
                DesktopInfo::current(),
            )
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        // Create connection info
        let connection = Arc::new(Connection::new(
            accepted.remote_device_id,
            accepted.remote_name.clone(),
            accepted.connection.remote_address(),
            ConnectionRole::Host,
        ));
        connection.set_state(ConnectionState::Connected).await;
        connection.set_session_id(accepted.session_id).await;

        // Store connection
        let mut connections = self.connections.write().await;
        connections.insert(accepted.remote_device_id, connection);

        // Emit connected event
        let _ = self.event_tx.send(ConnectionEvent::Connected {
            remote_id: accepted.remote_device_id,
        });

        Ok(EstablishedConnection {
            connection: accepted.connection,
            control_stream: accepted.control_stream,
            remote_device_id: accepted.remote_device_id,
            remote_name: accepted.remote_name,
            session_id: accepted.session_id,
            role: ConnectionRole::Host,
        })
    }

    /// Rejects a pending connection request
    pub async fn reject_connection(
        &self,
        connection_id: u64,
        reason: RejectReason,
    ) -> NetworkResult<()> {
        let pending = {
            let mut pending_conns = self.pending_connections.write().await;
            pending_conns.remove(&connection_id)
        };

        let pending = pending.ok_or_else(|| {
            NetworkError::ConnectionFailed("Pending connection not found".to_string())
        })?;

        pending
            .reject(reason, None)
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        Ok(())
    }

    /// Disconnects from a remote device
    pub async fn disconnect(&self, remote_id: DeviceId) -> NetworkResult<()> {
        info!("Disconnecting from {}", remote_id.format_with_spaces());

        // Remove from active connections
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(&remote_id) {
            connection.set_state(ConnectionState::Disconnecting).await;
            connection.set_state(ConnectionState::Disconnected).await;
        }

        // Remove QUIC connection
        let mut quic_conns = self.quic_connections.write().await;
        if let Some(conn) = quic_conns.remove(&remote_id) {
            conn.connection.close("user disconnect");
        }

        // Emit disconnected event
        let _ = self.event_tx.send(ConnectionEvent::Disconnected {
            remote_id,
            reason: "User initiated".to_string(),
        });

        info!("Disconnected from {}", remote_id.format_with_spaces());
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

    /// Gets the event receiver for subscription
    pub fn event_sender(&self) -> mpsc::UnboundedSender<ConnectionEvent> {
        self.event_tx.clone()
    }

    /// Gets the local device ID
    pub fn device_id(&self) -> DeviceId {
        self.config.device_id
    }

    /// Gets the local address the endpoint is bound to
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.endpoint.as_ref().map(|e| e.local_addr())
    }

    /// Gets all discovered peers
    pub async fn get_discovered_peers(&self) -> Vec<PeerInfo> {
        let discovery = self.discovery.read().await;
        discovery.get_all_peers().await
    }

    /// Manually adds a peer (for direct IP connection)
    pub async fn add_peer(&self, device_id: DeviceId, name: String, addr: SocketAddr) {
        let discovery = self.discovery.read().await;
        discovery
            .add_peer(PeerInfo::new(device_id, name, vec![addr]))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (ManagerConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let device_id = DeviceId::from_u32(123456789).unwrap();

        let config = ManagerConfig::new(
            device_id,
            "Test Device".to_string(),
            temp_dir.path().to_path_buf(),
        );

        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let (config, _temp_dir) = create_test_config();
        let manager = ConnectionManager::new(config.clone()).unwrap();

        assert_eq!(manager.device_id(), config.device_id);
    }

    #[tokio::test]
    async fn test_manager_config_builder() {
        let device_id = DeviceId::from_u32(123456789).unwrap();
        let temp_dir = TempDir::new().unwrap();

        let config = ManagerConfig::new(
            device_id,
            "Test".to_string(),
            temp_dir.path().to_path_buf(),
        )
        .with_port(8080);

        assert_eq!(config.service_port, 8080);
    }
}
