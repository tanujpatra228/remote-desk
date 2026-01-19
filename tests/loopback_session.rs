//! Integration tests for loopback session functionality
//!
//! These tests verify the complete session management pipeline including:
//! - Transport channel communication
//! - Frame encoding/decoding roundtrip
//! - Input event processing
//! - Session state management

use std::time::Duration;

use remote_desk::desktop::{EncodedFrame, Frame, FrameDecoder, FrameEncoder, FrameFormat};
use remote_desk::input::{InputEvent, Key, KeyboardEvent, MouseButton, MouseEvent};
use remote_desk::session::{
    create_loopback_transport, ClientSessionConfig, HostSessionConfig, SessionManager,
    SessionState, TransportFrame, TransportInput,
};

/// Tests that loopback transport channels work correctly
#[tokio::test]
async fn test_loopback_frame_roundtrip() {
    // Create loopback transport
    let (host, client) = create_loopback_transport();

    // Create a test frame
    let width = 100u32;
    let height = 100u32;
    let mut frame_data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let r = (x * 255 / width) as u8;
            let g = (y * 255 / height) as u8;
            frame_data.push(r);
            frame_data.push(g);
            frame_data.push(128);
            frame_data.push(255);
        }
    }

    let frame = Frame::new(width, height, frame_data.clone(), 1);

    // Encode the frame
    let encoder = FrameEncoder::jpeg(80);
    let encoded = encoder.encode(&frame).expect("Failed to encode frame");

    // Create transport frame
    let transport_frame = TransportFrame::new(
        encoded.sequence,
        encoded.width,
        encoded.height,
        encoded.format,
        encoded.data.clone(),
        encoded.original_size,
        0,
    );

    // Send from host
    host.frames
        .tx
        .send(transport_frame.clone())
        .await
        .expect("Failed to send frame");

    // Receive on client
    let mut client_rx = client.frames.rx;
    let received = client_rx
        .recv()
        .await
        .expect("Failed to receive frame");

    assert_eq!(received.sequence, 1);
    assert_eq!(received.width, width);
    assert_eq!(received.height, height);
    assert_eq!(received.format, FrameFormat::Jpeg);

    // Decode the received frame
    let decoder = FrameDecoder::new();
    let decoded = decoder
        .decode_transport(&received)
        .expect("Failed to decode frame");

    assert_eq!(decoded.width, width);
    assert_eq!(decoded.height, height);
    // Note: JPEG is lossy, so data won't match exactly
}

/// Tests that input events can be sent through the loopback transport
#[tokio::test]
async fn test_loopback_input_roundtrip() {
    let (host, client) = create_loopback_transport();

    // Create various input events
    let events = vec![
        TransportInput::new(
            InputEvent::Keyboard(KeyboardEvent::key_press(Key::A)),
            1,
        ),
        TransportInput::new(
            InputEvent::Keyboard(KeyboardEvent::key_release(Key::A)),
            2,
        ),
        TransportInput::new(InputEvent::Mouse(MouseEvent::move_to(100, 200)), 3),
        TransportInput::new(
            InputEvent::Mouse(MouseEvent::button_press(MouseButton::Left)),
            4,
        ),
        TransportInput::new(
            InputEvent::Mouse(MouseEvent::button_release(MouseButton::Left)),
            5,
        ),
        TransportInput::new(InputEvent::Mouse(MouseEvent::wheel(0, -10)), 6),
    ];

    // Send from client
    for event in &events {
        client
            .input
            .tx
            .send(event.clone())
            .await
            .expect("Failed to send input");
    }

    // Receive on host
    let mut host_rx = host.input.rx;
    for (i, expected) in events.iter().enumerate() {
        let received = host_rx.recv().await.expect("Failed to receive input");
        assert_eq!(received.sequence, expected.sequence, "Sequence mismatch at {}", i);
    }
}

/// Tests session manager with loopback session creation
#[tokio::test]
async fn test_session_manager_loopback() {
    let manager = SessionManager::new();

    let host_config = HostSessionConfig::default()
        .with_session_id("test-host".to_string());
    let client_config = ClientSessionConfig::default()
        .with_session_id("test-client".to_string());

    // Create loopback session
    let (host_id, client_id) = manager
        .create_loopback_session(host_config, client_config)
        .await
        .expect("Failed to create loopback session");

    assert_eq!(host_id, "test-host");
    assert_eq!(client_id, "test-client");
    assert_eq!(manager.session_count().await, 2);

    // Verify session info
    let host_info = manager
        .get_session_info(&host_id)
        .await
        .expect("Host session not found");
    assert_eq!(
        host_info.session_type,
        remote_desk::session::SessionType::Host
    );

    let client_info = manager
        .get_session_info(&client_id)
        .await
        .expect("Client session not found");
    assert_eq!(
        client_info.session_type,
        remote_desk::session::SessionType::Client
    );
}

/// Tests session state transitions
#[tokio::test]
async fn test_session_state_transitions() {
    use remote_desk::session::SessionStateMachine;

    let mut sm = SessionStateMachine::new();

    // Initial state
    assert_eq!(sm.current(), SessionState::Idle);

    // Valid transitions
    sm.transition(SessionState::Connecting)
        .expect("Should transition to Connecting");
    assert_eq!(sm.current(), SessionState::Connecting);

    sm.transition(SessionState::Authenticating)
        .expect("Should transition to Authenticating");
    assert_eq!(sm.current(), SessionState::Authenticating);

    sm.transition(SessionState::Active)
        .expect("Should transition to Active");
    assert_eq!(sm.current(), SessionState::Active);
    assert!(sm.is_active());

    // Pause and resume
    sm.transition(SessionState::Paused)
        .expect("Should transition to Paused");
    assert!(!sm.is_active());

    sm.transition(SessionState::Active)
        .expect("Should transition back to Active");
    assert!(sm.is_active());

    // Disconnect
    sm.transition(SessionState::Disconnecting)
        .expect("Should transition to Disconnecting");

    sm.transition(SessionState::Disconnected)
        .expect("Should transition to Disconnected");
    assert!(sm.is_terminated());
}

/// Tests frame encoder/decoder consistency
#[tokio::test]
async fn test_frame_codec_consistency() {
    // Test different formats
    for format in [FrameFormat::Raw, FrameFormat::Jpeg, FrameFormat::Png] {
        let width = 64u32;
        let height = 64u32;
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            data.extend_from_slice(&[128, 64, 192, 255]); // RGBA
        }

        let frame = Frame::new(width, height, data.clone(), 1);
        let encoder = FrameEncoder::new(format, 90);
        let decoder = FrameDecoder::new();

        let encoded = encoder.encode(&frame).expect("Encoding failed");
        assert_eq!(encoded.format, format);
        assert_eq!(encoded.width, width);
        assert_eq!(encoded.height, height);

        // Convert to transport and back
        let transport = TransportFrame::new(
            encoded.sequence,
            encoded.width,
            encoded.height,
            encoded.format,
            encoded.data,
            encoded.original_size,
            0,
        );

        let decoded = decoder
            .decode_transport(&transport)
            .expect("Decoding failed");
        assert_eq!(decoded.width, width);
        assert_eq!(decoded.height, height);

        // For raw format, data should match exactly
        if format == FrameFormat::Raw {
            assert_eq!(decoded.data, data);
        }
    }
}

/// Tests transport statistics tracking
#[tokio::test]
async fn test_transport_stats() {
    use remote_desk::session::TransportStats;

    let mut stats = TransportStats::default();
    stats.started_at = Some(std::time::Instant::now());
    stats.messages_sent = 100;
    stats.messages_received = 95;
    stats.bytes_sent = 1_000_000;
    stats.bytes_received = 500_000;

    stats.update_latency(20);
    assert_eq!(stats.latency_ms, Some(10)); // RTT / 2

    // Wait a bit for duration
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(stats.duration_secs() > 0.0);
}

/// Tests decoder statistics
#[tokio::test]
async fn test_decoder_stats() {
    let decoder = FrameDecoder::new();

    // Decode some frames
    for i in 1..=5 {
        let width = 32u32;
        let height = 32u32;
        let data = vec![128u8; (width * height * 4) as usize];

        let encoded = EncodedFrame {
            width,
            height,
            data,
            sequence: i,
            format: FrameFormat::Raw,
            original_size: (width * height * 4) as usize,
        };

        decoder.decode(&encoded).expect("Decode failed");
    }

    let stats = decoder.stats();
    assert_eq!(stats.frames_decoded, 5);
    assert_eq!(stats.frames_dropped, 0);
    assert_eq!(stats.last_sequence, 5);
    assert!(stats.success_rate() > 99.0);
}

/// Tests that duplicate session creation fails
#[tokio::test]
async fn test_duplicate_session_prevention() {
    let manager = SessionManager::new();

    let config = HostSessionConfig::default()
        .with_session_id("unique-session".to_string());

    let (transport1, _) = create_loopback_transport();
    manager
        .create_host_session(config.clone(), transport1)
        .await
        .expect("First session should succeed");

    let (transport2, _) = create_loopback_transport();
    let result = manager.create_host_session(config, transport2).await;

    assert!(
        result.is_err(),
        "Duplicate session creation should fail"
    );
}

/// Tests session cleanup
#[tokio::test]
async fn test_session_cleanup() {
    let manager = SessionManager::new();

    // Create multiple sessions
    for i in 0..5 {
        let config = HostSessionConfig::default()
            .with_session_id(format!("session-{}", i));
        let (transport, _) = create_loopback_transport();
        manager.create_host_session(config, transport).await.unwrap();
    }

    assert_eq!(manager.session_count().await, 5);

    // Remove one
    manager.remove_session("session-2").await.unwrap();
    assert_eq!(manager.session_count().await, 4);

    // Stop all
    manager.stop_all_sessions().await.unwrap();
}
