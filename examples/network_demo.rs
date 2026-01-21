//! Network Demo for RemoteDesk
//!
//! This example demonstrates real P2P networking using QUIC:
//!
//! # Host Mode (shares screen)
//! ```bash
//! cargo run --example network_demo -- --host
//! ```
//!
//! # Client Mode (connects to host)
//! ```bash
//! cargo run --example network_demo -- --connect <DEVICE_ID>
//! ```
//!
//! # Manual Connection (specify IP directly)
//! ```bash
//! cargo run --example network_demo -- --connect <DEVICE_ID> --addr 192.168.1.100:7070
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use remote_desk::desktop::{CaptureConfig, FrameEncoder, FrameFormat, ScreenCapturer};
use remote_desk::network::{
    ConnectionEvent, ConnectionManager, ConnectionRole, EstablishedConnection, ManagerConfig,
    PeerInfo, DEFAULT_QUIC_PORT,
};
use remote_desk::security::DeviceId;
use remote_desk::session::{
    create_quic_transport, ControlMessage, SessionTransport, TransportFrame,
};

use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, info, warn};

/// Command line arguments
struct Args {
    /// Run as host (share screen)
    host: bool,
    /// Device ID to connect to
    connect_to: Option<String>,
    /// Direct address to connect to (skips mDNS)
    addr: Option<SocketAddr>,
    /// Custom port
    port: u16,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut result = Args {
        host: false,
        connect_to: None,
        addr: None,
        port: DEFAULT_QUIC_PORT,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host" | "-h" => {
                result.host = true;
            }
            "--connect" | "-c" => {
                i += 1;
                if i < args.len() {
                    result.connect_to = Some(args[i].clone());
                }
            }
            "--addr" | "-a" => {
                i += 1;
                if i < args.len() {
                    result.addr = args[i].parse().ok();
                }
            }
            "--port" | "-p" => {
                i += 1;
                if i < args.len() {
                    result.port = args[i].parse().unwrap_or(DEFAULT_QUIC_PORT);
                }
            }
            _ => {}
        }
        i += 1;
    }

    result
}

fn print_usage() {
    println!("RemoteDesk Network Demo");
    println!();
    println!("Usage:");
    println!("  Host mode (share screen):");
    println!("    cargo run --example network_demo -- --host");
    println!();
    println!("  Client mode (connect to host):");
    println!("    cargo run --example network_demo -- --connect <DEVICE_ID>");
    println!();
    println!("  With direct IP address:");
    println!("    cargo run --example network_demo -- --connect <DEVICE_ID> --addr <IP:PORT>");
    println!();
    println!("Options:");
    println!("  --host, -h          Run as host (share screen)");
    println!("  --connect, -c ID    Connect to device with given ID");
    println!("  --addr, -a IP:PORT  Direct address (skip mDNS discovery)");
    println!("  --port, -p PORT     Port to use (default: 7070)");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("remote_desk=info".parse()?)
                .add_directive("network_demo=debug".parse()?),
        )
        .init();

    let args = parse_args();

    if !args.host && args.connect_to.is_none() {
        print_usage();
        return Ok(());
    }

    // Get or create device ID
    let config_dir = get_config_dir();
    let device_id = get_or_create_device_id(&config_dir)?;

    info!(
        "Local device ID: {}",
        device_id.format_with_spaces()
    );

    // Create connection manager
    // Client mode uses port 0 (auto-assign) to avoid conflicts with host
    let port = if args.host { args.port } else { 0 };
    let manager_config = ManagerConfig::new(
        device_id,
        format!("RemoteDesk-{}", hostname::get()?.to_string_lossy()),
        config_dir,
    )
    .with_port(port);

    let mut manager = ConnectionManager::new(manager_config)?;
    manager.start().await?;

    if let Some(addr) = manager.local_addr() {
        info!("Listening on {}", addr);
    }

    if args.host {
        run_host(manager).await?;
    } else if let Some(connect_to) = args.connect_to {
        run_client(manager, &connect_to, args.addr).await?;
    }

    Ok(())
}

/// Runs the host (screen sharing) mode
async fn run_host(mut manager: ConnectionManager) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running in HOST mode - waiting for connections...");
    info!("Share this Device ID with clients: {}", manager.device_id().format_with_spaces());
    println!();
    println!("=================================================");
    println!("  Your Device ID: {}", manager.device_id().format_with_spaces());
    println!("=================================================");
    println!();
    println!("Waiting for connections... (Ctrl+C to quit)");
    println!();

    // Print discovered peers
    let peers = manager.get_discovered_peers().await;
    if !peers.is_empty() {
        println!("Discovered peers on network:");
        for peer in peers {
            println!(
                "  {} ({}) at {:?}",
                peer.device_id.format_with_spaces(),
                peer.device_name,
                peer.primary_address()
            );
        }
        println!();
    }

    // Wait for connection events
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
            event = async {
                loop {
                    if let Some(event) = manager.try_recv_event().await {
                        return event;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            } => {
                match event {
                    ConnectionEvent::ConnectionRequest { remote_id, remote_name, connection_id, .. } => {
                        println!();
                        println!("Connection request from {} ({})", remote_id.format_with_spaces(), remote_name);
                        println!("Accept? [y/n]: ");

                        // Auto-accept for demo
                        info!("Auto-accepting connection for demo...");

                        match manager.accept_connection(connection_id).await {
                            Ok(conn) => {
                                info!("Connection accepted!");
                                if let Err(e) = run_host_session(conn).await {
                                    error!("Host session error: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    ConnectionEvent::PeerDiscovered { peer_info } => {
                        info!(
                            "Discovered peer: {} ({})",
                            peer_info.device_id.format_with_spaces(),
                            peer_info.device_name
                        );
                    }
                    ConnectionEvent::PeerLost { device_id } => {
                        info!("Peer lost: {}", device_id.format_with_spaces());
                    }
                    _ => {}
                }
            }
        }
    }

    manager.stop().await;
    Ok(())
}

/// Runs a host session after connection is established
async fn run_host_session(conn: EstablishedConnection) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Starting host session with {} ({})",
        conn.remote_device_id.format_with_spaces(),
        conn.remote_name
    );

    // Create QUIC transport
    let (transport, handle) = create_quic_transport(conn.connection, conn.role, conn.control_stream).await?;

    // Start screen capture
    info!("Starting screen capture...");

    let capturer = ScreenCapturer::new(CaptureConfig::default())?;
    let display_info = capturer.display_info();
    info!(
        "Capturing display: {}x{}",
        display_info.width, display_info.height
    );

    let encoder = FrameEncoder::new(FrameFormat::Jpeg, 80);

    // Capture and send frames using spawn_blocking (capturer is not Send)
    let frame_tx = transport.frames.tx.clone();
    let (encoded_tx, mut encoded_rx) = tokio::sync::mpsc::channel::<TransportFrame>(10);

    // Capture thread - runs blocking capture and encode
    let capture_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut sequence = 0u64;
        let start = std::time::Instant::now();

        loop {
            // Use blocking capture (the async version internally)
            match rt.block_on(capturer.capture_frame()) {
                Ok(frame) => {
                    // Encode frame
                    match encoder.encode(&frame) {
                        Ok(encoded) => {
                            let transport_frame = TransportFrame::new(
                                sequence,
                                encoded.width,
                                encoded.height,
                                encoded.format,
                                encoded.data,
                                encoded.original_size,
                                start.elapsed().as_millis() as u64,
                            );

                            if rt.block_on(encoded_tx.send(transport_frame)).is_err() {
                                info!("Frame channel closed");
                                break;
                            }

                            sequence += 1;

                            if sequence % 30 == 0 {
                                info!("Sent {} frames", sequence);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to encode frame: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to capture frame: {}", e);
                }
            }

            // Target 30 FPS
            std::thread::sleep(Duration::from_millis(33));
        }
    });

    // Forward frames from capture thread to transport
    let capture_task = tokio::spawn(async move {
        while let Some(frame) = encoded_rx.recv().await {
            if frame_tx.send(frame).await.is_err() {
                info!("Transport channel closed");
                break;
            }
        }
    });

    // Handle input events from client
    let mut input_rx = transport.input.rx;
    let input_task = tokio::spawn(async move {
        while let Some(input) = input_rx.recv().await {
            info!("Received input event: {:?}", input.event);
            // TODO: Simulate input event
        }
    });

    // Wait for Ctrl+C or tasks to complete
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Stopping host session...");
        }
        _ = capture_task => {
            info!("Capture task ended");
        }
        _ = input_task => {
            info!("Input task ended");
        }
    }

    handle.abort();
    Ok(())
}

/// Runs the client (viewer) mode
async fn run_client(
    mut manager: ConnectionManager,
    device_id_str: &str,
    direct_addr: Option<SocketAddr>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse device ID (remove spaces and dashes)
    let clean_id: String = device_id_str
        .chars()
        .filter(|c| c.is_digit(10))
        .collect();

    let device_id = DeviceId::from_str(&clean_id)?;

    info!(
        "Running in CLIENT mode - connecting to {}",
        device_id.format_with_spaces()
    );

    // If direct address provided, add it as a peer
    if let Some(addr) = direct_addr {
        info!("Using direct address: {}", addr);
        manager.add_peer(device_id, "Direct".to_string(), addr).await;
    } else {
        // Wait a bit for mDNS discovery
        info!("Waiting for peer discovery...");
        tokio::time::sleep(Duration::from_secs(2)).await;

        let peers = manager.get_discovered_peers().await;
        if peers.is_empty() {
            warn!("No peers discovered. Use --addr to specify direct address.");
        } else {
            println!("Discovered peers:");
            for peer in &peers {
                println!(
                    "  {} ({}) at {:?}",
                    peer.device_id.format_with_spaces(),
                    peer.device_name,
                    peer.primary_address()
                );
            }
        }
    }

    // Connect to host
    info!("Connecting to {}...", device_id.format_with_spaces());

    match manager.connect(device_id, None).await {
        Ok(conn) => {
            info!(
                "Connected to {} ({})",
                conn.remote_device_id.format_with_spaces(),
                conn.remote_name
            );

            if let Err(e) = run_client_session(conn).await {
                error!("Client session error: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to connect: {}", e);
            eprintln!();
            eprintln!("ERROR: {}", e);
            eprintln!();
            eprintln!("Connection failed. Make sure:");
            eprintln!("  1. The host is running");
            eprintln!("  2. Both devices are on the same network");
            eprintln!("  3. Port {} is not blocked by firewall", DEFAULT_QUIC_PORT);
            eprintln!();
            eprintln!("TIP: For same-machine testing, use --addr 127.0.0.1:{}", DEFAULT_QUIC_PORT);
        }
    }

    manager.stop().await;
    Ok(())
}

/// Runs a client session after connection is established
async fn run_client_session(conn: EstablishedConnection) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Starting client session with {} ({})",
        conn.remote_device_id.format_with_spaces(),
        conn.remote_name
    );

    // Create QUIC transport
    let (transport, handle) = create_quic_transport(conn.connection, conn.role, conn.control_stream).await?;

    // Receive and display frames
    let mut frame_rx = transport.frames.rx;
    let mut frame_count = 0u64;
    let start = std::time::Instant::now();

    let frame_task = tokio::spawn(async move {
        while let Some(frame) = frame_rx.recv().await {
            frame_count += 1;

            if frame_count % 30 == 0 {
                let elapsed = start.elapsed().as_secs_f64();
                let fps = frame_count as f64 / elapsed;
                info!(
                    "Received {} frames, {:.1} FPS, frame size: {} KB",
                    frame_count,
                    fps,
                    frame.data.len() / 1024
                );
            }
        }
        info!("Frame receiver ended");
    });

    // Wait for Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Stopping client session...");
        }
        _ = frame_task => {
            info!("Frame task ended");
        }
    }

    handle.abort();
    Ok(())
}

/// Gets the configuration directory
fn get_config_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "RemoteDesk")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Gets or creates a device ID
fn get_or_create_device_id(config_dir: &PathBuf) -> Result<DeviceId, Box<dyn std::error::Error>> {
    let id_file = config_dir.join("device_id");

    if id_file.exists() {
        let id_str = std::fs::read_to_string(&id_file)?;
        let id: u32 = id_str.trim().parse()?;
        Ok(DeviceId::from_u32(id)?)
    } else {
        std::fs::create_dir_all(config_dir)?;
        let device_id = DeviceId::generate();
        std::fs::write(&id_file, device_id.as_u32().to_string())?;
        Ok(device_id)
    }
}
