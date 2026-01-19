//! Clipboard synchronization for remote desktop sessions
//!
//! This module provides clipboard monitoring and synchronization between
//! host and client sessions.

use arboard::Clipboard;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{RemoteDeskError, Result};
use crate::session::transport::{ClipboardContentType, TransportClipboard};

/// Default polling interval for clipboard changes
const DEFAULT_POLL_INTERVAL_MS: u64 = 500;

/// Clipboard content with metadata
#[derive(Debug, Clone)]
pub struct ClipboardContent {
    /// Content type
    pub content_type: ClipboardContentType,
    /// Content data
    pub data: Vec<u8>,
    /// Content hash for deduplication
    pub hash: u64,
    /// Timestamp when content was captured
    pub timestamp: Instant,
}

impl ClipboardContent {
    /// Creates new text content
    pub fn text(text: &str) -> Self {
        let data = text.as_bytes().to_vec();
        let hash = Self::compute_hash(&data);
        Self {
            content_type: ClipboardContentType::Text,
            data,
            hash,
            timestamp: Instant::now(),
        }
    }

    /// Creates new image content (PNG data)
    pub fn image(png_data: Vec<u8>) -> Self {
        let hash = Self::compute_hash(&png_data);
        Self {
            content_type: ClipboardContentType::Image,
            data: png_data,
            hash,
            timestamp: Instant::now(),
        }
    }

    /// Computes hash for content
    fn compute_hash(data: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }

    /// Converts to TransportClipboard
    pub fn to_transport(&self, sequence: u64) -> TransportClipboard {
        TransportClipboard {
            content_type: self.content_type,
            data: self.data.clone(),
            content_hash: self.hash,
            sequence,
        }
    }

    /// Creates from TransportClipboard
    pub fn from_transport(transport: &TransportClipboard) -> Self {
        Self {
            content_type: transport.content_type,
            data: transport.data.clone(),
            hash: transport.content_hash,
            timestamp: Instant::now(),
        }
    }

    /// Returns content as string if it's text
    pub fn as_text(&self) -> Option<String> {
        if self.content_type == ClipboardContentType::Text {
            String::from_utf8(self.data.clone()).ok()
        } else {
            None
        }
    }
}

/// Clipboard monitor for detecting changes
pub struct ClipboardMonitor {
    /// Last content hash
    last_hash: Arc<AtomicU64>,
    /// Whether monitoring is active
    is_running: Arc<AtomicBool>,
    /// Poll interval
    poll_interval: Duration,
    /// Sequence counter
    sequence: Arc<AtomicU64>,
}

impl Default for ClipboardMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardMonitor {
    /// Creates a new clipboard monitor
    pub fn new() -> Self {
        Self {
            last_hash: Arc::new(AtomicU64::new(0)),
            is_running: Arc::new(AtomicBool::new(false)),
            poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Creates a monitor with custom poll interval
    pub fn with_interval(poll_interval_ms: u64) -> Self {
        Self {
            poll_interval: Duration::from_millis(poll_interval_ms),
            ..Self::new()
        }
    }

    /// Starts monitoring clipboard changes
    ///
    /// Returns a receiver for clipboard changes
    pub fn start_monitoring(&self) -> mpsc::Receiver<ClipboardContent> {
        let (tx, rx) = mpsc::channel(32);

        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("Clipboard monitor already running");
            return rx;
        }

        let is_running = Arc::clone(&self.is_running);
        let last_hash = Arc::clone(&self.last_hash);
        let poll_interval = self.poll_interval;
        let sequence = Arc::clone(&self.sequence);

        std::thread::spawn(move || {
            info!("Starting clipboard monitor");

            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create clipboard: {}", e);
                    return;
                }
            };

            while is_running.load(Ordering::SeqCst) {
                // Check for text content
                if let Ok(text) = clipboard.get_text() {
                    let content = ClipboardContent::text(&text);
                    let current_hash = last_hash.load(Ordering::SeqCst);

                    if content.hash != current_hash {
                        last_hash.store(content.hash, Ordering::SeqCst);
                        sequence.fetch_add(1, Ordering::SeqCst);

                        debug!("Clipboard changed: {} chars", text.len());

                        if tx.blocking_send(content).is_err() {
                            debug!("Clipboard receiver closed");
                            break;
                        }
                    }
                }

                // Note: Image clipboard support would require platform-specific handling
                // arboard supports get_image() but the API differs

                std::thread::sleep(poll_interval);
            }

            info!("Clipboard monitor stopped");
        });

        rx
    }

    /// Stops monitoring
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Returns whether monitoring is active
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Gets current clipboard content
    pub fn get_content(&self) -> Result<ClipboardContent> {
        let mut clipboard = Clipboard::new().map_err(|e| {
            RemoteDeskError::Generic(format!("Failed to access clipboard: {}", e))
        })?;

        let text = clipboard.get_text().map_err(|e| {
            RemoteDeskError::Generic(format!("Failed to get clipboard text: {}", e))
        })?;

        Ok(ClipboardContent::text(&text))
    }

    /// Sets clipboard content
    pub fn set_content(&self, content: &ClipboardContent) -> Result<()> {
        let mut clipboard = Clipboard::new().map_err(|e| {
            RemoteDeskError::Generic(format!("Failed to access clipboard: {}", e))
        })?;

        match content.content_type {
            ClipboardContentType::Text => {
                let text = String::from_utf8(content.data.clone()).map_err(|e| {
                    RemoteDeskError::Generic(format!("Invalid UTF-8 in clipboard: {}", e))
                })?;

                clipboard.set_text(&text).map_err(|e| {
                    RemoteDeskError::Generic(format!("Failed to set clipboard text: {}", e))
                })?;

                // Update hash to prevent echo
                self.last_hash.store(content.hash, Ordering::SeqCst);

                debug!("Set clipboard text: {} chars", text.len());
            }
            ClipboardContentType::Html | ClipboardContentType::Image => {
                // HTML and Image handling would require more complex implementation
                warn!("HTML/Image clipboard not yet fully supported");
            }
        }

        Ok(())
    }
}

/// Clipboard synchronizer for bidirectional sync
pub struct ClipboardSync {
    /// Local clipboard monitor
    monitor: ClipboardMonitor,
    /// Send channel for outgoing changes
    outgoing_tx: Option<mpsc::Sender<TransportClipboard>>,
    /// Whether sync is active
    is_running: Arc<AtomicBool>,
    /// Sequence counter
    sequence: Arc<AtomicU64>,
}

impl Default for ClipboardSync {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardSync {
    /// Creates a new clipboard synchronizer
    pub fn new() -> Self {
        Self {
            monitor: ClipboardMonitor::new(),
            outgoing_tx: None,
            is_running: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Starts clipboard synchronization
    ///
    /// Returns a receiver for changes from the remote side
    pub fn start(
        &mut self,
        outgoing_tx: mpsc::Sender<TransportClipboard>,
    ) -> mpsc::Receiver<TransportClipboard> {
        let (incoming_tx, incoming_rx) = mpsc::channel(32);

        self.outgoing_tx = Some(outgoing_tx.clone());
        self.is_running.store(true, Ordering::SeqCst);

        // Start monitoring local clipboard
        let local_changes = self.monitor.start_monitoring();
        let sequence = Arc::clone(&self.sequence);
        let is_running = Arc::clone(&self.is_running);

        // Forward local changes to outgoing channel
        tokio::spawn(async move {
            let mut local_changes = local_changes;

            while is_running.load(Ordering::SeqCst) {
                tokio::select! {
                    Some(content) = local_changes.recv() => {
                        let seq = sequence.fetch_add(1, Ordering::SeqCst);
                        let transport = content.to_transport(seq);

                        if outgoing_tx.send(transport).await.is_err() {
                            debug!("Outgoing clipboard channel closed");
                            break;
                        }
                    }
                    else => break,
                }
            }
        });

        incoming_rx
    }

    /// Stops clipboard synchronization
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.monitor.stop();
        self.outgoing_tx = None;
    }

    /// Applies remote clipboard content locally
    pub fn apply_remote(&self, transport: &TransportClipboard) -> Result<()> {
        let content = ClipboardContent::from_transport(transport);
        self.monitor.set_content(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_content_text() {
        let content = ClipboardContent::text("Hello, World!");
        assert_eq!(content.content_type, ClipboardContentType::Text);
        assert_eq!(content.as_text(), Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_clipboard_content_hash() {
        let content1 = ClipboardContent::text("test");
        let content2 = ClipboardContent::text("test");
        let content3 = ClipboardContent::text("different");

        assert_eq!(content1.hash, content2.hash);
        assert_ne!(content1.hash, content3.hash);
    }

    #[test]
    fn test_transport_conversion() {
        let content = ClipboardContent::text("test content");
        let transport = content.to_transport(42);

        assert_eq!(transport.content_type, ClipboardContentType::Text);
        assert_eq!(transport.sequence, 42);

        let restored = ClipboardContent::from_transport(&transport);
        assert_eq!(restored.as_text(), content.as_text());
    }

    #[test]
    fn test_clipboard_monitor_creation() {
        let monitor = ClipboardMonitor::new();
        assert!(!monitor.is_running());

        let monitor = ClipboardMonitor::with_interval(1000);
        assert_eq!(monitor.poll_interval, Duration::from_millis(1000));
    }

    // Note: Actual clipboard tests would require running on a system with clipboard access
    #[test]
    #[ignore]
    fn test_clipboard_get_set() {
        let monitor = ClipboardMonitor::new();

        let content = ClipboardContent::text("test from RemoteDesk");
        monitor.set_content(&content).unwrap();

        let retrieved = monitor.get_content().unwrap();
        assert_eq!(retrieved.as_text(), content.as_text());
    }
}
