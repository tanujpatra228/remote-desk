//! Password management for RemoteDesk
//!
//! This module handles password hashing, verification, and storage
//! using the Argon2id algorithm for secure password handling.

use crate::error::{SecurityError, SecurityResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::fs;
use std::path::Path;

// Password constraints (avoiding magic numbers)
const PASSWORD_MIN_LENGTH: usize = 6;
const PASSWORD_MAX_LENGTH: usize = 128;

/// Password manager for hashing and verifying passwords
pub struct PasswordManager;

impl PasswordManager {
    /// Hashes a password using Argon2id
    ///
    /// # Arguments
    ///
    /// * `password` - The password to hash
    ///
    /// # Returns
    ///
    /// A PHC-formatted hash string that can be stored
    ///
    /// # Errors
    ///
    /// Returns error if password is invalid or hashing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use remote_desk::security::password::PasswordManager;
    ///
    /// let hash = PasswordManager::hash_password("my_secure_password").unwrap();
    /// ```
    pub fn hash_password(password: &str) -> SecurityResult<String> {
        // Validate password length
        Self::validate_password_length(password)?;

        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);

        // Hash the password using Argon2id with default parameters
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| {
                SecurityError::EncryptionError(format!("Failed to hash password: {}", e))
            })?;

        Ok(password_hash.to_string())
    }

    /// Verifies a password against a hash
    ///
    /// # Arguments
    ///
    /// * `password` - The password to verify
    /// * `hash` - The stored password hash
    ///
    /// # Returns
    ///
    /// `Ok(())` if the password matches, error otherwise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use remote_desk::security::password::PasswordManager;
    ///
    /// let hash = PasswordManager::hash_password("my_password").unwrap();
    /// assert!(PasswordManager::verify_password("my_password", &hash).is_ok());
    /// assert!(PasswordManager::verify_password("wrong_password", &hash).is_err());
    /// ```
    pub fn verify_password(password: &str, hash: &str) -> SecurityResult<()> {
        let parsed_hash = PasswordHash::new(hash).map_err(|_e| {
            SecurityError::PasswordVerificationFailed
        })?;

        let argon2 = Argon2::default();

        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| SecurityError::PasswordVerificationFailed)?;

        Ok(())
    }

    /// Validates password length
    fn validate_password_length(password: &str) -> SecurityResult<()> {
        let len = password.len();

        if len < PASSWORD_MIN_LENGTH {
            return Err(SecurityError::PasswordTooShort {
                min: PASSWORD_MIN_LENGTH,
            });
        }

        if len > PASSWORD_MAX_LENGTH {
            return Err(SecurityError::PasswordTooLong {
                max: PASSWORD_MAX_LENGTH,
            });
        }

        Ok(())
    }

    /// Saves a password hash to a file
    ///
    /// # Errors
    ///
    /// Returns error if the file cannot be written
    pub fn save_password_hash(hash_file_path: &Path, hash: &str) -> SecurityResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = hash_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SecurityError::EncryptionError(format!(
                    "Failed to create directory for password hash: {}",
                    e
                ))
            })?;
        }

        fs::write(hash_file_path, hash).map_err(|e| {
            SecurityError::EncryptionError(format!("Failed to write password hash: {}", e))
        })?;

        Ok(())
    }

    /// Loads a password hash from a file
    ///
    /// # Errors
    ///
    /// Returns error if the file cannot be read
    pub fn load_password_hash(hash_file_path: &Path) -> SecurityResult<String> {
        if !hash_file_path.exists() {
            return Err(SecurityError::AuthenticationFailed(
                "Password not set".to_string(),
            ));
        }

        fs::read_to_string(hash_file_path).map_err(|e| {
            SecurityError::EncryptionError(format!("Failed to read password hash: {}", e))
        })
    }

    /// Checks if a password is set (hash file exists)
    pub fn is_password_set(hash_file_path: &Path) -> bool {
        hash_file_path.exists()
    }

    /// Removes the password (deletes hash file)
    ///
    /// # Errors
    ///
    /// Returns error if the file cannot be deleted
    pub fn remove_password(hash_file_path: &Path) -> SecurityResult<()> {
        if hash_file_path.exists() {
            fs::remove_file(hash_file_path).map_err(|e| {
                SecurityError::EncryptionError(format!("Failed to remove password hash: {}", e))
            })?;
        }
        Ok(())
    }

    /// Sets a new password
    ///
    /// Hashes the password and saves it to the file
    ///
    /// # Errors
    ///
    /// Returns error if password is invalid or cannot be saved
    pub fn set_password(hash_file_path: &Path, password: &str) -> SecurityResult<()> {
        let hash = Self::hash_password(password)?;
        Self::save_password_hash(hash_file_path, &hash)?;
        Ok(())
    }

    /// Verifies a password against the stored hash file
    ///
    /// # Errors
    ///
    /// Returns error if password doesn't match or hash file cannot be read
    pub fn verify_password_from_file(hash_file_path: &Path, password: &str) -> SecurityResult<()> {
        let hash = Self::load_password_hash(hash_file_path)?;
        Self::verify_password(password, &hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_password_hashing() {
        let password = "my_secure_password";
        let hash = PasswordManager::hash_password(password).unwrap();

        // Hash should not be empty
        assert!(!hash.is_empty());

        // Hash should start with $argon2id$
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_password_verification() {
        let password = "my_secure_password";
        let hash = PasswordManager::hash_password(password).unwrap();

        // Correct password should verify
        assert!(PasswordManager::verify_password(password, &hash).is_ok());

        // Wrong password should fail
        assert!(PasswordManager::verify_password("wrong_password", &hash).is_err());
    }

    #[test]
    fn test_password_length_validation() {
        // Too short
        assert!(PasswordManager::hash_password("short").is_err());

        // Valid
        assert!(PasswordManager::hash_password("valid_password").is_ok());

        // Too long
        let long_password = "a".repeat(PASSWORD_MAX_LENGTH + 1);
        assert!(PasswordManager::hash_password(&long_password).is_err());
    }

    #[test]
    fn test_save_and_load_password() {
        let temp_dir = TempDir::new().unwrap();
        let hash_file = temp_dir.path().join("password.hash");

        let password = "test_password";

        // Set password
        PasswordManager::set_password(&hash_file, password).unwrap();

        // Check if password is set
        assert!(PasswordManager::is_password_set(&hash_file));

        // Verify password from file
        assert!(PasswordManager::verify_password_from_file(&hash_file, password).is_ok());
        assert!(PasswordManager::verify_password_from_file(&hash_file, "wrong").is_err());
    }

    #[test]
    fn test_remove_password() {
        let temp_dir = TempDir::new().unwrap();
        let hash_file = temp_dir.path().join("password.hash");

        // Set password
        PasswordManager::set_password(&hash_file, "test_password").unwrap();
        assert!(PasswordManager::is_password_set(&hash_file));

        // Remove password
        PasswordManager::remove_password(&hash_file).unwrap();
        assert!(!PasswordManager::is_password_set(&hash_file));
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let password = "same_password";

        let hash1 = PasswordManager::hash_password(password).unwrap();
        let hash2 = PasswordManager::hash_password(password).unwrap();

        // Hashes should be different due to random salt
        assert_ne!(hash1, hash2);

        // But both should verify the same password
        assert!(PasswordManager::verify_password(password, &hash1).is_ok());
        assert!(PasswordManager::verify_password(password, &hash2).is_ok());
    }
}
