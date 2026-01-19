//! Clipboard module for RemoteDesk
//!
//! This module handles cross-platform clipboard synchronization between
//! host and client sessions.

pub mod sync;

pub use sync::{ClipboardContent, ClipboardMonitor, ClipboardSync};
