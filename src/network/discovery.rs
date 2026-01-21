//! Peer discovery for RemoteDesk using mDNS
//!
//! This module handles discovering peers on the local network using mDNS/DNS-SD.
//! It advertises this device and discovers other RemoteDesk instances.

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::security::DeviceId;

/// mDNS service type for RemoteDesk
const SERVICE_TYPE: &str = "_remotedesk._udp.local.";

/// Default service port
pub const DEFAULT_SERVICE_PORT: u16 = 7070;

/// TXT record key for device ID
const TXT_DEVICE_ID: &str = "device_id";

/// TXT record key for device name
const TXT_DEVICE_NAME: &str = "device_name";

/// TXT record key for protocol version
const TXT_PROTOCOL_VERSION: &str = "proto_ver";

/// Stale peer timeout in seconds
const STALE_PEER_TIMEOUT_SECS: u64 = 120;

/// Event emitted during peer discovery
#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// A new peer was discovered
    Discovered(PeerInfo),
    /// A peer was updated
    Updated(PeerInfo),
    /// A peer is no longer available
    Lost(DeviceId),
}

/// Information about a discovered peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Device ID
    pub device_id: DeviceId,
    /// Device name
    pub device_name: String,
    /// Network addresses
    pub addresses: Vec<SocketAddr>,
    /// Protocol version
    pub protocol_version: Option<u8>,
    /// Last seen timestamp
    pub last_seen: Instant,
}

impl PeerInfo {
    /// Creates new peer information
    pub fn new(device_id: DeviceId, device_name: String, addresses: Vec<SocketAddr>) -> Self {
        Self {
            device_id,
            device_name,
            addresses,
            protocol_version: None,
            last_seen: Instant::now(),
        }
    }

    /// Updates the last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Returns the age since last seen
    pub fn age(&self) -> std::time::Duration {
        self.last_seen.elapsed()
    }

    /// Returns the primary address (first IPv4, then IPv6)
    pub fn primary_address(&self) -> Option<SocketAddr> {
        // Prefer IPv4 for compatibility
        self.addresses
            .iter()
            .find(|addr| addr.is_ipv4())
            .or_else(|| self.addresses.first())
            .copied()
    }
}

/// Peer discovery manager using mDNS
pub struct PeerDiscovery {
    /// Local device ID
    local_id: DeviceId,
    /// Local device name
    local_name: String,
    /// Service port
    service_port: u16,
    /// Discovered peers
    peers: Arc<RwLock<HashMap<DeviceId, PeerInfo>>>,
    /// mDNS daemon
    mdns: Option<ServiceDaemon>,
    /// Service instance name for this device
    instance_name: String,
    /// Event channel for peer events
    event_tx: Option<mpsc::UnboundedSender<PeerEvent>>,
    /// Browser task handle
    browser_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PeerDiscovery {
    /// Creates a new peer discovery manager
    pub fn new(local_id: DeviceId, local_name: String, service_port: u16) -> Self {
        let instance_name = format!("remotedesk-{}", local_id.as_u32());

        Self {
            local_id,
            local_name,
            service_port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            mdns: None,
            instance_name,
            event_tx: None,
            browser_handle: None,
        }
    }

    /// Starts advertising this device on the network
    pub async fn start_advertising(&mut self) -> Result<(), String> {
        info!(
            "Starting mDNS advertising for device {} on port {}",
            self.local_id.format_with_spaces(),
            self.service_port
        );

        // Create mDNS daemon if not already created
        if self.mdns.is_none() {
            let mdns = ServiceDaemon::new()
                .map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;
            self.mdns = Some(mdns);
        }

        let mdns = self.mdns.as_ref().unwrap();

        // Build TXT records
        let mut properties = HashMap::new();
        properties.insert(TXT_DEVICE_ID.to_string(), self.local_id.as_u32().to_string());
        properties.insert(TXT_DEVICE_NAME.to_string(), self.local_name.clone());
        properties.insert(
            TXT_PROTOCOL_VERSION.to_string(),
            crate::network::protocol::CURRENT_PROTOCOL_VERSION.to_string(),
        );

        // Create service info
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &self.instance_name,
            &format!("{}.local.", hostname::get().unwrap_or_default().to_string_lossy()),
            "",
            self.service_port,
            properties,
        )
        .map_err(|e| format!("Failed to create service info: {}", e))?;

        // Register service
        mdns.register(service_info)
            .map_err(|e| format!("Failed to register service: {}", e))?;

        info!(
            "mDNS advertising started: {} on port {}",
            self.instance_name, self.service_port
        );

        Ok(())
    }

    /// Starts discovering peers on the network
    pub async fn start_discovery(&mut self) -> Result<mpsc::UnboundedReceiver<PeerEvent>, String> {
        info!("Starting mDNS peer discovery");

        // Create mDNS daemon if not already created
        if self.mdns.is_none() {
            let mdns = ServiceDaemon::new()
                .map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;
            self.mdns = Some(mdns);
        }

        let mdns = self.mdns.as_ref().unwrap();

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.event_tx = Some(event_tx.clone());

        // Browse for services
        let receiver = mdns
            .browse(SERVICE_TYPE)
            .map_err(|e| format!("Failed to browse services: {}", e))?;

        // Spawn task to handle discovery events
        let peers = self.peers.clone();
        let local_id = self.local_id;

        let handle = tokio::spawn(async move {
            Self::discovery_loop(receiver, peers, local_id, event_tx).await;
        });

        self.browser_handle = Some(handle);

        info!("mDNS discovery started");
        Ok(event_rx)
    }

    /// Main discovery event loop
    async fn discovery_loop(
        receiver: mdns_sd::Receiver<ServiceEvent>,
        peers: Arc<RwLock<HashMap<DeviceId, PeerInfo>>>,
        local_id: DeviceId,
        event_tx: mpsc::UnboundedSender<PeerEvent>,
    ) {
        loop {
            match receiver.recv() {
                Ok(event) => {
                    Self::handle_service_event(event, &peers, local_id, &event_tx).await;
                }
                Err(_) => {
                    debug!("mDNS receiver closed");
                    break;
                }
            }
        }
    }

    /// Handles a single service event
    async fn handle_service_event(
        event: ServiceEvent,
        peers: &Arc<RwLock<HashMap<DeviceId, PeerInfo>>>,
        local_id: DeviceId,
        event_tx: &mpsc::UnboundedSender<PeerEvent>,
    ) {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                // Parse device ID from TXT records
                let device_id_str = info
                    .get_properties()
                    .get(TXT_DEVICE_ID)
                    .map(|v| v.val_str());

                let device_id = match device_id_str {
                    Some(id_str) => match id_str.parse::<u32>() {
                        Ok(id) => match DeviceId::from_u32(id) {
                            Ok(device_id) => device_id,
                            Err(_) => {
                                debug!("Invalid device ID in mDNS record: {}", id_str);
                                return;
                            }
                        },
                        Err(_) => {
                            debug!("Could not parse device ID: {}", id_str);
                            return;
                        }
                    },
                    None => {
                        debug!("No device ID in mDNS record");
                        return;
                    }
                };

                // Skip our own device
                if device_id == local_id {
                    return;
                }

                // Get device name
                let device_name = info
                    .get_properties()
                    .get(TXT_DEVICE_NAME)
                    .map(|v| v.val_str().to_string())
                    .unwrap_or_else(|| format!("Device-{}", device_id.as_u32()));

                // Get protocol version
                let protocol_version = info
                    .get_properties()
                    .get(TXT_PROTOCOL_VERSION)
                    .and_then(|v| v.val_str().parse().ok());

                // Build addresses
                let port = info.get_port();
                let addresses: Vec<SocketAddr> = info
                    .get_addresses()
                    .iter()
                    .map(|ip| SocketAddr::new(*ip, port))
                    .collect();

                if addresses.is_empty() {
                    debug!("No addresses for peer {}", device_id.format_with_spaces());
                    return;
                }

                let mut peer_info = PeerInfo::new(device_id, device_name.clone(), addresses.clone());
                peer_info.protocol_version = protocol_version;

                // Check if this is a new peer or update
                let mut peers = peers.write().await;
                let is_new = !peers.contains_key(&device_id);

                peers.insert(device_id, peer_info.clone());

                let event = if is_new {
                    info!(
                        "Discovered peer: {} ({}) at {:?}",
                        device_id.format_with_spaces(),
                        device_name,
                        addresses
                    );
                    PeerEvent::Discovered(peer_info)
                } else {
                    debug!("Updated peer: {}", device_id.format_with_spaces());
                    PeerEvent::Updated(peer_info)
                };

                let _ = event_tx.send(event);
            }

            ServiceEvent::ServiceRemoved(_, fullname) => {
                // Try to find and remove the peer
                let mut peers = peers.write().await;

                // Extract device ID from fullname (format: remotedesk-XXXXXXXXX._remotedesk._udp.local.)
                if let Some(id_part) = fullname.strip_prefix("remotedesk-") {
                    if let Some(id_str) = id_part.split('.').next() {
                        if let Ok(id) = id_str.parse::<u32>() {
                            if let Ok(device_id) = DeviceId::from_u32(id) {
                                if peers.remove(&device_id).is_some() {
                                    info!("Peer left: {}", device_id.format_with_spaces());
                                    let _ = event_tx.send(PeerEvent::Lost(device_id));
                                }
                            }
                        }
                    }
                }
            }

            ServiceEvent::SearchStarted(_) => {
                debug!("mDNS search started");
            }

            ServiceEvent::SearchStopped(_) => {
                debug!("mDNS search stopped");
            }

            _ => {}
        }
    }

    /// Stops advertising
    pub async fn stop_advertising(&self) {
        info!("Stopping mDNS advertising");

        if let Some(ref mdns) = self.mdns {
            if let Err(e) = mdns.unregister(&format!("{}.{}", self.instance_name, SERVICE_TYPE)) {
                warn!("Failed to unregister mDNS service: {}", e);
            }
        }
    }

    /// Stops discovery
    pub async fn stop_discovery(&mut self) {
        info!("Stopping mDNS discovery");

        if let Some(handle) = self.browser_handle.take() {
            handle.abort();
        }

        if let Some(ref mdns) = self.mdns {
            if let Err(e) = mdns.stop_browse(SERVICE_TYPE) {
                warn!("Failed to stop mDNS browse: {}", e);
            }
        }
    }

    /// Shuts down the mDNS daemon
    pub fn shutdown(&mut self) {
        if let Some(mdns) = self.mdns.take() {
            if let Err(e) = mdns.shutdown() {
                warn!("Failed to shutdown mDNS daemon: {}", e);
            }
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
    pub async fn resolve(&self, device_id: DeviceId) -> Option<Vec<SocketAddr>> {
        if let Some(peer) = self.get_peer(device_id).await {
            if !peer.addresses.is_empty() {
                return Some(peer.addresses);
            }
        }

        debug!(
            "Peer {} not found in local discovery",
            device_id.format_with_spaces()
        );

        None
    }

    /// Adds or updates a discovered peer manually
    pub async fn add_peer(&self, peer_info: PeerInfo) {
        let device_id = peer_info.device_id;
        let mut peers = self.peers.write().await;

        if peers.contains_key(&device_id) {
            debug!("Updating peer: {}", device_id.format_with_spaces());
        } else {
            info!(
                "Manually added peer: {} ({})",
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
            info!("Removed peer: {}", device_id.format_with_spaces());
        }
    }

    /// Cleans up stale peers
    pub async fn cleanup_stale_peers(&self) {
        let max_age = std::time::Duration::from_secs(STALE_PEER_TIMEOUT_SECS);
        let mut peers = self.peers.write().await;

        peers.retain(|id, peer| {
            if peer.age() > max_age {
                info!("Removing stale peer: {}", id.format_with_spaces());
                false
            } else {
                true
            }
        });
    }
}

impl Drop for PeerDiscovery {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peer_discovery_creation() {
        let device_id = DeviceId::from_u32(123456789).unwrap();
        let discovery = PeerDiscovery::new(device_id, "Test Device".to_string(), DEFAULT_SERVICE_PORT);

        assert_eq!(discovery.local_id, device_id);
        assert_eq!(discovery.service_port, DEFAULT_SERVICE_PORT);
    }

    #[tokio::test]
    async fn test_add_and_get_peer() {
        let local_id = DeviceId::from_u32(123456789).unwrap();
        let peer_id = DeviceId::from_u32(987654321).unwrap();

        let discovery = PeerDiscovery::new(local_id, "Local".to_string(), DEFAULT_SERVICE_PORT);

        let addr: SocketAddr = "192.168.1.100:7070".parse().unwrap();
        let peer_info = PeerInfo::new(peer_id, "Remote".to_string(), vec![addr]);

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

        let discovery = PeerDiscovery::new(local_id, "Local".to_string(), DEFAULT_SERVICE_PORT);

        let addr: SocketAddr = "192.168.1.100:7070".parse().unwrap();
        let peer_info = PeerInfo::new(peer_id, "Remote".to_string(), vec![addr]);

        discovery.add_peer(peer_info).await;
        assert!(discovery.has_peer(peer_id).await);

        discovery.remove_peer(peer_id).await;
        assert!(!discovery.has_peer(peer_id).await);
    }

    #[tokio::test]
    async fn test_get_all_peers() {
        let local_id = DeviceId::from_u32(123456789).unwrap();
        let discovery = PeerDiscovery::new(local_id, "Local".to_string(), DEFAULT_SERVICE_PORT);

        let peer1_id = DeviceId::from_u32(111111111).unwrap();
        let peer2_id = DeviceId::from_u32(222222222).unwrap();

        let addr: SocketAddr = "192.168.1.100:7070".parse().unwrap();

        discovery
            .add_peer(PeerInfo::new(peer1_id, "Peer1".to_string(), vec![addr]))
            .await;

        discovery
            .add_peer(PeerInfo::new(peer2_id, "Peer2".to_string(), vec![addr]))
            .await;

        let all_peers = discovery.get_all_peers().await;
        assert_eq!(all_peers.len(), 2);
    }

    #[test]
    fn test_peer_info_primary_address() {
        let device_id = DeviceId::from_u32(123456789).unwrap();

        // Test IPv4 preference
        let ipv4: SocketAddr = "192.168.1.1:7070".parse().unwrap();
        let ipv6: SocketAddr = "[::1]:7070".parse().unwrap();

        let peer = PeerInfo::new(device_id, "Test".to_string(), vec![ipv6, ipv4]);
        assert_eq!(peer.primary_address(), Some(ipv4));

        // Test IPv6 when no IPv4
        let peer = PeerInfo::new(device_id, "Test".to_string(), vec![ipv6]);
        assert_eq!(peer.primary_address(), Some(ipv6));

        // Test empty
        let peer = PeerInfo::new(device_id, "Test".to_string(), vec![]);
        assert_eq!(peer.primary_address(), None);
    }
}
