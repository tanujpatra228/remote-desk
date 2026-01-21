//! Certificate generation and management for QUIC connections
//!
//! This module provides self-signed certificate generation for secure
//! peer-to-peer QUIC connections. Each device generates its own certificate
//! which is persisted in the config directory.

use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rustls::{Certificate as RustlsCert, PrivateKey};
use std::fs;
use std::io::{BufReader, Write};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Certificate validity period in days
const CERT_VALIDITY_DAYS: u32 = 365;

/// Certificate organization name
const CERT_ORG_NAME: &str = "RemoteDesk";

/// Certificate file name
const CERT_FILE_NAME: &str = "server.crt";

/// Private key file name
const KEY_FILE_NAME: &str = "server.key";

/// Error type for certificate operations
#[derive(Debug, thiserror::Error)]
pub enum CertError {
    #[error("Failed to generate certificate: {0}")]
    GenerationFailed(String),

    #[error("Failed to load certificate: {0}")]
    LoadFailed(String),

    #[error("Failed to save certificate: {0}")]
    SaveFailed(String),

    #[error("Invalid certificate: {0}")]
    Invalid(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for certificate operations
pub type CertResult<T> = Result<T, CertError>;

/// Certificate pair containing the certificate and private key
#[derive(Debug, Clone)]
pub struct CertPair {
    /// The certificate chain (single self-signed cert)
    pub cert_chain: Vec<RustlsCert>,
    /// The private key
    pub private_key: PrivateKey,
}

impl CertPair {
    /// Creates a new certificate pair from raw bytes
    pub fn new(cert_der: Vec<u8>, key_der: Vec<u8>) -> Self {
        Self {
            cert_chain: vec![RustlsCert(cert_der)],
            private_key: PrivateKey(key_der),
        }
    }

    /// Returns the certificate chain for rustls
    pub fn cert_chain(&self) -> Vec<RustlsCert> {
        self.cert_chain.clone()
    }

    /// Returns the private key for rustls
    pub fn private_key(&self) -> PrivateKey {
        self.private_key.clone()
    }
}

/// Generates a new self-signed certificate for the given device ID
///
/// # Arguments
///
/// * `device_id` - The device ID to include in the certificate's Common Name
///
/// # Returns
///
/// A `CertPair` containing the certificate and private key
pub fn generate_self_signed_cert(device_id: u32) -> CertResult<CertPair> {
    info!("Generating self-signed certificate for device {}", device_id);

    // Create certificate parameters
    let mut params = CertificateParams::default();

    // Set distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::OrganizationName, CERT_ORG_NAME);
    dn.push(DnType::CommonName, format!("RemoteDesk-{}", device_id));
    params.distinguished_name = dn;

    // Add Subject Alternative Names for localhost and common local addresses
    params.subject_alt_names = vec![
        SanType::DnsName("localhost".to_string()),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
    ];

    // Set validity period
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = params.not_before + time::Duration::days(CERT_VALIDITY_DAYS as i64);

    // Generate key pair
    let key_pair = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| CertError::GenerationFailed(e.to_string()))?;

    params.key_pair = Some(key_pair);

    // Generate certificate
    let cert = Certificate::from_params(params)
        .map_err(|e| CertError::GenerationFailed(e.to_string()))?;

    let cert_der = cert.serialize_der()
        .map_err(|e| CertError::GenerationFailed(e.to_string()))?;

    let key_der = cert.serialize_private_key_der();

    debug!("Generated certificate: {} bytes, key: {} bytes", cert_der.len(), key_der.len());

    Ok(CertPair::new(cert_der, key_der))
}

/// Loads an existing certificate pair from the specified directory
///
/// # Arguments
///
/// * `config_dir` - Path to the configuration directory containing the certificate files
///
/// # Returns
///
/// A `CertPair` if both files exist and are valid, or an error
pub fn load_cert_from_dir(config_dir: &Path) -> CertResult<CertPair> {
    let cert_path = config_dir.join(CERT_FILE_NAME);
    let key_path = config_dir.join(KEY_FILE_NAME);

    if !cert_path.exists() || !key_path.exists() {
        return Err(CertError::LoadFailed("Certificate files not found".to_string()));
    }

    debug!("Loading certificate from {:?}", config_dir);

    // Load certificate
    let cert_file = fs::File::open(&cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader)
        .map_err(|e| CertError::LoadFailed(format!("Failed to parse certificate: {}", e)))?;

    if certs.is_empty() {
        return Err(CertError::LoadFailed("No certificates found in file".to_string()));
    }

    // Load private key
    let key_file = fs::File::open(&key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .map_err(|e| CertError::LoadFailed(format!("Failed to parse private key: {}", e)))?;

    if keys.is_empty() {
        // Try reading as EC key
        let key_file = fs::File::open(&key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let ec_keys = rustls_pemfile::ec_private_keys(&mut key_reader)
            .map_err(|e| CertError::LoadFailed(format!("Failed to parse EC key: {}", e)))?;

        if ec_keys.is_empty() {
            return Err(CertError::LoadFailed("No private keys found in file".to_string()));
        }

        return Ok(CertPair::new(certs[0].clone(), ec_keys[0].clone()));
    }

    info!("Loaded certificate from {:?}", config_dir);
    Ok(CertPair::new(certs[0].clone(), keys[0].clone()))
}

/// Saves a certificate pair to the specified directory
///
/// # Arguments
///
/// * `cert_pair` - The certificate pair to save
/// * `config_dir` - Path to the configuration directory
pub fn save_cert_to_dir(cert_pair: &CertPair, config_dir: &Path) -> CertResult<()> {
    // Ensure directory exists
    fs::create_dir_all(config_dir)?;

    let cert_path = config_dir.join(CERT_FILE_NAME);
    let key_path = config_dir.join(KEY_FILE_NAME);

    debug!("Saving certificate to {:?}", config_dir);

    // Save certificate in PEM format
    let cert_pem = pem::encode(&pem::Pem::new(
        "CERTIFICATE",
        cert_pair.cert_chain[0].0.clone(),
    ));
    let mut cert_file = fs::File::create(&cert_path)?;
    cert_file.write_all(cert_pem.as_bytes())?;

    // Save private key in PEM format
    let key_pem = pem::encode(&pem::Pem::new(
        "PRIVATE KEY",
        cert_pair.private_key.0.clone(),
    ));
    let mut key_file = fs::File::create(&key_path)?;
    key_file.write_all(key_pem.as_bytes())?;

    // Set restrictive permissions on key file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&key_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&key_path, perms)?;
    }

    info!("Saved certificate to {:?}", config_dir);
    Ok(())
}

/// Loads or creates a certificate for the given device
///
/// This is the main entry point for certificate management. It will:
/// 1. Try to load an existing certificate from the config directory
/// 2. If not found, generate a new self-signed certificate
/// 3. Save the new certificate for future use
///
/// # Arguments
///
/// * `config_dir` - Path to the configuration directory
/// * `device_id` - The device ID for certificate generation
pub fn load_or_create_cert(config_dir: &Path, device_id: u32) -> CertResult<CertPair> {
    // Try to load existing certificate
    match load_cert_from_dir(config_dir) {
        Ok(cert) => {
            info!("Using existing certificate");
            return Ok(cert);
        }
        Err(e) => {
            debug!("Could not load existing certificate: {}, generating new one", e);
        }
    }

    // Generate new certificate
    let cert_pair = generate_self_signed_cert(device_id)?;

    // Save for future use
    if let Err(e) = save_cert_to_dir(&cert_pair, config_dir) {
        warn!("Failed to save certificate: {}, will regenerate on next start", e);
    }

    Ok(cert_pair)
}

/// Creates a rustls ServerConfig for accepting connections
pub fn create_server_config(cert_pair: &CertPair) -> CertResult<rustls::ServerConfig> {
    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_pair.cert_chain(), cert_pair.private_key())
        .map_err(|e| CertError::Invalid(e.to_string()))?;

    Ok(config)
}

/// Creates a rustls ClientConfig that skips server certificate verification
///
/// This is necessary for peer-to-peer connections where both peers use
/// self-signed certificates. In a production environment, you would want
/// to implement proper certificate pinning or a trust-on-first-use model.
pub fn create_client_config() -> CertResult<rustls::ClientConfig> {
    // Create a config that doesn't verify server certificates
    // This is acceptable for P2P where we'll use other authentication
    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    Ok(config)
}

/// Certificate verifier that skips verification (for self-signed certs)
struct SkipServerVerification;

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &RustlsCert,
        _intermediates: &[RustlsCert],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        // Skip verification for self-signed certificates
        // Authentication is done at the application layer
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_self_signed_cert() {
        let cert_pair = generate_self_signed_cert(123456789).unwrap();

        assert!(!cert_pair.cert_chain.is_empty());
        assert!(!cert_pair.private_key.0.is_empty());
    }

    #[test]
    fn test_save_and_load_cert() {
        let temp_dir = TempDir::new().unwrap();
        let cert_pair = generate_self_signed_cert(987654321).unwrap();

        // Save
        save_cert_to_dir(&cert_pair, temp_dir.path()).unwrap();

        // Verify files exist
        assert!(temp_dir.path().join(CERT_FILE_NAME).exists());
        assert!(temp_dir.path().join(KEY_FILE_NAME).exists());

        // Load
        let loaded = load_cert_from_dir(temp_dir.path()).unwrap();

        assert_eq!(cert_pair.cert_chain[0].0, loaded.cert_chain[0].0);
    }

    #[test]
    fn test_load_or_create_cert() {
        let temp_dir = TempDir::new().unwrap();

        // First call should create
        let cert1 = load_or_create_cert(temp_dir.path(), 111111111).unwrap();

        // Second call should load existing
        let cert2 = load_or_create_cert(temp_dir.path(), 111111111).unwrap();

        // Should be the same certificate
        assert_eq!(cert1.cert_chain[0].0, cert2.cert_chain[0].0);
    }

    #[test]
    fn test_create_server_config() {
        let cert_pair = generate_self_signed_cert(123456789).unwrap();
        let config = create_server_config(&cert_pair);
        assert!(config.is_ok());
    }

    #[test]
    fn test_create_client_config() {
        let config = create_client_config();
        assert!(config.is_ok());
    }
}
