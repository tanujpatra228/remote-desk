//! Peer discovery for RemoteDesk
//!
//! This module handles discovering peers on the local network using mDNS.

use crate::security::DeviceId;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

// mDNS constants (avoiding magic numbers)
const SERVICE_TYPE: &str = "_remotedesk._udp.local.";
const SERVICE_PORT_DEFAULT: u16 = 7070;

/// Default service port
pub const DEFAULT_SERVICE_PORT: u16 = SERVICE_PORT_DEFAULT;

/// Information about a discovered peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Device ID
    pub device_id: DeviceId,

    /// Device name
    pub device_name: String,

    /// Network addresses
    pub addresses: Vec<SocketAddr>,

    /// Last seen timestamp
    pub last_seen: std::time::Instant,
}

/// Peer discovery manager
pub struct PeerDiscovery {
    /// Local device ID
    local_id: DeviceId,

    /// Local device name
    local_name: String,

    /// Service port
    service_port: u16,

    /// Discovered peers
    peers: Arc<RwLock<HashMap<DeviceId, PeerInfo>>>,
}

impl PeerDiscovery {
    /// Creates a new peer discovery manager
    pub fn new(local_id: DeviceId, local_name: String, service_port: u16) -> Self {
        Self {
            local_id,
            local_name,
            service_port,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Starts advertising this device on the network
    ///
    /// # Returns
    ///
    /// Returns Ok if advertising started successfully
    pub async fn start_advertising(&self) -> Result<(), String> {
        info!(
            "Starting mDNS advertising for device {} on port {}",
            self.local_id.format_with_spaces(),
            self.service_port
        );

        // TODO: Implement actual mDNS advertising
        // This will use the mdns-sd crate in the full implementation
        // For now, just log that we would advertise

        debug!(
            "Would advertise service: {}@{} (port {})",
            self.local_id.as_u32(),
            SERVICE_TYPE,
            self.service_port
        );

        Ok(())
    }

    /// Starts discovering peers on the network
    ///
    /// # Returns
    ///
    /// Returns Ok if discovery started successfully
    pub async fn start_discovery(&self) -> Result<(), String> {
        info!("Starting mDNS peer discovery");

        // TODO: Implement actual mDNS discovery
        // This will use the mdns-sd crate in the full implementation
        // For now, just log that we would discover

        debug!("Would browse for service: {}", SERVICE_TYPE);

        Ok(())
    }

    /// Stops advertising
    pub async fn stop_advertising(&self) {
        info!("Stopping mDNS advertising");
        // TODO: Stop mDNS advertising
    }

    /// Stops discovery
    pub async fn stop_discovery(&self) {
        info!("Stopping mDNS discovery");
        // TODO: Stop mDNS discovery
    }

    /// Adds or updates a discovered peer
    pub async fn add_peer(&self, peer_info: PeerInfo) {
        let device_id = peer_info.device_id;
        let mut peers = self.peers.write().await;

        if peers.contains_key(&device_id) {
            debug!("Updating peer: {}", device_id.format_with_spaces());
        } else {
            info!(
                "Discovered new peer: {} ({})",
                device_id.format_with_spaces(),
                peer_info.device_name
            );
        }

        peers.insert(device_id, peer_info);
    }

    /// Removes a peer
    pub async fn remove_peer(&self, device_id: DeviceId) {
        let mut peers = self.peers.write().await;
        if peers.remove(&device_id).is_some() {
            info!("Peer left: {}", device_id.format_with_spaces());
        }
    }

    /// Gets information about a peer
    pub async fn get_peer(&self, device_id: DeviceId) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        peers.get(&device_id).cloned()
    }

    /// Gets all discovered peers
    pub async fn get_all_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Checks if a peer is discovered
    pub async fn has_peer(&self, device_id: DeviceId) -> bool {
        let peers = self.peers.read().await;
        peers.contains_key(&device_id)
    }

    /// Resolves a device ID to addresses
    ///
    /// First checks discovered peers, then attempts direct resolution
    pub async fn resolve(&self, device_id: DeviceId) -> Option<Vec<SocketAddr>> {
        // Check if we have this peer already discovered
        if let Some(peer) = self.get_peer(device_id).await {
            if !peer.addresses.is_empty() {
                return Some(peer.addresses);
            }
        }

        // TODO: Implement additional resolution methods:
        // - Check local cache
        // - Query DHT or discovery server (if implemented)
        // - Try well-known ports

        debug!(
            "Peer {} not found in local discovery",
            device_id.format_with_spaces()
        );

        None
    }

    /// Cleans up stale peers
    ///
    /// Removes peers that haven't been seen for a while
    pub async fn cleanup_stale_peers(&self, max_age: std::time::Duration) {
        let mut peers = self.peers.write().await;
        let now = std::time::Instant::now();

        peers.retain(|id, peer| {
            let age = now.duration_since(peer.last_seen);
            if age > max_age {
                info!("Removing stale peer: {}", id.format_with_spaces());
                false
            } else {
                true
            }
        });
    }
}

impl PeerInfo {
    /// Creates new peer information
    pub fn new(device_id: DeviceId, device_name: String, addresses: Vec<SocketAddr>) -> Self {
        Self {
            device_id,
            device_name,
            addresses,
            last_seen: std::time::Instant::now(),
        }
    }

    /// Updates the last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = std::time::Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peer_discovery_creation() {
        let device_id = DeviceId::from_u32(123456789).unwrap();
        let discovery = PeerDiscovery::new(
            device_id,
            "Test Device".to_string(),
            DEFAULT_SERVICE_PORT,
        );

        assert_eq!(discovery.local_id, device_id);
        assert_eq!(discovery.service_port, DEFAULT_SERVICE_PORT);
    }

    #[tokio::test]
    async fn test_add_and_get_peer() {
        let local_id = DeviceId::from_u32(123456789).unwrap();
        let peer_id = DeviceId::from_u32(987654321).unwrap();

        let discovery = PeerDiscovery::new(
            local_id,
            "Local".to_string(),
            DEFAULT_SERVICE_PORT,
        );

        let addr: SocketAddr = "192.168.1.100:7070".parse().unwrap();
        let peer_info = PeerInfo::new(
            peer_id,
            "Remote".to_string(),
            vec![addr],
        );

        discovery.add_peer(peer_info.clone()).await;

        assert!(discovery.has_peer(peer_id).await);

        let retrieved = discovery.get_peer(peer_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().device_id, peer_id);
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let local_id = DeviceId::from_u32(123456789).unwrap();
        let peer_id = DeviceId::from_u32(987654321).unwrap();

        let discovery = PeerDiscovery::new(
            local_id,
            "Local".to_string(),
            DEFAULT_SERVICE_PORT,
        );

        let addr: SocketAddr = "192.168.1.100:7070".parse().unwrap();
        let peer_info = PeerInfo::new(
            peer_id,
            "Remote".to_string(),
            vec![addr],
        );

        discovery.add_peer(peer_info).await;
        assert!(discovery.has_peer(peer_id).await);

        discovery.remove_peer(peer_id).await;
        assert!(!discovery.has_peer(peer_id).await);
    }

    #[tokio::test]
    async fn test_get_all_peers() {
        let local_id = DeviceId::from_u32(123456789).unwrap();
        let discovery = PeerDiscovery::new(
            local_id,
            "Local".to_string(),
            DEFAULT_SERVICE_PORT,
        );

        let peer1_id = DeviceId::from_u32(111111111).unwrap();
        let peer2_id = DeviceId::from_u32(222222222).unwrap();

        let addr: SocketAddr = "192.168.1.100:7070".parse().unwrap();

        discovery.add_peer(PeerInfo::new(
            peer1_id,
            "Peer1".to_string(),
            vec![addr],
        )).await;

        discovery.add_peer(PeerInfo::new(
            peer2_id,
            "Peer2".to_string(),
            vec![addr],
        )).await;

        let all_peers = discovery.get_all_peers().await;
        assert_eq!(all_peers.len(), 2);
    }
}
