//! RemoteDesk - Lightweight peer-to-peer remote desktop application
//!
//! This is the main entry point for the RemoteDesk application.

use remote_desk::{
    config::ConfigManager,
    error::Result,
    logging::{init_logging, LogLevel},
    network::{ConnectionManager, ManagerConfig},
    security::{DeviceIdManager, PasswordManager},
};
use tracing::{error, info, warn};

/// Application state
struct App {
    config_manager: ConfigManager,
    device_id: remote_desk::security::DeviceId,
    connection_manager: ConnectionManager,
}

impl App {
    /// Initializes the application
    ///
    /// # Errors
    ///
    /// Returns error if initialization fails
    async fn initialize() -> Result<Self> {
        info!("Initializing RemoteDesk...");

        // Load or create configuration
        let config_manager = ConfigManager::new()?;
        let config = config_manager.load_or_create_default()?;

        info!("Configuration loaded from: {:?}", config_manager.config_directory());

        // Get or create device ID
        let device_id_path = config_manager.device_id_path();
        let device_id = DeviceIdManager::get_or_create(&device_id_path)?;

        info!("Device ID: {}", device_id.format_with_spaces());

        // Check if password is set
        let password_set = PasswordManager::is_password_set(&config_manager.password_hash_path());
        if password_set {
            info!("Password authentication: ENABLED");
            info!("Security mode: Password Access");
        } else {
            info!("Password authentication: DISABLED");
            info!("Security mode: Manual Accept (default)");
        }

        // Display configuration summary
        info!("Network - Listen port: {}", config.network.listen_port);
        info!("Network - mDNS discovery: {}", config.network.enable_mdns);
        info!("Network - Max connections: {}", config.network.max_connections);
        info!("Desktop - Quality: {}%", config.desktop.default_quality);
        info!("Desktop - FPS: {}", config.desktop.default_fps);
        info!("Security - Session timeout: {} minutes", config.security.session_timeout_minutes);
        info!("Clipboard - Enabled: {}", config.clipboard.enabled);

        // Create connection manager
        let manager_config = ManagerConfig {
            device_id,
            device_name: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| format!("RemoteDesk-{}", device_id.as_u32())),
            service_port: config.network.listen_port,
            password_hash_path: config_manager.password_hash_path(),
            max_connections: config.network.max_connections as usize,
        };

        let connection_manager = ConnectionManager::new(manager_config);

        // Start connection manager
        if let Err(e) = connection_manager.start().await {
            error!("Failed to start connection manager: {}", e);
            // Continue anyway for now
        }

        Ok(Self {
            config_manager,
            device_id,
            connection_manager,
        })
    }

    /// Runs the application
    ///
    /// # Errors
    ///
    /// Returns error if application encounters a fatal error
    async fn run(&self) -> Result<()> {
        info!("RemoteDesk is ready!");
        info!("");
        info!("╔═══════════════════════════════════════════════════╗");
        info!("║           RemoteDesk - Ready to Connect          ║");
        info!("╚═══════════════════════════════════════════════════╝");
        info!("");
        info!("  Your Device ID: {}", self.device_id.format_with_spaces());
        info!("");
        info!("  Share this ID with others to allow connections.");
        info!("");

        // Check if password is set
        let password_set = PasswordManager::is_password_set(
            &self.config_manager.password_hash_path()
        );

        if password_set {
            info!("  🔐 Password Access Mode: ENABLED");
            info!("     Connections with correct password will be accepted automatically.");
        } else {
            info!("  🔓 Manual Accept Mode: ENABLED");
            info!("     You will need to accept each connection manually.");
        }

        info!("");
        info!("  Commands:");
        info!("    Type 'connect <ID>' to connect to another device");
        info!("    Type 'password <new_password>' to set a password");
        info!("    Type 'remove-password' to remove password");
        info!("    Type 'help' for all commands");
        info!("    Type 'quit' or press Ctrl+C to exit");
        info!("");

        // TODO: Start network services
        // TODO: Start UI (system tray)
        // TODO: Handle incoming connections

        // Handle CLI input
        self.handle_cli_input().await?;

        info!("Shutting down RemoteDesk...");

        Ok(())
    }

    /// Handles CLI input for basic commands
    async fn handle_cli_input(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::io::stdin;

        let mut reader = BufReader::new(stdin()).lines();

        loop {
            // Use tokio::select to handle both Ctrl+C and user input
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
                line = reader.next_line() => {
                    match line {
                        Ok(Some(input)) => {
                            let input = input.trim();
                            if input.is_empty() {
                                continue;
                            }

                            if let Err(e) = self.handle_command(input).await {
                                error!("Command error: {}", e);
                            }

                            // Check if we should exit
                            if input == "quit" || input == "exit" {
                                break;
                            }
                        }
                        Ok(None) => break, // EOF
                        Err(e) => {
                            error!("Failed to read input: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handles a single command
    async fn handle_command(&self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "help" => {
                info!("");
                info!("Available commands:");
                info!("  connect <ID> [password]  - Connect to another device");
                info!("                             Example: connect 123 456 789");
                info!("                             Example: connect 123456789 mypassword");
                info!("  disconnect <ID>          - Disconnect from a device");
                info!("                             Example: disconnect 123 456 789");
                info!("  password <new_password>  - Set a password for this device");
                info!("  remove-password          - Remove the password (use manual accept)");
                info!("  id                       - Show your device ID");
                info!("  status                   - Show current status and connections");
                info!("  help                     - Show this help message");
                info!("  quit / exit              - Exit the application");
                info!("");
            }
            "connect" => {
                if parts.len() < 2 {
                    error!("Usage: connect <ID> [password]");
                    error!("Example: connect 123 456 789");
                    error!("Example: connect 123456789 mypassword");
                    return Ok(());
                }

                // Parse the ID (could be with or without spaces)
                let id_str = if parts.len() >= 4 && parts[1].parse::<u32>().is_ok()
                              && parts[2].parse::<u32>().is_ok()
                              && parts[3].parse::<u32>().is_ok() {
                    // Format: connect 123 456 789 [password]
                    format!("{}{}{}", parts[1], parts[2], parts[3])
                } else {
                    // Format: connect 123456789 [password]
                    parts[1].to_string()
                };

                // Get password if provided
                let password = if parts.len() >= 4 && parts[1].parse::<u32>().is_ok() {
                    // Format was: connect 123 456 789 password
                    if parts.len() > 4 {
                        Some(parts[4])
                    } else {
                        None
                    }
                } else if parts.len() >= 3 {
                    // Format was: connect 123456789 password
                    Some(parts[2])
                } else {
                    None
                };

                // Validate the ID
                match remote_desk::security::DeviceId::validate(&id_str) {
                    Ok(_) => {
                        let formatted_id = id_str.chars()
                            .collect::<Vec<_>>()
                            .chunks(3)
                            .map(|chunk| chunk.iter().collect::<String>())
                            .collect::<Vec<_>>()
                            .join(" ");

                        info!("");
                        info!("Connecting to device: {}", formatted_id);
                        if let Some(pwd) = password {
                            info!("Using password: {}", "*".repeat(pwd.len()));
                        } else {
                            info!("No password provided (manual accept required)");
                        }
                        info!("");

                        // Parse the device ID
                        let remote_id = match id_str.parse::<remote_desk::security::DeviceId>() {
                            Ok(id) => id,
                            Err(e) => {
                                error!("Failed to parse device ID: {}", e);
                                return Ok(());
                            }
                        };

                        // Attempt connection
                        match self.connection_manager.connect(remote_id, password.map(|s| s.to_string())).await {
                            Ok(_) => {
                                info!("✓ Connection initiated successfully!");
                                info!("  Status: Connected to {}", formatted_id);
                                info!("");
                                info!("📝 Note: This is a simulated connection for Milestone 1.2");
                                info!("   Full QUIC networking will be added in the next iteration.");
                                info!("");
                            }
                            Err(e) => {
                                error!("✗ Connection failed: {}", e);
                                info!("");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid device ID: {}", e);
                        error!("Device ID must be 9 digits (e.g., 123456789 or 123 456 789)");
                    }
                }
            }
            "password" => {
                if parts.len() < 2 {
                    error!("Usage: password <new_password>");
                    error!("Example: password MySecurePassword123");
                    return Ok(());
                }

                let new_password = parts[1..].join(" ");

                match PasswordManager::set_password(
                    &self.config_manager.password_hash_path(),
                    &new_password
                ) {
                    Ok(_) => {
                        info!("");
                        info!("✓ Password set successfully!");
                        info!("  Password Access Mode is now ENABLED");
                        info!("  Connections with this password will be accepted automatically.");
                        info!("");
                    }
                    Err(e) => {
                        error!("Failed to set password: {}", e);
                    }
                }
            }
            "remove-password" => {
                match PasswordManager::remove_password(&self.config_manager.password_hash_path()) {
                    Ok(_) => {
                        info!("");
                        info!("✓ Password removed successfully!");
                        info!("  Manual Accept Mode is now ENABLED");
                        info!("  You will need to accept each connection manually.");
                        info!("");
                    }
                    Err(e) => {
                        error!("Failed to remove password: {}", e);
                    }
                }
            }
            "id" => {
                info!("");
                info!("Your Device ID: {}", self.device_id.format_with_spaces());
                info!("");
            }
            "status" => {
                info!("");
                info!("Status:");
                info!("  Device ID: {}", self.device_id.format_with_spaces());

                let password_set = PasswordManager::is_password_set(
                    &self.config_manager.password_hash_path()
                );

                if password_set {
                    info!("  Mode: 🔐 Password Access");
                } else {
                    info!("  Mode: 🔓 Manual Accept");
                }

                // Get active connections
                let active_connections = self.connection_manager.get_active_connections().await;

                if active_connections.is_empty() {
                    info!("  Connections: None");
                } else {
                    info!("  Connections: {} active", active_connections.len());
                    for conn_info in active_connections {
                        info!(
                            "    - {} ({}) [{}]",
                            conn_info.remote_id.format_with_spaces(),
                            conn_info.remote_name,
                            conn_info.role
                        );
                    }
                }

                info!("");
            }
            "disconnect" => {
                if parts.len() < 2 {
                    error!("Usage: disconnect <ID>");
                    error!("Example: disconnect 123 456 789");
                    return Ok(());
                }

                // Parse the ID
                let id_str = if parts.len() >= 4 && parts[1].parse::<u32>().is_ok() {
                    format!("{}{}{}", parts[1], parts[2], parts[3])
                } else {
                    parts[1].to_string()
                };

                match id_str.parse::<remote_desk::security::DeviceId>() {
                    Ok(remote_id) => {
                        match self.connection_manager.disconnect(remote_id).await {
                            Ok(_) => {
                                info!("");
                                info!("✓ Disconnected from {}", remote_id.format_with_spaces());
                                info!("");
                            }
                            Err(e) => {
                                error!("Failed to disconnect: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid device ID: {}", e);
                    }
                }
            }
            "quit" | "exit" => {
                info!("Exiting...");
                // Stop connection manager
                self.connection_manager.stop().await;
            }
            _ => {
                error!("Unknown command: {}", parts[0]);
                error!("Type 'help' for available commands");
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    // Check for debug mode via environment variable
    let log_level = if std::env::var("RUST_LOG").is_ok() {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    init_logging(log_level);

    info!("Starting RemoteDesk v{}", env!("CARGO_PKG_VERSION"));

    // Initialize and run application
    match App::initialize().await {
        Ok(app) => {
            if let Err(e) = app.run().await {
                error!("Application error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Failed to initialize application: {}", e);
            std::process::exit(1);
        }
    }

    info!("RemoteDesk stopped.");
}
