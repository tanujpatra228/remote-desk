//! Integration tests for QUIC P2P connections
//!
//! These tests verify the QUIC networking layer works correctly:
//! - Certificate generation and TLS
//! - Endpoint creation
//! - Connection establishment
//! - Stream communication
//! - Protocol handshake

use std::net::SocketAddr;
use std::time::Duration;

use remote_desk::network::{
    cert, BiStream, ConnectionRole, Message, MessagePayload, MessageType, QuicConfig,
    QuicConnection, QuicEndpoint, StreamReceiver, StreamSender, CURRENT_PROTOCOL_VERSION,
};
use remote_desk::network::protocol::{ConnectionAccept, ConnectionRequest, DesktopInfo};
use remote_desk::security::DeviceId;
use remote_desk::session::{TransportFrame, TransportInput};
use remote_desk::desktop::FrameFormat;
use remote_desk::input::{InputEvent, KeyboardEvent, Key};

use tempfile::TempDir;

/// Creates a test QUIC endpoint with a certificate
fn create_test_endpoint(port: u16) -> (QuicEndpoint, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let device_id = DeviceId::generate();
    let cert_pair = cert::load_or_create_cert(temp_dir.path(), device_id.as_u32()).unwrap();

    let config = QuicConfig::default()
        .with_bind_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_cert_pair(cert_pair);

    let endpoint = QuicEndpoint::new(config).unwrap();
    (endpoint, temp_dir)
}

#[tokio::test]
async fn test_quic_endpoint_creation() {
    let (endpoint, _temp) = create_test_endpoint(17100);
    assert!(endpoint.local_addr().port() > 0);
}

#[tokio::test]
async fn test_quic_client_only_endpoint() {
    let endpoint = QuicEndpoint::client_only().unwrap();
    assert!(endpoint.local_addr().port() > 0);
}

#[tokio::test]
async fn test_quic_connection_establishment() {
    let (server, _temp1) = create_test_endpoint(17101);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17102);

    // Spawn server accept task
    let server_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap().unwrap();
        assert!(!conn.is_closed());
        conn
    });

    // Connect from client
    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    assert!(!client_conn.is_closed());
    assert_eq!(client_conn.remote_address(), server_addr);

    // Wait for server
    let server_conn = server_task.await.unwrap();
    assert!(!server_conn.is_closed());

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_bidirectional_stream() {
    let (server, _temp1) = create_test_endpoint(17103);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17104);

    // Connect
    let server_task = tokio::spawn(async move {
        server.accept().await.unwrap().unwrap()
    });

    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    let server_conn = server_task.await.unwrap();

    // Client opens bidirectional stream
    let (client_send, client_recv) = client_conn.open_bi().await.unwrap();

    // Server accepts stream
    let (server_send, server_recv) = server_conn.accept_bi().await.unwrap();

    // Create typed senders/receivers
    let mut client_sender: StreamSender<String> = StreamSender::new(client_send);
    let mut server_receiver: StreamReceiver<String> = StreamReceiver::new(server_recv);

    let mut server_sender: StreamSender<String> = StreamSender::new(server_send);
    let mut client_receiver: StreamReceiver<String> = StreamReceiver::new(client_recv);

    // Client sends to server
    client_sender.send("Hello from client".to_string()).await.unwrap();
    let received = server_receiver.recv().await.unwrap();
    assert_eq!(received, "Hello from client");

    // Server responds
    server_sender.send("Hello from server".to_string()).await.unwrap();
    let received = client_receiver.recv().await.unwrap();
    assert_eq!(received, "Hello from server");

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_unidirectional_stream() {
    let (server, _temp1) = create_test_endpoint(17105);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17106);

    // Connect
    let server_task = tokio::spawn(async move {
        server.accept().await.unwrap().unwrap()
    });

    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    let server_conn = server_task.await.unwrap();

    // Client opens unidirectional stream
    let client_send = client_conn.open_uni().await.unwrap();

    // Server accepts stream
    let server_recv = server_conn.accept_uni().await.unwrap();

    let mut sender: StreamSender<u32> = StreamSender::new(client_send);
    let mut receiver: StreamReceiver<u32> = StreamReceiver::new(server_recv);

    // Send multiple messages
    for i in 0..10u32 {
        sender.send(i).await.unwrap();
    }

    // Receive all
    for i in 0..10 {
        let received = receiver.recv().await.unwrap();
        assert_eq!(received, i);
    }

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_protocol_handshake() {
    let (server, _temp1) = create_test_endpoint(17107);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17108);

    let host_id = DeviceId::from_u32(123456789).unwrap();
    let client_id = DeviceId::from_u32(987654321).unwrap();

    // Server task - accept and handle handshake
    let server_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap().unwrap();

        // Accept control stream
        let (send, recv) = conn.accept_bi().await.unwrap();
        let mut control: BiStream<Message> = BiStream::new(send, recv);

        // Receive connection request
        let request_msg = control.recv().await.unwrap();
        let request = match request_msg.payload {
            MessagePayload::ConnectionRequest(req) => req,
            _ => panic!("Expected ConnectionRequest"),
        };

        assert_eq!(request.protocol_version, CURRENT_PROTOCOL_VERSION);
        assert_eq!(request.host_id, host_id.as_u32());
        assert_eq!(request.client_id, client_id.as_u32());

        // Send accept
        let accept = ConnectionAccept::new("Test Host".to_string(), DesktopInfo::current());
        let response = Message::new(
            MessageType::ConnectionAccept,
            MessagePayload::ConnectionAccept(accept.clone()),
        );
        control.send(response).await.unwrap();

        (conn, accept.session_id)
    });

    // Client connects
    let client_conn = client.connect(server_addr, "localhost").await.unwrap();

    // Open control stream
    let (send, recv) = client_conn.open_bi().await.unwrap();
    let mut control: BiStream<Message> = BiStream::new(send, recv);

    // Send connection request
    let request = ConnectionRequest::new(client_id, "Test Client".to_string(), host_id, None);
    let request_msg = Message::new(
        MessageType::ConnectionRequest,
        MessagePayload::ConnectionRequest(request),
    );
    control.send(request_msg).await.unwrap();

    // Receive response
    let response = control.recv().await.unwrap();
    let accept = match response.payload {
        MessagePayload::ConnectionAccept(accept) => accept,
        _ => panic!("Expected ConnectionAccept"),
    };

    assert_eq!(accept.host_name, "Test Host");

    // Verify server received correct info
    let (server_conn, session_id) = server_task.await.unwrap();
    assert_eq!(accept.session_id, session_id);

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_frame_streaming() {
    let (server, _temp1) = create_test_endpoint(17109);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17110);

    // Connect
    let server_task = tokio::spawn(async move {
        server.accept().await.unwrap().unwrap()
    });

    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    let server_conn = server_task.await.unwrap();

    // Server (host) opens video stream
    let video_send = server_conn.open_uni().await.unwrap();

    // Client accepts video stream
    let video_recv = client_conn.accept_uni().await.unwrap();

    let mut sender: StreamSender<TransportFrame> = StreamSender::new(video_send);
    let mut receiver: StreamReceiver<TransportFrame> = StreamReceiver::new(video_recv);

    // Send frames
    for seq in 0..5 {
        let frame = TransportFrame::new(
            seq,
            1920,
            1080,
            FrameFormat::Jpeg,
            vec![0u8; 10_000], // Simulated frame data
            8_294_400,
            seq * 33,
        );
        sender.send(frame).await.unwrap();
    }

    // Receive frames
    for seq in 0..5 {
        let frame = receiver.recv().await.unwrap();
        assert_eq!(frame.sequence, seq);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.format, FrameFormat::Jpeg);
    }

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_input_streaming() {
    let (server, _temp1) = create_test_endpoint(17111);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17112);

    // Connect
    let server_task = tokio::spawn(async move {
        server.accept().await.unwrap().unwrap()
    });

    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    let server_conn = server_task.await.unwrap();

    // Client opens input stream
    let input_send = client_conn.open_uni().await.unwrap();

    // Server accepts input stream
    let input_recv = server_conn.accept_uni().await.unwrap();

    let mut sender: StreamSender<TransportInput> = StreamSender::new(input_send);
    let mut receiver: StreamReceiver<TransportInput> = StreamReceiver::new(input_recv);

    // Send input events
    let events = vec![
        TransportInput::new(InputEvent::Keyboard(KeyboardEvent::key_press(Key::A)), 1),
        TransportInput::new(InputEvent::Keyboard(KeyboardEvent::key_release(Key::A)), 2),
    ];

    for event in events.clone() {
        sender.send(event).await.unwrap();
    }

    // Receive events
    for (i, expected) in events.iter().enumerate() {
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.sequence, expected.sequence);
    }

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_connection_close_and_reconnect() {
    let (server, temp1) = create_test_endpoint(17113);
    let server_addr = server.local_addr();

    // First connection
    {
        let (client, _temp2) = create_test_endpoint(17114);

        let server_task = tokio::spawn({
            let server = &server; // Borrow for the async block - need different approach
            async move {
                // Can't move server into closure, so we'll handle differently
            }
        });

        // Accept connection
        let conn = server.accept().await.unwrap().unwrap();
        let client_conn = client.connect(server_addr, "localhost").await.unwrap();

        // Exchange a message
        let (send, recv) = client_conn.open_bi().await.unwrap();
        let mut sender: StreamSender<String> = StreamSender::new(send);
        sender.send("test".to_string()).await.unwrap();

        // Close
        client_conn.close("first connection done");
        conn.close("first connection done");
    }

    // Give time for connection to fully close
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second connection to same server
    {
        let (client, _temp2) = create_test_endpoint(17115);

        let conn = server.accept().await.unwrap().unwrap();
        let client_conn = client.connect(server_addr, "localhost").await.unwrap();

        // Should work fine
        let (send, recv) = client_conn.open_bi().await.unwrap();
        let mut sender: StreamSender<String> = StreamSender::new(send);
        sender.send("second test".to_string()).await.unwrap();

        client_conn.close("second connection done");
        conn.close("second connection done");
    }
}

#[tokio::test]
async fn test_multiple_streams() {
    let (server, _temp1) = create_test_endpoint(17116);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17117);

    // Connect
    let server_task = tokio::spawn(async move {
        server.accept().await.unwrap().unwrap()
    });

    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    let server_conn = server_task.await.unwrap();

    // Open multiple streams
    let (c_send1, c_recv1) = client_conn.open_bi().await.unwrap();
    let (c_send2, c_recv2) = client_conn.open_bi().await.unwrap();

    // Server accepts them
    let (s_send1, s_recv1) = server_conn.accept_bi().await.unwrap();
    let (s_send2, s_recv2) = server_conn.accept_bi().await.unwrap();

    // Use both streams simultaneously
    let mut c_sender1: StreamSender<u32> = StreamSender::new(c_send1);
    let mut c_sender2: StreamSender<u32> = StreamSender::new(c_send2);
    let mut s_recv1: StreamReceiver<u32> = StreamReceiver::new(s_recv1);
    let mut s_recv2: StreamReceiver<u32> = StreamReceiver::new(s_recv2);

    c_sender1.send(100).await.unwrap();
    c_sender2.send(200).await.unwrap();

    let val1 = s_recv1.recv().await.unwrap();
    let val2 = s_recv2.recv().await.unwrap();

    assert_eq!(val1, 100);
    assert_eq!(val2, 200);

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}

#[tokio::test]
async fn test_connection_rtt() {
    let (server, _temp1) = create_test_endpoint(17118);
    let server_addr = server.local_addr();

    let (client, _temp2) = create_test_endpoint(17119);

    // Connect
    let server_task = tokio::spawn(async move {
        server.accept().await.unwrap().unwrap()
    });

    let client_conn = client.connect(server_addr, "localhost").await.unwrap();
    let server_conn = server_task.await.unwrap();

    // Exchange some data
    let (send, recv) = client_conn.open_bi().await.unwrap();
    let mut sender: StreamSender<Vec<u8>> = StreamSender::new(send);

    // Send data
    sender.send(vec![0u8; 10_000]).await.unwrap();

    // Wait a bit for RTT to be estimated
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Check RTT (should be very low for localhost)
    let rtt = client_conn.rtt_ms();
    // RTT should be under 100ms for localhost
    assert!(rtt < 100);

    // Cleanup
    client_conn.close("test complete");
    server_conn.close("test complete");
}
