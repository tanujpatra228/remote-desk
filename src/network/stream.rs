//! QUIC stream adapters for RemoteDesk
//!
//! This module provides adapters that bridge QUIC streams to typed message
//! channels, enabling seamless integration with the session transport system.

use bytes::{Buf, BufMut, BytesMut};
use quinn::{RecvStream, SendStream};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use tokio::sync::mpsc;
use tracing::{debug, error, trace, warn};

use crate::network::quic::QuicError;

/// Maximum message size (10 MB)
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Length prefix size (4 bytes for u32)
const LENGTH_PREFIX_SIZE: usize = 4;

/// Result type for stream operations
pub type StreamResult<T> = Result<T, StreamError>;

/// Error type for stream operations
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Stream write error: {0}")]
    WriteError(String),

    #[error("Stream read error: {0}")]
    ReadError(String),

    #[error("Stream closed")]
    StreamClosed,

    #[error("Message too large: {size} bytes (max {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Channel closed")]
    ChannelClosed,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<StreamError> for QuicError {
    fn from(err: StreamError) -> Self {
        QuicError::StreamError(err.to_string())
    }
}

/// Typed adapter for sending messages over a QUIC stream
///
/// Uses length-prefixed framing: each message is prefixed with a 4-byte
/// big-endian length, followed by bincode-serialized data.
pub struct StreamSender<T> {
    stream: SendStream,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> StreamSender<T> {
    /// Creates a new stream sender
    pub fn new(stream: SendStream) -> Self {
        Self {
            stream,
            _phantom: PhantomData,
        }
    }

    /// Sends a message over the stream (takes ownership for Send safety)
    pub async fn send(&mut self, msg: T) -> StreamResult<usize> {
        // Serialize the message
        let data = bincode::serialize(&msg)
            .map_err(|e| StreamError::Serialization(e.to_string()))?;

        let size = data.len();

        if size > MAX_MESSAGE_SIZE {
            return Err(StreamError::MessageTooLarge {
                size,
                max: MAX_MESSAGE_SIZE,
            });
        }

        // Write length prefix
        let len_bytes = (size as u32).to_be_bytes();
        self.stream
            .write_all(&len_bytes)
            .await
            .map_err(|e| StreamError::WriteError(e.to_string()))?;

        // Write message data
        self.stream
            .write_all(&data)
            .await
            .map_err(|e| StreamError::WriteError(e.to_string()))?;

        trace!("Sent message: {} bytes", size);
        Ok(size + LENGTH_PREFIX_SIZE)
    }

    /// Sends a reference to a message (for cases where ownership isn't needed)
    pub async fn send_ref(&mut self, msg: &T) -> StreamResult<usize> {
        // Serialize the message
        let data = bincode::serialize(msg)
            .map_err(|e| StreamError::Serialization(e.to_string()))?;

        let size = data.len();

        if size > MAX_MESSAGE_SIZE {
            return Err(StreamError::MessageTooLarge {
                size,
                max: MAX_MESSAGE_SIZE,
            });
        }

        // Write length prefix
        let len_bytes = (size as u32).to_be_bytes();
        self.stream
            .write_all(&len_bytes)
            .await
            .map_err(|e| StreamError::WriteError(e.to_string()))?;

        // Write message data
        self.stream
            .write_all(&data)
            .await
            .map_err(|e| StreamError::WriteError(e.to_string()))?;

        trace!("Sent message: {} bytes", size);
        Ok(size + LENGTH_PREFIX_SIZE)
    }

    /// Finishes the stream, signaling no more data will be sent
    pub async fn finish(mut self) -> StreamResult<()> {
        self.stream
            .finish()
            .await
            .map_err(|e| StreamError::WriteError(e.to_string()))?;
        Ok(())
    }
}

/// Typed adapter for receiving messages from a QUIC stream
pub struct StreamReceiver<T> {
    stream: RecvStream,
    buffer: BytesMut,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> StreamReceiver<T> {
    /// Creates a new stream receiver
    pub fn new(stream: RecvStream) -> Self {
        Self {
            stream,
            buffer: BytesMut::with_capacity(64 * 1024),
            _phantom: PhantomData,
        }
    }

    /// Receives a message from the stream
    pub async fn recv(&mut self) -> StreamResult<T> {
        // Read length prefix
        let len = self.read_length().await?;

        if len > MAX_MESSAGE_SIZE {
            return Err(StreamError::MessageTooLarge {
                size: len,
                max: MAX_MESSAGE_SIZE,
            });
        }

        // Read message data
        let data = self.read_exact(len).await?;

        // Deserialize
        let msg = bincode::deserialize(&data)
            .map_err(|e| StreamError::Deserialization(e.to_string()))?;

        trace!("Received message: {} bytes", len);
        Ok(msg)
    }

    /// Reads the 4-byte length prefix
    async fn read_length(&mut self) -> StreamResult<usize> {
        let data = self.read_exact(LENGTH_PREFIX_SIZE).await?;
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&data);
        Ok(u32::from_be_bytes(len_bytes) as usize)
    }

    /// Reads exactly `len` bytes from the stream
    async fn read_exact(&mut self, len: usize) -> StreamResult<Vec<u8>> {
        self.buffer.clear();
        self.buffer.reserve(len);

        while self.buffer.len() < len {
            match self.stream.read_chunk(len - self.buffer.len(), true).await {
                Ok(Some(chunk)) => {
                    self.buffer.put(chunk.bytes);
                }
                Ok(None) => {
                    return Err(StreamError::StreamClosed);
                }
                Err(e) => {
                    return Err(StreamError::ReadError(e.to_string()));
                }
            }
        }

        Ok(self.buffer.to_vec())
    }
}

/// Bidirectional typed stream adapter
pub struct BiStream<T> {
    pub sender: StreamSender<T>,
    pub receiver: StreamReceiver<T>,
}

impl<T: Serialize + DeserializeOwned> BiStream<T> {
    /// Creates a new bidirectional stream from quinn streams
    pub fn new(send: SendStream, recv: RecvStream) -> Self {
        Self {
            sender: StreamSender::new(send),
            receiver: StreamReceiver::new(recv),
        }
    }

    /// Sends a message (takes ownership)
    pub async fn send(&mut self, msg: T) -> StreamResult<usize> {
        self.sender.send(msg).await
    }

    /// Sends a message reference
    pub async fn send_ref(&mut self, msg: &T) -> StreamResult<usize> {
        self.sender.send_ref(msg).await
    }

    /// Receives a message
    pub async fn recv(&mut self) -> StreamResult<T> {
        self.receiver.recv().await
    }
}

/// Spawns a task that bridges a QUIC receive stream to an mpsc channel
///
/// This allows integrating QUIC streams with the existing SessionTransport
/// channel-based architecture.
pub fn spawn_recv_to_channel<T>(
    mut receiver: StreamReceiver<T>,
    tx: mpsc::Sender<T>,
) -> tokio::task::JoinHandle<()>
where
    T: DeserializeOwned + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(msg) => {
                    if tx.send(msg).await.is_err() {
                        debug!("Channel closed, stopping receive bridge");
                        break;
                    }
                }
                Err(StreamError::StreamClosed) => {
                    debug!("Stream closed, stopping receive bridge");
                    break;
                }
                Err(e) => {
                    error!("Stream receive error: {}", e);
                    break;
                }
            }
        }
    })
}

/// Spawns a task that bridges an mpsc channel to a QUIC send stream
pub fn spawn_channel_to_send<T>(
    mut sender: StreamSender<T>,
    mut rx: mpsc::Receiver<T>,
) -> tokio::task::JoinHandle<()>
where
    T: Serialize + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = sender.send(msg).await {
                error!("Stream send error: {}", e);
                break;
            }
        }
        debug!("Channel closed, stopping send bridge");

        // Gracefully finish the stream
        if let Err(e) = sender.finish().await {
            warn!("Failed to finish stream: {}", e);
        }
    })
}

/// Creates a bidirectional bridge between QUIC streams and mpsc channels
///
/// Returns handles to the spawned tasks for lifecycle management.
pub fn create_stream_channel_bridge<T>(
    send_stream: SendStream,
    recv_stream: RecvStream,
    tx: mpsc::Sender<T>,
    rx: mpsc::Receiver<T>,
) -> (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)
where
    T: Serialize + DeserializeOwned + Send + 'static,
{
    let sender = StreamSender::new(send_stream);
    let receiver = StreamReceiver::new(recv_stream);

    let recv_handle = spawn_recv_to_channel(receiver, tx);
    let send_handle = spawn_channel_to_send(sender, rx);

    (recv_handle, send_handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
    struct TestMessage {
        id: u32,
        data: String,
    }

    #[test]
    fn test_stream_error_display() {
        let err = StreamError::MessageTooLarge {
            size: 100,
            max: 50,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_stream_error_conversion() {
        let stream_err = StreamError::StreamClosed;
        let quic_err: QuicError = stream_err.into();
        assert!(matches!(quic_err, QuicError::StreamError(_)));
    }

    // Integration tests with actual QUIC streams are in tests/quic_connection.rs
}
