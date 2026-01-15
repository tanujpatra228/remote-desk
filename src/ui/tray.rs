//! System tray icon implementation

use std::sync::mpsc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon as TrayIconBuilder, TrayIconBuilder as Builder,
};

/// Commands that can be sent from the tray icon
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayCommand {
    /// Show the device ID dialog
    ShowDeviceId,
    /// Show the connect to peer dialog
    ConnectToPeer,
    /// Show the settings dialog
    ShowSettings,
    /// Quit the application
    Quit,
}

/// System tray icon manager
pub struct TrayIcon {
    _tray: TrayIconBuilder,
    menu_rx: mpsc::Receiver<TrayCommand>,
}

impl TrayIcon {
    /// Create a new system tray icon
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create menu items
        let show_id = MenuItem::new("Show Device ID", true, None);
        let connect = MenuItem::new("Connect to Peer...", true, None);
        let separator = PredefinedMenuItem::separator();
        let settings = MenuItem::new("Settings...", true, None);
        let quit = PredefinedMenuItem::quit(Some("Quit RemoteDesk"));

        // Create menu
        let menu = Menu::new();
        menu.append(&show_id)?;
        menu.append(&connect)?;
        menu.append(&separator)?;
        menu.append(&settings)?;
        menu.append(&quit)?;

        // Create tray icon
        let tray = Builder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("RemoteDesk - Remote Desktop")
            .with_icon(Self::create_icon())
            .build()?;

        // Create channel for menu events
        let (tx, rx) = mpsc::channel();

        // Extract menu item IDs before spawning thread (MenuItem is not Send)
        let show_id_str = show_id.id().0.clone();
        let connect_id_str = connect.id().0.clone();
        let settings_id_str = settings.id().0.clone();
        let quit_id_str = quit.id().0.clone();

        // Set up menu event handler
        let menu_channel = MenuEvent::receiver();
        std::thread::spawn(move || {
            while let Ok(event) = menu_channel.recv() {
                let command = match event.id.0.as_str() {
                    id if id == show_id_str => TrayCommand::ShowDeviceId,
                    id if id == connect_id_str => TrayCommand::ConnectToPeer,
                    id if id == settings_id_str => TrayCommand::ShowSettings,
                    id if id == quit_id_str => TrayCommand::Quit,
                    _ => continue,
                };

                if tx.send(command).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            _tray: tray,
            menu_rx: rx,
        })
    }

    /// Try to receive a tray command (non-blocking)
    pub fn try_recv(&self) -> Option<TrayCommand> {
        self.menu_rx.try_recv().ok()
    }

    /// Receive a tray command (blocking)
    pub fn recv(&self) -> Option<TrayCommand> {
        self.menu_rx.recv().ok()
    }

    /// Create the tray icon image
    fn create_icon() -> tray_icon::Icon {
        // Create a simple 32x32 icon with RD letters
        let size = 32;
        let mut rgba = vec![0u8; size * size * 4];

        // Background color (dark blue)
        for i in 0..size * size {
            rgba[i * 4] = 30;     // R
            rgba[i * 4 + 1] = 60; // G
            rgba[i * 4 + 2] = 120; // B
            rgba[i * 4 + 3] = 255; // A
        }

        // Draw a simple "RD" pattern (simplified for now)
        // In production, you would load an actual icon file
        for y in 8..24 {
            for x in 8..24 {
                let i = (y * size + x) as usize;
                if (x >= 8 && x <= 10) || // R left vertical
                   (y >= 8 && y <= 10 && x <= 16) || // R top horizontal
                   (y >= 14 && y <= 16 && x <= 14) || // R middle
                   (x >= 18 && x <= 20) || // D left vertical
                   (y >= 8 && y <= 10 && x <= 24) || // D top
                   (y >= 22 && y <= 24 && x <= 24) || // D bottom
                   (x >= 22 && x <= 24 && y >= 10 && y <= 22) // D right
                {
                    rgba[i * 4] = 255;     // R
                    rgba[i * 4 + 1] = 255; // G
                    rgba[i * 4 + 2] = 255; // B
                }
            }
        }

        tray_icon::Icon::from_rgba(rgba, size as u32, size as u32)
            .expect("Failed to create tray icon")
    }
}

impl Default for TrayIcon {
    fn default() -> Self {
        Self::new().expect("Failed to create tray icon")
    }
}
