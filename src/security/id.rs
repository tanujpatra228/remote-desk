//! Device ID generation and management
//!
//! This module handles the 9-digit device ID system for RemoteDesk.
//! Each device gets a unique numeric ID on first launch.

use crate::error::{SecurityError, SecurityResult};
use rand::Rng;
use std::fmt;
use std::fs;
use std::path::Path;
use std::str::FromStr;

// Constants for ID generation (avoiding magic numbers)
const DEVICE_ID_MIN: u32 = 100_000_000; // 9 digits minimum
const DEVICE_ID_MAX: u32 = 999_999_999; // 9 digits maximum
const DEVICE_ID_LENGTH: usize = 9;

/// Represents a 9-digit device ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(u32);

impl DeviceId {
    /// Generates a new random device ID
    ///
    /// # Returns
    ///
    /// A random 9-digit device ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use remote_desk::security::id::DeviceId;
    ///
    /// let id = DeviceId::generate();
    /// assert!(id.as_u32() >= 100_000_000);
    /// assert!(id.as_u32() <= 999_999_999);
    /// ```
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let id = rng.gen_range(DEVICE_ID_MIN..=DEVICE_ID_MAX);
        Self(id)
    }

    /// Creates a DeviceId from a u32 value
    ///
    /// # Errors
    ///
    /// Returns error if the ID is not a valid 9-digit number
    pub fn from_u32(id: u32) -> SecurityResult<Self> {
        if id < DEVICE_ID_MIN || id > DEVICE_ID_MAX {
            return Err(SecurityError::InvalidDeviceId(format!(
                "Device ID must be a 9-digit number, got: {}",
                id
            )));
        }
        Ok(Self(id))
    }

    /// Gets the device ID as a u32
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// Formats the device ID with spaces for readability
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use remote_desk::security::id::DeviceId;
    ///
    /// let id = DeviceId::from_u32(123456789).unwrap();
    /// assert_eq!(id.format_with_spaces(), "123 456 789");
    /// ```
    pub fn format_with_spaces(&self) -> String {
        let id_str = format!("{:09}", self.0);
        format!(
            "{} {} {}",
            &id_str[0..3],
            &id_str[3..6],
            &id_str[6..9]
        )
    }

    /// Validates a device ID string (with or without spaces)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use remote_desk::security::id::DeviceId;
    ///
    /// assert!(DeviceId::validate("123456789").is_ok());
    /// assert!(DeviceId::validate("123 456 789").is_ok());
    /// assert!(DeviceId::validate("12345678").is_err()); // Too short
    /// ```
    pub fn validate(id_str: &str) -> SecurityResult<()> {
        let cleaned = id_str.replace(' ', "");

        if cleaned.len() != DEVICE_ID_LENGTH {
            return Err(SecurityError::InvalidDeviceId(format!(
                "Device ID must be {} digits, got: {}",
                DEVICE_ID_LENGTH,
                cleaned.len()
            )));
        }

        let id: u32 = cleaned.parse().map_err(|_| {
            SecurityError::InvalidDeviceId("Device ID must contain only digits".to_string())
        })?;

        Self::from_u32(id)?;
        Ok(())
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:09}", self.0)
    }
}

impl FromStr for DeviceId {
    type Err = SecurityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleaned = s.replace(' ', "");
        let id: u32 = cleaned.parse().map_err(|_| {
            SecurityError::InvalidDeviceId("Invalid device ID format".to_string())
        })?;
        Self::from_u32(id)
    }
}

/// Device ID manager for loading and saving device IDs
pub struct DeviceIdManager;

impl DeviceIdManager {
    /// Gets or creates a device ID
    ///
    /// If a device ID file exists, loads it. Otherwise, generates a new ID and saves it.
    ///
    /// # Arguments
    ///
    /// * `id_file_path` - Path to the device ID file
    ///
    /// # Returns
    ///
    /// The device ID (existing or newly generated)
    ///
    /// # Errors
    ///
    /// Returns error if the ID file cannot be read or written
    pub fn get_or_create(id_file_path: &Path) -> SecurityResult<DeviceId> {
        if id_file_path.exists() {
            Self::load(id_file_path)
        } else {
            let id = DeviceId::generate();
            Self::save(id_file_path, id)?;
            Ok(id)
        }
    }

    /// Loads a device ID from file
    fn load(id_file_path: &Path) -> SecurityResult<DeviceId> {
        let content = fs::read_to_string(id_file_path).map_err(|e| {
            SecurityError::InvalidDeviceId(format!("Failed to read device ID file: {}", e))
        })?;

        let id_str = content.trim();
        id_str.parse()
    }

    /// Saves a device ID to file
    fn save(id_file_path: &Path, id: DeviceId) -> SecurityResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = id_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SecurityError::InvalidDeviceId(format!(
                    "Failed to create directory for device ID: {}",
                    e
                ))
            })?;
        }

        fs::write(id_file_path, id.to_string()).map_err(|e| {
            SecurityError::InvalidDeviceId(format!("Failed to write device ID file: {}", e))
        })?;

        Ok(())
    }

    /// Regenerates a device ID (useful for collision resolution)
    ///
    /// # Errors
    ///
    /// Returns error if the new ID cannot be saved
    pub fn regenerate(id_file_path: &Path) -> SecurityResult<DeviceId> {
        let new_id = DeviceId::generate();
        Self::save(id_file_path, new_id)?;
        Ok(new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_device_id() {
        let id = DeviceId::generate();
        assert!(id.as_u32() >= DEVICE_ID_MIN);
        assert!(id.as_u32() <= DEVICE_ID_MAX);
    }

    #[test]
    fn test_device_id_from_u32() {
        let id = DeviceId::from_u32(123456789).unwrap();
        assert_eq!(id.as_u32(), 123456789);

        // Test invalid IDs
        assert!(DeviceId::from_u32(12345678).is_err()); // Too small
        assert!(DeviceId::from_u32(1234567890).is_err()); // Too large
    }

    #[test]
    fn test_format_with_spaces() {
        let id = DeviceId::from_u32(123456789).unwrap();
        assert_eq!(id.format_with_spaces(), "123 456 789");

        let id2 = DeviceId::from_u32(100000000).unwrap();
        assert_eq!(id2.format_with_spaces(), "100 000 000");
    }

    #[test]
    fn test_device_id_display() {
        let id = DeviceId::from_u32(123456789).unwrap();
        assert_eq!(id.to_string(), "123456789");
    }

    #[test]
    fn test_device_id_from_str() {
        // Without spaces
        let id: DeviceId = "123456789".parse().unwrap();
        assert_eq!(id.as_u32(), 123456789);

        // With spaces
        let id2: DeviceId = "123 456 789".parse().unwrap();
        assert_eq!(id2.as_u32(), 123456789);

        // Invalid formats
        assert!("12345678".parse::<DeviceId>().is_err());
        assert!("abc123456".parse::<DeviceId>().is_err());
    }

    #[test]
    fn test_device_id_validate() {
        assert!(DeviceId::validate("123456789").is_ok());
        assert!(DeviceId::validate("123 456 789").is_ok());
        assert!(DeviceId::validate("12345678").is_err());
        assert!(DeviceId::validate("1234567890").is_err());
        assert!(DeviceId::validate("abc123456").is_err());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let id_file = temp_dir.path().join("device_id");

        let original_id = DeviceId::from_u32(123456789).unwrap();
        DeviceIdManager::save(&id_file, original_id).unwrap();

        let loaded_id = DeviceIdManager::load(&id_file).unwrap();
        assert_eq!(original_id, loaded_id);
    }

    #[test]
    fn test_get_or_create() {
        let temp_dir = TempDir::new().unwrap();
        let id_file = temp_dir.path().join("device_id");

        // First call should create new ID
        let id1 = DeviceIdManager::get_or_create(&id_file).unwrap();
        assert!(id1.as_u32() >= DEVICE_ID_MIN);

        // Second call should load existing ID
        let id2 = DeviceIdManager::get_or_create(&id_file).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_regenerate() {
        let temp_dir = TempDir::new().unwrap();
        let id_file = temp_dir.path().join("device_id");

        let id1 = DeviceIdManager::get_or_create(&id_file).unwrap();
        let id2 = DeviceIdManager::regenerate(&id_file).unwrap();

        // IDs should be different (with very high probability)
        assert_ne!(id1, id2);

        // New ID should be saved
        let id3 = DeviceIdManager::get_or_create(&id_file).unwrap();
        assert_eq!(id2, id3);
    }
}
