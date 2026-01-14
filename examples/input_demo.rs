//! Input simulation demo
//!
//! This example demonstrates the input simulation functionality.
//!
//! **WARNING**: This will actually move your mouse and press keys!
//! Make sure you're ready before running this.
//!
//! Run with: cargo run --example input_demo

use remote_desk::input::{InputSimulator, Key, KeyboardEvent, MouseButton, MouseEvent};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    remote_desk::logging::init_logging(remote_desk::logging::LogLevel::Info);

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        RemoteDesk - Input Simulation Demo                 ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("⚠️  WARNING: This will actually control your mouse and keyboard!");
    println!("   Press Ctrl+C now if you don't want to continue.");
    println!();
    println!("Starting in 3 seconds...");

    thread::sleep(Duration::from_secs(3));

    // Create input simulator
    println!("🎮 Creating input simulator...");
    let simulator = InputSimulator::new();
    println!("✓ Input simulator created");
    println!();

    // Test keyboard simulation
    println!("⌨️  Testing keyboard simulation...");
    println!("   (Nothing will be typed unless you have a text editor open)");
    println!();

    // Simulate pressing and releasing a key
    let event = KeyboardEvent::key_press(Key::A);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Simulated key press: A"),
        Err(e) => println!("✗ Failed to simulate key press: {}", e),
    }

    thread::sleep(Duration::from_millis(100));

    let event = KeyboardEvent::key_release(Key::A);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Simulated key release: A"),
        Err(e) => println!("✗ Failed to simulate key release: {}", e),
    }

    thread::sleep(Duration::from_millis(500));

    // Type a string (if text editor is open)
    println!();
    println!("⌨️  Testing string typing...");
    match simulator.type_string("Hello from RemoteDesk!") {
        Ok(()) => println!("✓ Typed: 'Hello from RemoteDesk!'"),
        Err(e) => println!("✗ Failed to type string: {}", e),
    }

    thread::sleep(Duration::from_secs(1));

    // Test mouse simulation
    println!();
    println!("🖱️  Testing mouse simulation...");
    println!("   (Watch your mouse cursor move!)");
    println!();

    // Get current mouse position and move in a square pattern
    let start_x = 500;
    let start_y = 500;
    let size = 100;

    println!("Moving mouse in a square pattern...");

    // Move to start position
    let event = MouseEvent::move_to(start_x, start_y);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Moved to ({}, {})", start_x, start_y),
        Err(e) => println!("✗ Failed to move mouse: {}", e),
    }
    thread::sleep(Duration::from_millis(500));

    // Right
    let event = MouseEvent::move_to(start_x + size, start_y);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Moved right"),
        Err(e) => println!("✗ Failed: {}", e),
    }
    thread::sleep(Duration::from_millis(500));

    // Down
    let event = MouseEvent::move_to(start_x + size, start_y + size);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Moved down"),
        Err(e) => println!("✗ Failed: {}", e),
    }
    thread::sleep(Duration::from_millis(500));

    // Left
    let event = MouseEvent::move_to(start_x, start_y + size);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Moved left"),
        Err(e) => println!("✗ Failed: {}", e),
    }
    thread::sleep(Duration::from_millis(500));

    // Up (back to start)
    let event = MouseEvent::move_to(start_x, start_y);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Moved up (back to start)"),
        Err(e) => println!("✗ Failed: {}", e),
    }
    thread::sleep(Duration::from_millis(500));

    // Test mouse clicks
    println!();
    println!("Testing mouse click...");

    let event = MouseEvent::button_press(MouseButton::Left);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Left button pressed"),
        Err(e) => println!("✗ Failed: {}", e),
    }

    thread::sleep(Duration::from_millis(100));

    let event = MouseEvent::button_release(MouseButton::Left);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Left button released"),
        Err(e) => println!("✗ Failed: {}", e),
    }

    // Test mouse wheel
    println!();
    println!("Testing mouse wheel...");

    let event = MouseEvent::wheel(0, -10);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Scrolled wheel down"),
        Err(e) => println!("✗ Failed: {}", e),
    }

    thread::sleep(Duration::from_millis(500));

    let event = MouseEvent::wheel(0, 10);
    match simulator.simulate(&event.into()) {
        Ok(()) => println!("✓ Scrolled wheel up"),
        Err(e) => println!("✗ Failed: {}", e),
    }

    // Show statistics
    println!();
    println!("📊 Input Simulation Statistics:");
    println!("  Events simulated: {}", simulator.events_simulated());
    println!("  Events failed: {}", simulator.events_failed());
    println!("  Success rate: {:.1}%", simulator.success_rate() * 100.0);
    println!();

    if simulator.success_rate() >= 0.95 {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║  ✓ Input simulation is working correctly!                 ║");
        println!("╚════════════════════════════════════════════════════════════╝");
    } else {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║  ⚠️  Some input events failed. Check permissions.          ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!();
        println!("On Linux, you may need to run with sudo or configure uinput.");
        println!("On macOS, you may need to grant accessibility permissions.");
    }

    Ok(())
}
