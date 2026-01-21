//! QUIC transport layer for RemoteDesk
//!
//! This module provides QUIC-based networking using the quinn library.
//! It handles endpoint creation, connection establishment, and connection management.

use quinn::{
    ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig, TransportConfig,
    VarInt,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::network::cert::{self, CertPair};

/// Default QUIC port for RemoteDesk
pub const DEFAULT_QUIC_PORT: u16 = 7070;

/// Maximum idle timeout for connections
const IDLE_TIMEOUT_SECS: u64 = 30;

/// Keep-alive interval
const KEEP_ALIVE_INTERVAL_SECS: u64 = 5;

/// Maximum concurrent bidirectional streams
const MAX_CONCURRENT_BIDI_STREAMS: u32 = 10;

/// Maximum concurrent unidirectional streams
const MAX_CONCURRENT_UNI_STREAMS: u32 = 10;

/// Error type for QUIC operations
#[derive(Debug, thiserror::Error)]
pub enum QuicError {
    #[error("Failed to create endpoint: {0}")]
    EndpointCreation(String),

    #[error("Failed to connect: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed: {0}")]
    ConnectionClosed(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Certificate error: {0}")]
    CertificateError(#[from] cert::CertError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Quinn connection error: {0}")]
    QuinnConnection(#[from] quinn::ConnectionError),

    #[error("Quinn connect error: {0}")]
    QuinnConnect(#[from] quinn::ConnectError),
}

/// Result type for QUIC operations
pub type QuicResult<T> = Result<T, QuicError>;

/// Configuration for QUIC endpoint
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Address to bind to
    pub bind_addr: SocketAddr,
    /// Certificate pair for TLS
    pub cert_pair: Option<CertPair>,
    /// Maximum idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Keep-alive interval in seconds
    pub keep_alive_interval_secs: u64,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], DEFAULT_QUIC_PORT)),
            cert_pair: None,
            idle_timeout_secs: IDLE_TIMEOUT_SECS,
            keep_alive_interval_secs: KEEP_ALIVE_INTERVAL_SECS,
        }
    }
}

impl QuicConfig {
    /// Creates a new config with the specified bind address
    pub fn with_bind_addr(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Sets the certificate pair
    pub fn with_cert_pair(mut self, cert_pair: CertPair) -> Self {
        self.cert_pair = Some(cert_pair);
        self
    }

    /// Sets the idle timeout
    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }
}

/// QUIC endpoint that can accept and initiate connections
pub struct QuicEndpoint {
    /// The underlying quinn endpoint
    endpoint: Endpoint,
    /// Local address
    local_addr: SocketAddr,
}

impl QuicEndpoint {
    /// Creates a new QUIC endpoint for both server and client use
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the endpoint
    pub fn new(config: QuicConfig) -> QuicResult<Self> {
        let cert_pair = config.cert_pair
            .ok_or_else(|| QuicError::EndpointCreation("Certificate pair required".to_string()))?;

        // Create transport config
        let mut transport_config = TransportConfig::default();
        transport_config.max_idle_timeout(Some(
            Duration::from_secs(config.idle_timeout_secs).try_into().unwrap()
        ));
        transport_config.keep_alive_interval(Some(
            Duration::from_secs(config.keep_alive_interval_secs)
        ));
        transport_config.max_concurrent_bidi_streams(VarInt::from_u32(MAX_CONCURRENT_BIDI_STREAMS));
        transport_config.max_concurrent_uni_streams(VarInt::from_u32(MAX_CONCURRENT_UNI_STREAMS));

        let transport_config = Arc::new(transport_config);

        // Create server config
        let rustls_server_config = cert::create_server_config(&cert_pair)?;
        let mut server_config = ServerConfig::with_crypto(Arc::new(rustls_server_config));
        server_config.transport_config(transport_config.clone());

        // Create client config
        let client_config = cert::create_client_config()?;
        let mut client_config = ClientConfig::new(Arc::new(client_config));
        client_config.transport_config(transport_config);

        // Create endpoint
        let mut endpoint = Endpoint::server(server_config, config.bind_addr)
            .map_err(|e| QuicError::EndpointCreation(e.to_string()))?;

        endpoint.set_default_client_config(client_config);

        let local_addr = endpoint.local_addr()
            .map_err(|e| QuicError::EndpointCreation(e.to_string()))?;

        info!("QUIC endpoint created on {}", local_addr);

        Ok(Self {
            endpoint,
            local_addr,
        })
    }

    /// Creates a client-only endpoint (for connecting without accepting)
    pub fn client_only() -> QuicResult<Self> {
        let client_config = cert::create_client_config()?;
        let client_config = ClientConfig::new(Arc::new(client_config));

        // Bind to any available port
        let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();

        let mut endpoint = Endpoint::client(bind_addr)
            .map_err(|e| QuicError::EndpointCreation(e.to_string()))?;

        endpoint.set_default_client_config(client_config);

        let local_addr = endpoint.local_addr()
            .map_err(|e| QuicError::EndpointCreation(e.to_string()))?;

        debug!("Client-only QUIC endpoint created on {}", local_addr);

        Ok(Self {
            endpoint,
            local_addr,
        })
    }

    /// Returns the local address this endpoint is bound to
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Connects to a remote endpoint
    ///
    /// # Arguments
    ///
    /// * `addr` - Remote address to connect to
    /// * `server_name` - Server name for TLS (can be "localhost" for local connections)
    pub async fn connect(&self, addr: SocketAddr, server_name: &str) -> QuicResult<QuicConnection> {
        info!("Connecting to {} ({})", addr, server_name);

        let connection = self.endpoint
            .connect(addr, server_name)?
            .await?;

        let remote_addr = connection.remote_address();
        info!("Connected to {}", remote_addr);

        Ok(QuicConnection::new(connection))
    }

    /// Accepts an incoming connection
    ///
    /// Returns `None` if the endpoint is closed
    pub async fn accept(&self) -> Option<QuicResult<QuicConnection>> {
        let incoming = self.endpoint.accept().await?;

        Some(async move {
            let connection = incoming.await
                .map_err(QuicError::QuinnConnection)?;

            let remote_addr = connection.remote_address();
            info!("Accepted connection from {}", remote_addr);

            Ok(QuicConnection::new(connection))
        }.await)
    }

    /// Closes the endpoint
    pub fn close(&self) {
        info!("Closing QUIC endpoint");
        self.endpoint.close(VarInt::from_u32(0), b"shutdown");
    }

    /// Waits for the endpoint to be idle (all connections closed)
    pub async fn wait_idle(&self) {
        self.endpoint.wait_idle().await;
    }
}

/// A QUIC connection to a remote peer
pub struct QuicConnection {
    /// The underlying quinn connection
    connection: Connection,
}

impl QuicConnection {
    /// Creates a new QuicConnection wrapper
    fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Returns the remote address
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Returns the stable connection ID
    pub fn stable_id(&self) -> usize {
        self.connection.stable_id()
    }

    /// Opens a new bidirectional stream
    pub async fn open_bi(&self) -> QuicResult<(SendStream, RecvStream)> {
        let (send, recv) = self.connection
            .open_bi()
            .await
            .map_err(|e| QuicError::StreamError(e.to_string()))?;

        debug!("Opened bidirectional stream");
        Ok((send, recv))
    }

    /// Opens a new unidirectional send stream
    pub async fn open_uni(&self) -> QuicResult<SendStream> {
        let send = self.connection
            .open_uni()
            .await
            .map_err(|e| QuicError::StreamError(e.to_string()))?;

        debug!("Opened unidirectional stream");
        Ok(send)
    }

    /// Accepts an incoming bidirectional stream
    pub async fn accept_bi(&self) -> QuicResult<(SendStream, RecvStream)> {
        let (send, recv) = self.connection
            .accept_bi()
            .await
            .map_err(|e| QuicError::StreamError(e.to_string()))?;

        debug!("Accepted bidirectional stream");
        Ok((send, recv))
    }

    /// Accepts an incoming unidirectional stream
    pub async fn accept_uni(&self) -> QuicResult<RecvStream> {
        let recv = self.connection
            .accept_uni()
            .await
            .map_err(|e| QuicError::StreamError(e.to_string()))?;

        debug!("Accepted unidirectional stream");
        Ok(recv)
    }

    /// Closes the connection
    pub fn close(&self, reason: &str) {
        info!("Closing connection: {}", reason);
        self.connection.close(VarInt::from_u32(0), reason.as_bytes());
    }

    /// Returns the RTT estimate in milliseconds
    pub fn rtt_ms(&self) -> u64 {
        self.connection.rtt().as_millis() as u64
    }

    /// Checks if the connection is closed
    pub fn is_closed(&self) -> bool {
        self.connection.close_reason().is_some()
    }

    /// Gets the close reason if the connection is closed
    pub fn close_reason(&self) -> Option<String> {
        self.connection.close_reason().map(|e| e.to_string())
    }
}

impl Clone for QuicConnection {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
        }
    }
}

/// Stream identifiers for different data channels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamType {
    /// Control and handshake messages
    Control = 0,
    /// Video frame data (host → client)
    Video = 1,
    /// Input events (client → host)
    Input = 2,
    /// Clipboard synchronization
    Clipboard = 3,
}

impl StreamType {
    /// Converts a u8 to StreamType
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(StreamType::Control),
            1 => Some(StreamType::Video),
            2 => Some(StreamType::Input),
            3 => Some(StreamType::Clipboard),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_endpoint(port: u16) -> QuicResult<QuicEndpoint> {
        let temp_dir = TempDir::new().unwrap();
        let cert_pair = cert::load_or_create_cert(temp_dir.path(), 123456789).unwrap();

        let config = QuicConfig::default()
            .with_bind_addr(SocketAddr::from(([127, 0, 0, 1], port)))
            .with_cert_pair(cert_pair);

        QuicEndpoint::new(config)
    }

    #[test]
    fn test_quic_config_default() {
        let config = QuicConfig::default();
        assert_eq!(config.bind_addr.port(), DEFAULT_QUIC_PORT);
        assert!(config.cert_pair.is_none());
    }

    #[test]
    fn test_quic_config_builder() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let config = QuicConfig::default()
            .with_bind_addr(addr)
            .with_idle_timeout(60);

        assert_eq!(config.bind_addr, addr);
        assert_eq!(config.idle_timeout_secs, 60);
    }

    #[tokio::test]
    async fn test_client_only_endpoint() {
        let endpoint = QuicEndpoint::client_only().unwrap();
        assert!(endpoint.local_addr().port() > 0);
    }

    #[tokio::test]
    async fn test_endpoint_creation() {
        let endpoint = create_test_endpoint(17070).unwrap();
        assert_eq!(endpoint.local_addr().port(), 17070);
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        // Create server endpoint
        let server = create_test_endpoint(17071).unwrap();
        let server_addr = server.local_addr();

        // Create client endpoint
        let client = QuicEndpoint::client_only().unwrap();

        // Spawn server accept task
        let server_handle = tokio::spawn(async move {
            let conn = server.accept().await.unwrap().unwrap();
            assert!(!conn.is_closed());
            conn
        });

        // Connect from client
        let client_conn = client.connect(server_addr, "localhost").await.unwrap();
        assert!(!client_conn.is_closed());

        // Wait for server to accept
        let server_conn = server_handle.await.unwrap();

        // Open and accept a stream
        let (mut send, _recv) = client_conn.open_bi().await.unwrap();
        let (_, _) = server_conn.accept_bi().await.unwrap();

        // Close connections
        client_conn.close("test complete");
        server_conn.close("test complete");
    }

    #[test]
    fn test_stream_type_conversion() {
        assert_eq!(StreamType::from_u8(0), Some(StreamType::Control));
        assert_eq!(StreamType::from_u8(1), Some(StreamType::Video));
        assert_eq!(StreamType::from_u8(2), Some(StreamType::Input));
        assert_eq!(StreamType::from_u8(3), Some(StreamType::Clipboard));
        assert_eq!(StreamType::from_u8(4), None);
    }
}
