//! Loopback Demo - Test session management with local screen sharing
//!
//! This example demonstrates:
//! - Creating a loopback session (host + client in same process)
//! - Capturing the local screen via the host session
//! - Displaying the captured frames in a viewer window
//! - Simulating input events back to the local desktop
//!
//! Run with: cargo run --example loopback_demo
//!
//! Note: This demo requires a display and will capture your actual screen.
//! Input events from the viewer window will be simulated on your desktop.
//! Press Escape in the viewer to exit.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use remote_desk::desktop::CaptureConfig;
use remote_desk::session::{
    create_loopback_transport, ClientSessionConfig, HostSession, HostSessionConfig,
    SessionManager,
};
use remote_desk::ui::{ViewerConfig, ViewerWindow};

use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("remote_desk=info".parse().unwrap())
                .add_directive("loopback_demo=info".parse().unwrap()),
        )
        .init();

    info!("=== RemoteDesk Loopback Demo ===");
    info!("This demo captures your screen and displays it in a viewer window.");
    info!("Input in the viewer will be simulated on your desktop.");
    info!("Press Escape or close the window to exit.");
    info!("");

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Run the demo
    if let Err(e) = rt.block_on(run_loopback_demo()) {
        error!("Demo failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_loopback_demo() -> Result<(), Box<dyn std::error::Error>> {
    // Create loopback transport
    let (host_transport, client_transport) = create_loopback_transport();

    // Extract channels for the viewer
    let frame_rx = client_transport.frames.rx;
    let input_tx = client_transport.input.tx;

    // Keep the remaining transport parts
    let _client_frame_tx = client_transport.frames.tx;
    let _client_input_rx = client_transport.input.rx;
    let _client_clipboard = client_transport.clipboard;
    let _client_control = client_transport.control;

    // Configure host session
    let host_config = HostSessionConfig {
        capture: CaptureConfig::new(30, 80), // 30 FPS, 80% quality
        allow_input: true,
        session_id: "loopback-host".to_string(),
    };

    // Create and start host session
    let mut host_session = HostSession::new(host_config, host_transport);

    info!("Starting host session...");
    host_session.start().await?;
    info!("Host session started!");

    // Give the host time to start capturing
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check host stats
    let stats = host_session.stats().await;
    info!(
        "Host stats: {} frames sent, {} dropped",
        stats.frames_sent, stats.frames_dropped
    );

    // Configure viewer
    let viewer_config = ViewerConfig {
        title: "RemoteDesk Loopback Demo".to_string(),
        width: 1280,
        height: 720,
        show_overlay: true,
        capture_input: true, // Enable input capture
    };

    info!("Opening viewer window...");
    info!("(Input events will be simulated on your desktop - be careful!)");

    // Spawn host session monitoring in background
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    tokio::spawn(async move {
        while running_clone.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Run the viewer (this blocks until the window is closed)
    // Note: eframe must run on the main thread
    let viewer = ViewerWindow::new(viewer_config, frame_rx, input_tx);

    // Stop the host session when viewer exits
    running.store(false, Ordering::SeqCst);

    // Note: viewer.run() is blocking and must be called on the main thread
    // For a real application, you'd use proper thread management
    match viewer.run() {
        Ok(()) => {
            info!("Viewer closed normally");
        }
        Err(e) => {
            warn!("Viewer error: {}", e);
        }
    }

    // Stop host session
    info!("Stopping host session...");
    host_session.stop().await?;

    // Final stats
    let final_stats = host_session.stats().await;
    info!("");
    info!("=== Session Statistics ===");
    info!("Frames sent: {}", final_stats.frames_sent);
    info!("Frames dropped: {}", final_stats.frames_dropped);
    info!("Bytes sent: {}", final_stats.bytes_sent);
    info!(
        "Average FPS: {:.1}",
        final_stats.frames_sent as f64 / final_stats.duration_secs().max(1.0)
    );
    info!(
        "Average encode time: {:.2}ms",
        final_stats.avg_encode_time_ms
    );
    info!("");
    info!("Demo completed successfully!");

    Ok(())
}

/// Alternative demo using SessionManager
#[allow(dead_code)]
async fn run_with_session_manager() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SessionManager::new();

    let host_config = HostSessionConfig::default();
    let client_config = ClientSessionConfig::default();

    // Create loopback session
    let (host_id, client_id) = manager
        .create_loopback_session(host_config, client_config)
        .await?;

    info!("Created loopback session: host={}, client={}", host_id, client_id);

    // Start both sessions
    manager.start_session(&host_id).await?;
    manager.start_session(&client_id).await?;

    info!("Sessions started. Running for 10 seconds...");

    tokio::time::sleep(Duration::from_secs(10)).await;

    // Stop sessions
    manager.stop_all_sessions().await?;

    info!("Sessions stopped.");

    Ok(())
}
