//! Error types for RemoteDesk
//!
//! This module defines all error types used throughout the application.
//! Following clean code principles: single source of truth for errors,
//! descriptive error messages, and proper error categorization.

use std::io;
use thiserror::Error;

/// Main error type for RemoteDesk application
#[derive(Error, Debug)]
pub enum RemoteDeskError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Security and authentication errors
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Generic error with context
    #[error("{0}")]
    Generic(String),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadFailed(String),

    #[error("Failed to save configuration: {0}")]
    SaveFailed(String),

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),

    #[error("Configuration directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Failed to create configuration directory: {0}")]
    DirectoryCreationFailed(String),
}

/// Security and authentication errors
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid device ID: {0}")]
    InvalidDeviceId(String),

    #[error("Password hash verification failed")]
    PasswordVerificationFailed,

    #[error("Password is too short (minimum {min} characters)")]
    PasswordTooShort { min: usize },

    #[error("Password is too long (maximum {max} characters)")]
    PasswordTooLong { max: usize },

    #[error("Account is locked due to too many failed attempts")]
    AccountLocked,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),
}

/// Network-related errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Connection rejected: {0}")]
    ConnectionRejected(String),

    #[error("Invalid peer ID: {0}")]
    InvalidPeerId(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Invalid message format")]
    InvalidMessageFormat,

    #[error("Disconnected: {0}")]
    Disconnected(String),
}

/// Type alias for Results using RemoteDeskError
pub type Result<T> = std::result::Result<T, RemoteDeskError>;

/// Type alias for Config Results
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

/// Type alias for Security Results
pub type SecurityResult<T> = std::result::Result<T, SecurityError>;

/// Type alias for Network Results
pub type NetworkResult<T> = std::result::Result<T, NetworkError>;

impl From<bincode::Error> for RemoteDeskError {
    fn from(err: bincode::Error) -> Self {
        RemoteDeskError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for RemoteDeskError {
    fn from(err: toml::de::Error) -> Self {
        RemoteDeskError::Config(ConfigError::LoadFailed(err.to_string()))
    }
}

impl From<toml::ser::Error> for RemoteDeskError {
    fn from(err: toml::ser::Error) -> Self {
        RemoteDeskError::Config(ConfigError::SaveFailed(err.to_string()))
    }
}

impl From<SecurityError> for NetworkError {
    fn from(err: SecurityError) -> Self {
        NetworkError::ProtocolError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = SecurityError::PasswordTooShort { min: 8 };
        assert_eq!(
            error.to_string(),
            "Password is too short (minimum 8 characters)"
        );
    }

    #[test]
    fn test_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let remote_error: RemoteDeskError = io_error.into();
        assert!(matches!(remote_error, RemoteDeskError::Io(_)));
    }
}
