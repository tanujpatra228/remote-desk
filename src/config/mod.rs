//! Configuration management for RemoteDesk
//!
//! This module handles application configuration including:
//! - Loading and saving configuration files
//! - Managing configuration directory
//! - Providing sensible defaults
//! - Configuration validation

use crate::error::{ConfigError, ConfigResult};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Constants for configuration (avoiding magic numbers)
const CONFIG_FILE_NAME: &str = "config.toml";
const DEVICE_ID_FILE_NAME: &str = "device_id";
const PASSWORD_HASH_FILE_NAME: &str = "password.hash";
const CONNECTION_LOG_FILE_NAME: &str = "connections.log";

const DEFAULT_LISTEN_PORT: u16 = 0; // 0 = random port
const DEFAULT_MAX_CONNECTIONS: u8 = 1;
const DEFAULT_SESSION_TIMEOUT_MINUTES: u32 = 30;
const DEFAULT_IDLE_TIMEOUT_MINUTES: u32 = 10;
const DEFAULT_QUALITY: u8 = 80;
const DEFAULT_FPS: u8 = 30;
const DEFAULT_COMPRESSION_LEVEL: u8 = 3;
const DEFAULT_MAX_PASSWORD_ATTEMPTS: u32 = 5;
const DEFAULT_LOCKOUT_DURATION_MINUTES: u32 = 15;
const DEFAULT_CLIPBOARD_MAX_SIZE_MB: u32 = 10;
const DEFAULT_CLIPBOARD_SYNC_DELAY_MS: u64 = 500;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Network configuration
    pub network: NetworkConfig,

    /// Desktop capture configuration
    pub desktop: DesktopConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Clipboard configuration
    pub clipboard: ClipboardConfig,

    /// UI configuration
    pub ui: UiConfig,
}

/// Network-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Port to listen on (0 for random)
    pub listen_port: u16,

    /// Enable mDNS for local network discovery
    pub enable_mdns: bool,

    /// STUN servers for NAT traversal
    pub stun_servers: Vec<String>,

    /// Maximum concurrent connections
    pub max_connections: u8,
}

/// Desktop capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    /// Default quality (0-100)
    pub default_quality: u8,

    /// Default frame rate
    pub default_fps: u8,

    /// Compression level (0-22 for zstd)
    pub compression_level: u8,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Require password for connections
    pub require_password: bool,

    /// Minimum password length
    pub min_password_length: usize,

    /// Session timeout in minutes
    pub session_timeout_minutes: u32,

    /// Idle timeout in minutes
    pub idle_timeout_minutes: u32,

    /// Maximum password attempts before lockout
    pub max_password_attempts: u32,

    /// Lockout duration in minutes
    pub lockout_duration_minutes: u32,
}

/// Clipboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    /// Enable clipboard synchronization
    pub enabled: bool,

    /// Maximum clipboard size in MB
    pub max_size_mb: u32,

    /// Delay before syncing clipboard (debounce) in milliseconds
    pub sync_delay_ms: u64,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Show system tray icon
    pub show_tray_icon: bool,

    /// Minimize to tray on close
    pub minimize_to_tray: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            desktop: DesktopConfig::default(),
            security: SecurityConfig::default(),
            clipboard: ClipboardConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: DEFAULT_LISTEN_PORT,
            enable_mdns: true,
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            default_quality: DEFAULT_QUALITY,
            default_fps: DEFAULT_FPS,
            compression_level: DEFAULT_COMPRESSION_LEVEL,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_password: false, // Manual accept by default
            min_password_length: 6,
            session_timeout_minutes: DEFAULT_SESSION_TIMEOUT_MINUTES,
            idle_timeout_minutes: DEFAULT_IDLE_TIMEOUT_MINUTES,
            max_password_attempts: DEFAULT_MAX_PASSWORD_ATTEMPTS,
            lockout_duration_minutes: DEFAULT_LOCKOUT_DURATION_MINUTES,
        }
    }
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: DEFAULT_CLIPBOARD_MAX_SIZE_MB,
            sync_delay_ms: DEFAULT_CLIPBOARD_SYNC_DELAY_MS,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_tray_icon: true,
            minimize_to_tray: true,
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl ConfigManager {
    /// Creates a new ConfigManager
    ///
    /// # Errors
    ///
    /// Returns error if project directory cannot be determined
    pub fn new() -> ConfigResult<Self> {
        let config_dir = Self::get_config_directory()?;
        let config_file = config_dir.join(CONFIG_FILE_NAME);

        Ok(Self {
            config_dir,
            config_file,
        })
    }

    /// Gets the configuration directory path
    fn get_config_directory() -> ConfigResult<PathBuf> {
        ProjectDirs::from("com", "remotedesk", "RemoteDesk")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .ok_or_else(|| {
                ConfigError::DirectoryNotFound(
                    "Could not determine configuration directory".to_string(),
                )
            })
    }

    /// Ensures the configuration directory exists
    fn ensure_config_directory(&self) -> ConfigResult<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir).map_err(|e| {
                ConfigError::DirectoryCreationFailed(format!(
                    "Failed to create config directory at {:?}: {}",
                    self.config_dir, e
                ))
            })?;
        }
        Ok(())
    }

    /// Loads configuration from file, or creates default if it doesn't exist
    ///
    /// # Errors
    ///
    /// Returns error if configuration cannot be loaded or created
    pub fn load_or_create_default(&self) -> ConfigResult<Config> {
        self.ensure_config_directory()?;

        if self.config_file.exists() {
            self.load()
        } else {
            let config = Config::default();
            self.save(&config)?;
            Ok(config)
        }
    }

    /// Loads configuration from file
    fn load(&self) -> ConfigResult<Config> {
        let content = fs::read_to_string(&self.config_file).map_err(|e| {
            ConfigError::LoadFailed(format!("Failed to read config file: {}", e))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            ConfigError::LoadFailed(format!("Failed to parse config file: {}", e))
        })?;

        self.validate(&config)?;

        Ok(config)
    }

    /// Saves configuration to file
    ///
    /// # Errors
    ///
    /// Returns error if configuration cannot be saved
    pub fn save(&self, config: &Config) -> ConfigResult<()> {
        self.ensure_config_directory()?;
        self.validate(config)?;

        let content = toml::to_string_pretty(config).map_err(|e| {
            ConfigError::SaveFailed(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&self.config_file, content).map_err(|e| {
            ConfigError::SaveFailed(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// Validates configuration values
    fn validate(&self, config: &Config) -> ConfigResult<()> {
        // Validate quality (0-100)
        if config.desktop.default_quality > 100 {
            return Err(ConfigError::InvalidValue(
                "Quality must be between 0 and 100".to_string(),
            ));
        }

        // Validate FPS (1-60)
        if config.desktop.default_fps == 0 || config.desktop.default_fps > 60 {
            return Err(ConfigError::InvalidValue(
                "FPS must be between 1 and 60".to_string(),
            ));
        }

        // Validate compression level (0-22 for zstd)
        if config.desktop.compression_level > 22 {
            return Err(ConfigError::InvalidValue(
                "Compression level must be between 0 and 22".to_string(),
            ));
        }

        // Validate password length
        if config.security.min_password_length < 4 || config.security.min_password_length > 128 {
            return Err(ConfigError::InvalidValue(
                "Minimum password length must be between 4 and 128".to_string(),
            ));
        }

        Ok(())
    }

    /// Gets the path to the device ID file
    pub fn device_id_path(&self) -> PathBuf {
        self.config_dir.join(DEVICE_ID_FILE_NAME)
    }

    /// Gets the path to the password hash file
    pub fn password_hash_path(&self) -> PathBuf {
        self.config_dir.join(PASSWORD_HASH_FILE_NAME)
    }

    /// Gets the path to the connection log file
    pub fn connection_log_path(&self) -> PathBuf {
        self.config_dir.join(CONNECTION_LOG_FILE_NAME)
    }

    /// Gets the configuration directory path
    pub fn config_directory(&self) -> &PathBuf {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.network.listen_port, 0);
        assert_eq!(config.desktop.default_quality, 80);
        assert_eq!(config.security.min_password_length, 6);
        assert!(config.clipboard.enabled);
    }

    #[test]
    fn test_config_validation() {
        let manager = ConfigManager::new().unwrap();

        let mut config = Config::default();
        config.desktop.default_quality = 150; // Invalid

        assert!(manager.validate(&config).is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.network.listen_port, deserialized.network.listen_port);
    }
}
