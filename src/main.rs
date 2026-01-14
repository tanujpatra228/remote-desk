//! RemoteDesk - Lightweight peer-to-peer remote desktop application
//!
//! This is the main entry point for the RemoteDesk application.

use remote_desk::{
    config::ConfigManager,
    error::Result,
    logging::{init_logging, LogLevel},
    security::{DeviceIdManager, PasswordManager},
};
use tracing::{error, info};

/// Application state
struct App {
    config_manager: ConfigManager,
    device_id: remote_desk::security::DeviceId,
}

impl App {
    /// Initializes the application
    ///
    /// # Errors
    ///
    /// Returns error if initialization fails
    fn initialize() -> Result<Self> {
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

        Ok(Self {
            config_manager,
            device_id,
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
        info!("  Press Ctrl+C to exit");
        info!("");

        // TODO: Start network services
        // TODO: Start UI (system tray)
        // TODO: Handle incoming connections

        // For now, just wait
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");

        info!("Shutting down RemoteDesk...");

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
    match App::initialize() {
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
