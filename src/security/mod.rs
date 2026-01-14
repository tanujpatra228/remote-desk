//! Security module for RemoteDesk
//!
//! This module contains all security-related functionality including:
//! - Device ID generation and management
//! - Password hashing and verification
//! - Authentication (to be implemented)
//! - Encryption (to be implemented)

pub mod id;
pub mod password;

// Re-export commonly used types
pub use id::{DeviceId, DeviceIdManager};
pub use password::PasswordManager;
