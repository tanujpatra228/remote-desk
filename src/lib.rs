//! RemoteDesk - A lightweight peer-to-peer remote desktop application
//!
//! This library provides the core functionality for RemoteDesk, including:
//! - Configuration management
//! - Device ID generation and management
//! - Password hashing and authentication
//! - Logging infrastructure
//!
//! # Examples
//!
//! ```no_run
//! use remote_desk::{config::ConfigManager, security::DeviceIdManager, logging};
//!
//! // Initialize logging
//! logging::init_default_logging();
//!
//! // Load configuration
//! let config_manager = ConfigManager::new().unwrap();
//! let config = config_manager.load_or_create_default().unwrap();
//!
//! // Get or create device ID
//! let device_id = DeviceIdManager::get_or_create(
//!     &config_manager.device_id_path()
//! ).unwrap();
//!
//! println!("Your device ID: {}", device_id.format_with_spaces());
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod clipboard;
pub mod config;
pub mod desktop;
pub mod error;
pub mod input;
pub mod logging;
pub mod network;
pub mod security;
pub mod ui;

// Re-export commonly used types at crate root
pub use error::{RemoteDeskError, Result};
