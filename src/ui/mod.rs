//! UI module for RemoteDesk
//!
//! This module handles the system tray icon, dialogs, and user interface.

pub mod app;
pub mod dialogs;
pub mod overlay;
pub mod tray;
pub mod viewer;

pub use app::{App, AppCommand};
pub use overlay::{OverlayConfig, OverlayPosition, StatusOverlay};
pub use tray::TrayIcon;
pub use viewer::{ViewerConfig, ViewerStats, ViewerWindow};
