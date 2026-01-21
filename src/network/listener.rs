//! Connection listener for accepting incoming QUIC connections
//!
//! This module handles the server-side of connection establishment:
//! - Accepting incoming QUIC connections
//! - Performing the protocol handshake
//! - Setting up session transports for accepted connections

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::network::protocol::{
    ConnectionAccept, ConnectionReject, ConnectionRequest, DesktopInfo, Message, MessagePayload,
    MessageType, RejectReason, CURRENT_PROTOCOL_VERSION,
};
use crate::network::quic::{QuicConnection, QuicEndpoint, QuicError, QuicResult};
use crate::network::stream::{BiStream, StreamReceiver, StreamSender};
use crate::security::DeviceId;

/// Event emitted when a connection request is received
#[derive(Debug, Clone)]
pub struct IncomingConnection {
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Remote device ID
    pub remote_device_id: DeviceId,
    /// Remote device name
    pub remote_name: String,
    /// Whether password was provided
    pub has_password: bool,
    /// The password hash if provided
    pub password_hash: Option<[u8; 32]>,
    /// Connection ID for responding
    pub connection_id: u64,
}

/// Response to an incoming connection request
#[derive(Debug, Clone)]
pub enum ConnectionResponse {
    /// Accept the connection
    Accept {
        /// Host device name
        host_name: String,
        /// Desktop information
        desktop_info: DesktopInfo,
    },
    /// Reject the connection
    Reject {
        /// Rejection reason
        reason: RejectReason,
        /// Optional message
        message: Option<String>,
    },
}

/// Connection listener that accepts incoming QUIC connections
pub struct ConnectionListener {
    /// The QUIC endpoint
    endpoint: Arc<QuicEndpoint>,
    /// Local device ID
    local_device_id: DeviceId,
    /// Local device name
    local_device_name: String,
    /// Channel for incoming connection events
    incoming_tx: mpsc::UnboundedSender<(IncomingConnection, PendingConnection)>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
}

/// A pending connection awaiting accept/reject decision
pub struct PendingConnection {
    /// The QUIC connection
    connection: QuicConnection,
    /// Control stream for handshake
    control_stream: BiStream<Message>,
    /// Original request message
    request: ConnectionRequest,
}

impl PendingConnection {
    /// Accepts the connection, completing the handshake
    pub async fn accept(
        mut self,
        host_name: String,
        desktop_info: DesktopInfo,
    ) -> QuicResult<AcceptedConnection> {
        // Create accept message
        let accept = ConnectionAccept::new(host_name.clone(), desktop_info.clone());
        let session_id = accept.session_id;

        let response = Message::new(
            MessageType::ConnectionAccept,
            MessagePayload::ConnectionAccept(accept),
        );

        // Send response
        self.control_stream
            .send(response)
            .await
            .map_err(|e| QuicError::StreamError(e.to_string()))?;

        info!(
            "Accepted connection from device {}",
            self.request.client_id
        );

        Ok(AcceptedConnection {
            connection: self.connection,
            control_stream: self.control_stream,
            remote_device_id: DeviceId::from_u32(self.request.client_id)
                .map_err(|e| QuicError::ConnectionFailed(e.to_string()))?,
            remote_name: self.request.client_name,
            session_id,
        })
    }

    /// Rejects the connection
    pub async fn reject(mut self, reason: RejectReason, message: Option<String>) -> QuicResult<()> {
        let reject = ConnectionReject::new(reason, message);

        let response = Message::new(
            MessageType::ConnectionReject,
            MessagePayload::ConnectionReject(reject),
        );

        // Send response (ignore errors since we're rejecting anyway)
        let _ = self.control_stream.send(response).await;

        info!(
            "Rejected connection from device {}: {:?}",
            self.request.client_id, reason
        );

        // Close the connection
        self.connection.close("connection rejected");

        Ok(())
    }

    /// Returns the connection request details
    pub fn request(&self) -> &ConnectionRequest {
        &self.request
    }

    /// Returns the remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.connection.remote_address()
    }
}

/// An accepted connection ready for session use
pub struct AcceptedConnection {
    /// The QUIC connection
    pub connection: QuicConnection,
    /// Control stream
    pub control_stream: BiStream<Message>,
    /// Remote device ID
    pub remote_device_id: DeviceId,
    /// Remote device name
    pub remote_name: String,
    /// Session ID
    pub session_id: [u8; 16],
}

impl ConnectionListener {
    /// Creates a new connection listener
    pub fn new(
        endpoint: Arc<QuicEndpoint>,
        local_device_id: DeviceId,
        local_device_name: String,
    ) -> (Self, mpsc::UnboundedReceiver<(IncomingConnection, PendingConnection)>) {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();

        let listener = Self {
            endpoint,
            local_device_id,
            local_device_name,
            incoming_tx,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        (listener, incoming_rx)
    }

    /// Starts the listener loop
    ///
    /// This runs in a loop accepting connections until stopped.
    pub async fn run(&self) {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("Connection listener started");

        let mut connection_counter: u64 = 0;

        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            match self.endpoint.accept().await {
                Some(Ok(connection)) => {
                    connection_counter += 1;
                    let connection_id = connection_counter;

                    // Handle connection in a separate task
                    let incoming_tx = self.incoming_tx.clone();
                    let local_device_id = self.local_device_id;

                    tokio::spawn(async move {
                        match Self::handle_incoming_connection(
                            connection,
                            connection_id,
                            local_device_id,
                        )
                        .await
                        {
                            Ok((incoming, pending)) => {
                                if incoming_tx.send((incoming, pending)).is_err() {
                                    warn!("Failed to send incoming connection event");
                                }
                            }
                            Err(e) => {
                                error!("Failed to handle incoming connection: {}", e);
                            }
                        }
                    });
                }
                Some(Err(e)) => {
                    error!("Failed to accept connection: {}", e);
                }
                None => {
                    debug!("Endpoint closed, stopping listener");
                    break;
                }
            }
        }

        info!("Connection listener stopped");
    }

    /// Stops the listener
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Handles an incoming connection, performing the initial handshake
    async fn handle_incoming_connection(
        connection: QuicConnection,
        connection_id: u64,
        local_device_id: DeviceId,
    ) -> QuicResult<(IncomingConnection, PendingConnection)> {
        let remote_addr = connection.remote_address();
        debug!("Handling incoming connection from {}", remote_addr);

        // Accept the control stream (client opens it first)
        let (send_stream, recv_stream) = connection.accept_bi().await?;
        let mut control_stream: BiStream<Message> = BiStream::new(send_stream, recv_stream);

        // Receive connection request
        let request_msg = control_stream
            .recv()
            .await
            .map_err(|e| QuicError::StreamError(e.to_string()))?;

        // Validate message type
        let request = match request_msg.payload {
            MessagePayload::ConnectionRequest(req) => req,
            _ => {
                return Err(QuicError::ConnectionFailed(
                    "Expected ConnectionRequest message".to_string(),
                ));
            }
        };

        // Validate protocol version
        if request.protocol_version != CURRENT_PROTOCOL_VERSION {
            // Send reject for version mismatch
            let reject = ConnectionReject::new(
                RejectReason::UnsupportedVersion,
                Some(format!(
                    "Expected protocol version {}, got {}",
                    CURRENT_PROTOCOL_VERSION, request.protocol_version
                )),
            );
            let response = Message::new(
                MessageType::ConnectionReject,
                MessagePayload::ConnectionReject(reject),
            );
            let _ = control_stream.send(response).await;
            connection.close("protocol version mismatch");

            return Err(QuicError::ConnectionFailed(
                "Protocol version mismatch".to_string(),
            ));
        }

        // Validate host ID matches
        if request.host_id != local_device_id.as_u32() {
            let reject = ConnectionReject::new(
                RejectReason::InvalidId,
                Some("Host ID does not match".to_string()),
            );
            let response = Message::new(
                MessageType::ConnectionReject,
                MessagePayload::ConnectionReject(reject),
            );
            let _ = control_stream.send(response).await;
            connection.close("invalid host ID");

            return Err(QuicError::ConnectionFailed("Invalid host ID".to_string()));
        }

        let remote_device_id = DeviceId::from_u32(request.client_id)
            .map_err(|e| QuicError::ConnectionFailed(e.to_string()))?;

        info!(
            "Received connection request from {} ({})",
            remote_device_id.format_with_spaces(),
            request.client_name
        );

        let incoming = IncomingConnection {
            remote_addr,
            remote_device_id,
            remote_name: request.client_name.clone(),
            has_password: request.password_hash.is_some(),
            password_hash: request.password_hash,
            connection_id,
        };

        let pending = PendingConnection {
            connection,
            control_stream,
            request,
        };

        Ok((incoming, pending))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incoming_connection() {
        let incoming = IncomingConnection {
            remote_addr: "127.0.0.1:8080".parse().unwrap(),
            remote_device_id: DeviceId::from_u32(123456789).unwrap(),
            remote_name: "Test Device".to_string(),
            has_password: false,
            password_hash: None,
            connection_id: 1,
        };

        assert_eq!(incoming.remote_device_id.as_u32(), 123456789);
        assert!(!incoming.has_password);
    }

    #[test]
    fn test_connection_response() {
        let response = ConnectionResponse::Accept {
            host_name: "Host".to_string(),
            desktop_info: DesktopInfo::current(),
        };

        assert!(matches!(response, ConnectionResponse::Accept { .. }));

        let response = ConnectionResponse::Reject {
            reason: RejectReason::UserDenied,
            message: None,
        };

        assert!(matches!(response, ConnectionResponse::Reject { .. }));
    }
}
