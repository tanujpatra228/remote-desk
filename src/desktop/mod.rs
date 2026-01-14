//! Desktop module for RemoteDesk
//!
//! This module handles desktop-related functionality including:
//! - Screen capture across multiple platforms
//! - Frame encoding and compression
//! - Display management
//! - Performance statistics

pub mod capture;
pub mod encoder;
pub mod types;

// Re-export commonly used types
pub use capture::ScreenCapturer;
pub use encoder::{compress_zstd, decompress_zstd, FrameEncoder};
pub use types::{
    CaptureConfig, CaptureStats, DisplayInfo, EncodedFrame, Frame, FrameFormat, Fps, Quality,
    DEFAULT_FPS, DEFAULT_QUALITY, MAX_FPS, MAX_QUALITY, MIN_FPS, MIN_QUALITY,
};
