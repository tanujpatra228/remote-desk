//! Input module for RemoteDesk
//!
//! This module handles input event simulation including:
//! - Keyboard event types and simulation
//! - Mouse event types and simulation
//! - Cross-platform input handling
//! - Event serialization for network transmission

pub mod simulator;
pub mod types;

// Re-export commonly used types
pub use simulator::InputSimulator;
pub use types::{
    InputEvent, Key, KeyboardEvent, KeyboardEventType, MouseButton, MouseEvent, MouseEventType,
};
