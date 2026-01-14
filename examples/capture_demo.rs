//! Screen capture demo
//!
//! This example demonstrates the desktop capture functionality.
//!
//! Run with: cargo run --example capture_demo

use remote_desk::desktop::{CaptureConfig, FrameEncoder, FrameFormat, ScreenCapturer};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    remote_desk::logging::init_logging(remote_desk::logging::LogLevel::Info);

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        RemoteDesk - Screen Capture Demo                   ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // List available displays
    println!("📺 Available displays:");
    match ScreenCapturer::list_displays() {
        Ok(displays) => {
            for display in &displays {
                println!(
                    "  Display {}: {}x{} {}",
                    display.id,
                    display.width,
                    display.height,
                    if display.is_primary { "(primary)" } else { "" }
                );
            }
            println!();
        }
        Err(e) => {
            eprintln!("❌ Could not enumerate displays: {}", e);
            eprintln!("   This may be expected in headless environments.");
            return Ok(());
        }
    }

    // Create capture configuration
    let config = CaptureConfig::new(30, 80).with_format(FrameFormat::Jpeg);

    println!("⚙️  Capture configuration:");
    println!("  FPS: {}", config.fps);
    println!("  Quality: {}", config.quality);
    println!("  Format: {:?}", config.format);
    println!();

    // Create capturer
    println!("📸 Creating screen capturer...");
    let capturer = match ScreenCapturer::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to create capturer: {}", e);
            eprintln!("   This may be expected in headless environments.");
            return Ok(());
        }
    };

    let display_info = capturer.display_info();
    println!(
        "✓ Capturing from: {} ({}x{})",
        display_info.name, display_info.width, display_info.height
    );
    println!();

    // Capture a single frame
    println!("🎬 Capturing single frame...");
    let start = Instant::now();

    match capturer.capture_frame().await {
        Ok(frame) => {
            let capture_time = start.elapsed();

            println!("✓ Frame captured successfully!");
            println!("  Sequence: {}", frame.sequence);
            println!("  Size: {}x{}", frame.width, frame.height);
            println!("  Data size: {} bytes ({:.2} MB)", frame.size_bytes(), frame.size_bytes() as f64 / 1_048_576.0);
            println!("  Capture time: {:.2}ms", capture_time.as_millis());
            println!();

            // Encode the frame
            println!("🔧 Encoding frame...");
            let encoder = FrameEncoder::jpeg(80);
            let encode_start = Instant::now();

            match encoder.encode(&frame) {
                Ok(encoded) => {
                    let encode_time = encode_start.elapsed();

                    println!("✓ Frame encoded successfully!");
                    println!("  Format: {:?}", encoded.format);
                    println!("  Original size: {} bytes", encoded.original_size);
                    println!("  Encoded size: {} bytes", encoded.data.len());
                    println!("  Compression: {:.1}%", encoded.compression_percentage());
                    println!("  Encode time: {:.2}ms", encode_time.as_millis());
                    println!();

                    // Test decoding
                    println!("🔓 Decoding frame...");
                    let decode_start = Instant::now();

                    match FrameEncoder::decode(&encoded) {
                        Ok(decoded) => {
                            let decode_time = decode_start.elapsed();
                            println!("✓ Frame decoded successfully!");
                            println!("  Size: {}x{}", decoded.width, decoded.height);
                            println!("  Decode time: {:.2}ms", decode_time.as_millis());
                            println!();
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to decode frame: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to encode frame: {}", e);
                }
            }

            // Show performance summary
            println!("📊 Performance Summary:");
            println!("  Total time: {:.2}ms", start.elapsed().as_millis());
            println!("  Capture: {:.2}ms", capture_time.as_millis());
            println!("  Memory per frame: {:.2} MB (uncompressed)", frame.size_bytes() as f64 / 1_048_576.0);
            println!();

            // Continuous capture demo (5 frames)
            println!("🎥 Starting continuous capture (5 frames)...");
            let mut frame_rx = capturer.start_capture();
            let mut count = 0;
            let demo_start = Instant::now();

            while let Some(frame) = frame_rx.recv().await {
                count += 1;
                println!(
                    "  Frame {}: {}x{} - {} bytes",
                    frame.sequence,
                    frame.width,
                    frame.height,
                    frame.size_bytes()
                );

                if count >= 5 {
                    break;
                }
            }

            capturer.stop_capture();
            let demo_duration = demo_start.elapsed();

            println!();
            println!("✓ Captured {} frames in {:.2}s", count, demo_duration.as_secs_f64());
            println!("  Average FPS: {:.1}", count as f64 / demo_duration.as_secs_f64());
            println!();

            // Show statistics
            let stats = capturer.get_stats().await;
            println!("📈 Capture Statistics:");
            println!("  Frames captured: {}", stats.frames_captured);
            println!("  Frames dropped: {}", stats.frames_dropped);
            println!("  Drop rate: {:.2}%", stats.drop_rate());
            println!("  Bytes captured: {} ({:.2} MB)", stats.bytes_captured, stats.bytes_captured as f64 / 1_048_576.0);
            println!("  Avg capture time: {:.2}ms", stats.avg_capture_time_ms);
            println!("  Actual FPS: {:.1}", stats.actual_fps());
            println!();
        }
        Err(e) => {
            eprintln!("❌ Failed to capture frame: {}", e);
            eprintln!("   This may be expected in headless environments.");
            return Ok(());
        }
    }

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  ✓ Desktop capture layer is working correctly!            ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}
