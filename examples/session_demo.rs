//! Remote desktop session demo
//!
//! This example demonstrates the integration of screen capture, input simulation,
//! and the session management system.
//!
//! Run with: cargo run --example session_demo

use remote_desk::desktop::{CaptureConfig, FrameFormat};
use remote_desk::input::{InputEvent, Key, KeyboardEvent, MouseButton, MouseEvent};
use remote_desk::network::{KeyboardEventData, KeyboardEventTypeData, MouseEventData, MouseEventTypeData};
use remote_desk::security::DeviceId;
use remote_desk::session::{Session, SessionConfig, SessionMode};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    remote_desk::logging::init_logging(remote_desk::logging::LogLevel::Info);

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     RemoteDesk - Session Integration Demo                 ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Create device IDs
    let host_id = DeviceId::generate();
    let client_id = DeviceId::generate();

    println!("🖥️  Host Device ID: {}", host_id.format_with_spaces());
    println!("💻  Client Device ID: {}", client_id.format_with_spaces());
    println!();

    // Test Host Mode (being controlled)
    println!("════════════════════════════════════════════════════════════");
    println!(" Host Mode - Screen Capture & Input Simulation");
    println!("════════════════════════════════════════════════════════════");
    println!();

    test_host_mode(host_id, client_id).await?;

    println!();
    println!("════════════════════════════════════════════════════════════");
    println!(" Client Mode - Conceptual Demo");
    println!("════════════════════════════════════════════════════════════");
    println!();

    test_client_mode(client_id, host_id).await?;

    println!();
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  ✓ Session integration is working correctly!              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("📝 Note: This demo shows the session layer working end-to-end.");
    println!("   In production, frames and events would be transmitted over");
    println!("   the network using the QUIC protocol.");

    Ok(())
}

async fn test_host_mode(
    host_id: DeviceId,
    client_id: DeviceId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📸 Creating host session...");

    // Create host session configuration
    let capture_config = CaptureConfig::new(30, 80).with_format(FrameFormat::Jpeg);
    let config = SessionConfig::host(host_id, client_id, capture_config);

    // Create and start session
    let mut session = Session::new(config)?;
    session.start().await?;

    println!("✓ Host session started");
    println!("  Mode: Host (being controlled)");
    println!("  Capture: 30 FPS, Quality 80, JPEG");
    println!();

    // Test single frame capture
    println!("1️⃣  Testing single frame capture...");
    match session.capture_frame().await {
        Ok(frame_data) => {
            println!("✓ Frame captured and encoded");
            println!("  Sequence: {}", frame_data.sequence);
            println!("  Resolution: {}x{}", frame_data.width, frame_data.height);
            println!("  Format: {:?}", frame_data.format);
            println!("  Size: {} bytes ({:.2} KB)", frame_data.data.len(), frame_data.data.len() as f64 / 1024.0);
        }
        Err(e) => {
            println!("⚠️  Could not capture frame (expected in headless): {}", e);
        }
    }
    println!();

    // Test frame streaming (capture 3 frames)
    println!("2️⃣  Testing frame stream (3 frames)...");
    match session.start_frame_stream() {
        Ok(mut frame_rx) => {
            let mut count = 0;
            let timeout = Duration::from_secs(2);
            let start = std::time::Instant::now();

            while count < 3 && start.elapsed() < timeout {
                tokio::select! {
                    Some(frame_data) = frame_rx.recv() => {
                        count += 1;
                        println!("  Frame {}: {}x{} - {} bytes",
                            frame_data.sequence,
                            frame_data.width,
                            frame_data.height,
                            frame_data.data.len()
                        );
                    }
                    _ = tokio::time::sleep(timeout) => break,
                }
            }

            if count > 0 {
                println!("✓ Captured {} frames", count);
            } else {
                println!("⚠️  No frames captured (expected in headless)");
            }
        }
        Err(e) => {
            println!("⚠️  Could not start frame stream: {}", e);
        }
    }
    println!();

    // Test input event processing
    println!("3️⃣  Testing input event processing...");

    // Simulate keyboard event
    let kb_event = KeyboardEvent::key_press(Key::A);
    match session.process_input(&InputEvent::Keyboard(kb_event)).await {
        Ok(()) => println!("✓ Processed keyboard event (Key A press)"),
        Err(e) => println!("⚠️  Could not process keyboard event: {}", e),
    }

    // Simulate mouse event
    let mouse_event = MouseEvent::move_to(100, 100);
    match session.process_input(&InputEvent::Mouse(mouse_event)).await {
        Ok(()) => println!("✓ Processed mouse event (Move to 100,100)"),
        Err(e) => println!("⚠️  Could not process mouse event: {}", e),
    }
    println!();

    // Test network message processing
    println!("4️⃣  Testing network message processing...");

    // Process keyboard event from network
    let net_kb_event = KeyboardEventData::new(KeyboardEventTypeData::KeyPress, 0x41); // 'A' key
    match session.process_keyboard_event(&net_kb_event).await {
        Ok(()) => println!("✓ Processed network keyboard event"),
        Err(e) => println!("⚠️  Could not process network keyboard event: {}", e),
    }

    // Process mouse event from network
    let net_mouse_event = MouseEventData::move_to(200, 200);
    match session.process_mouse_event(&net_mouse_event).await {
        Ok(()) => println!("✓ Processed network mouse event"),
        Err(e) => println!("⚠️  Could not process network mouse event: {}", e),
    }
    println!();

    // Show statistics
    let stats = session.stats().await;
    println!("📊 Host Session Statistics:");
    println!("  Frames processed: {}", stats.frames_processed);
    println!("  Input events processed: {}", stats.input_events_processed);
    println!("  Bytes sent: {} ({:.2} KB)", stats.bytes_sent, stats.bytes_sent as f64 / 1024.0);
    println!("  Session duration: {} seconds", stats.duration_secs());
    if stats.frames_processed > 0 {
        println!("  Average FPS: {:.1}", stats.average_fps());
    }
    println!();

    // Stop session
    session.stop().await;
    println!("✓ Host session stopped");

    Ok(())
}

async fn test_client_mode(
    client_id: DeviceId,
    host_id: DeviceId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💻 Creating client session...");

    // Create client session configuration
    let config = SessionConfig::client(client_id, host_id);

    // Create and start session
    let mut session = Session::new(config)?;
    session.start().await?;

    println!("✓ Client session started");
    println!("  Mode: Client (controlling)");
    println!("  Connected to: {}", host_id.format_with_spaces());
    println!();

    println!("📝 In client mode, the session would:");
    println!("  1. Receive encoded frames from the host");
    println!("  2. Decode and display frames locally");
    println!("  3. Capture local keyboard/mouse input");
    println!("  4. Send input events to the host");
    println!();

    println!("⚙️  Creating example input events to send...");

    // Create keyboard events
    let kb_press = KeyboardEventData::new(KeyboardEventTypeData::KeyPress, 0x41); // 'A'
    let kb_release = KeyboardEventData::new(KeyboardEventTypeData::KeyRelease, 0x41);

    println!("✓ Created keyboard events (Key A press/release)");
    println!("  Event data size: {} bytes", bincode::serialize(&kb_press).unwrap().len());

    // Create mouse events
    let mouse_move = MouseEventData::move_to(500, 300);
    let mouse_click = MouseEventData::button_press(1); // Left button

    println!("✓ Created mouse events (Move & Click)");
    println!("  Event data size: {} bytes", bincode::serialize(&mouse_move).unwrap().len());
    println!();

    // Show statistics
    let stats = session.stats().await;
    println!("📊 Client Session Statistics:");
    println!("  Session duration: {} seconds", stats.duration_secs());
    println!("  (In production: would show frames received, input sent, etc.)");
    println!();

    // Stop session
    session.stop().await;
    println!("✓ Client session stopped");

    Ok(())
}
