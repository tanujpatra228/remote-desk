//! Main application UI manager

use crate::security::DeviceId;
use std::sync::mpsc;

/// Commands that can be sent to/from the application
#[derive(Debug, Clone)]
pub enum AppCommand {
    /// Show the device ID window
    ShowDeviceId(DeviceId),
    /// Show connection request dialog
    ShowConnectionRequest {
        /// Remote device ID
        remote_id: DeviceId,
        /// Remote device name
        remote_name: String,
        /// Whether password is required
        requires_password: bool,
    },
    /// Show password entry dialog
    ShowPasswordDialog {
        /// Remote device ID
        remote_id: DeviceId,
    },
    /// Show settings window
    ShowSettings,
    /// Connection accepted
    ConnectionAccepted {
        /// Remote device ID
        remote_id: DeviceId,
        /// Optional password
        password: Option<String>,
    },
    /// Connection rejected
    ConnectionRejected {
        /// Remote device ID
        remote_id: DeviceId,
    },
    /// User entered password
    PasswordEntered {
        /// Remote device ID
        remote_id: DeviceId,
        /// Password
        password: String,
    },
    /// Settings updated
    SettingsUpdated,
    /// Quit application
    Quit,
}

/// Main application UI manager
pub struct App {
    command_tx: mpsc::Sender<AppCommand>,
    command_rx: mpsc::Receiver<AppCommand>,
}

impl App {
    /// Create a new application manager
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            command_tx: tx,
            command_rx: rx,
        }
    }

    /// Get a sender for app commands
    pub fn command_sender(&self) -> mpsc::Sender<AppCommand> {
        self.command_tx.clone()
    }

    /// Try to receive a command (non-blocking)
    pub fn try_recv_command(&self) -> Option<AppCommand> {
        self.command_rx.try_recv().ok()
    }

    /// Receive a command (blocking)
    pub fn recv_command(&self) -> Option<AppCommand> {
        self.command_rx.recv().ok()
    }

    /// Send a command
    pub fn send_command(&self, command: AppCommand) -> Result<(), mpsc::SendError<AppCommand>> {
        self.command_tx.send(command)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
