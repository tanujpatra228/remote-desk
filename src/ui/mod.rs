//! UI module for RemoteDesk
//!
//! This module handles the system tray icon, dialogs, and user interface.

pub mod app;
pub mod dialogs;
pub mod tray;

pub use app::{App, AppCommand};
pub use tray::TrayIcon;
