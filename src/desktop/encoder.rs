//! Frame encoding and compression
//!
//! This module handles encoding captured frames for efficient transmission.

use crate::desktop::types::{EncodedFrame, Frame, FrameFormat, Quality};
use crate::error::{RemoteDeskError, Result};
use image::{ImageBuffer, ImageFormat, Rgba};
use std::io::Cursor;
use std::time::Instant;
use tracing::{debug, warn};

/// Minimum quality for JPEG encoding
const JPEG_MIN_QUALITY: Quality = 1;

/// Maximum quality for JPEG encoding
const JPEG_MAX_QUALITY: Quality = 100;

/// Default JPEG quality
const JPEG_DEFAULT_QUALITY: Quality = 80;

/// Frame encoder
pub struct FrameEncoder {
    /// Encoding format
    format: FrameFormat,
    /// Quality setting (for lossy formats)
    quality: Quality,
}

impl FrameEncoder {
    /// Creates a new frame encoder
    pub fn new(format: FrameFormat, quality: Quality) -> Self {
        let quality = quality.clamp(JPEG_MIN_QUALITY, JPEG_MAX_QUALITY);
        Self { format, quality }
    }

    /// Creates a JPEG encoder with specified quality
    pub fn jpeg(quality: Quality) -> Self {
        Self::new(FrameFormat::Jpeg, quality)
    }

    /// Creates a PNG encoder (lossless)
    pub fn png() -> Self {
        Self::new(FrameFormat::Png, JPEG_DEFAULT_QUALITY)
    }

    /// Creates a raw encoder (no compression)
    pub fn raw() -> Self {
        Self::new(FrameFormat::Raw, JPEG_DEFAULT_QUALITY)
    }

    /// Encodes a frame
    ///
    /// # Errors
    ///
    /// Returns error if encoding fails
    pub fn encode(&self, frame: &Frame) -> Result<EncodedFrame> {
        let start = Instant::now();

        let encoded_data = match self.format {
            FrameFormat::Raw => self.encode_raw(frame)?,
            FrameFormat::Jpeg => self.encode_jpeg(frame)?,
            FrameFormat::Png => self.encode_png(frame)?,
            FrameFormat::WebP => {
                // WebP not implemented yet, fall back to JPEG
                warn!("WebP encoding not implemented, using JPEG");
                self.encode_jpeg(frame)?
            }
        };

        let encoded_frame = EncodedFrame {
            width: frame.width,
            height: frame.height,
            data: encoded_data,
            sequence: frame.sequence,
            format: self.format,
            original_size: frame.size_bytes(),
        };

        debug!(
            "Encoded frame {} ({} -> {} bytes, {:.1}% compression) in {:.2}ms",
            frame.sequence,
            frame.size_bytes(),
            encoded_frame.data.len(),
            encoded_frame.compression_percentage(),
            start.elapsed().as_millis()
        );

        Ok(encoded_frame)
    }

    /// Encodes frame as raw RGBA (no compression)
    fn encode_raw(&self, frame: &Frame) -> Result<Vec<u8>> {
        Ok(frame.data.clone())
    }

    /// Encodes frame as JPEG
    fn encode_jpeg(&self, frame: &Frame) -> Result<Vec<u8>> {
        // Create image buffer from RGBA data
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone()).ok_or_else(
                || {
                    RemoteDeskError::Generic("Failed to create image buffer from frame".to_string())
                },
            )?;

        // Encode to JPEG
        let mut buffer = Cursor::new(Vec::new());

        // Convert RGBA to RGB (JPEG doesn't support alpha channel)
        let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();

        // Encode with quality setting
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, self.quality);
        encoder
            .encode(
                rgb_img.as_raw(),
                rgb_img.width(),
                rgb_img.height(),
                image::ColorType::Rgb8,
            )
            .map_err(|e| RemoteDeskError::Generic(format!("JPEG encoding failed: {}", e)))?;

        Ok(buffer.into_inner())
    }

    /// Encodes frame as PNG
    fn encode_png(&self, frame: &Frame) -> Result<Vec<u8>> {
        // Create image buffer from RGBA data
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone()).ok_or_else(
                || {
                    RemoteDeskError::Generic("Failed to create image buffer from frame".to_string())
                },
            )?;

        // Encode to PNG
        let mut buffer = Cursor::new(Vec::new());
        let dynamic_img = image::DynamicImage::ImageRgba8(img);

        dynamic_img
            .write_to(&mut buffer, ImageFormat::Png)
            .map_err(|e| RemoteDeskError::Generic(format!("PNG encoding failed: {}", e)))?;

        Ok(buffer.into_inner())
    }

    /// Decodes an encoded frame back to raw RGBA
    ///
    /// # Errors
    ///
    /// Returns error if decoding fails
    pub fn decode(encoded: &EncodedFrame) -> Result<Frame> {
        let start = Instant::now();

        let data = match encoded.format {
            FrameFormat::Raw => encoded.data.clone(),
            FrameFormat::Jpeg => Self::decode_jpeg(encoded)?,
            FrameFormat::Png => Self::decode_png(encoded)?,
            FrameFormat::WebP => {
                warn!("WebP decoding not implemented, attempting as JPEG");
                Self::decode_jpeg(encoded)?
            }
        };

        let frame = Frame::new(encoded.width, encoded.height, data, encoded.sequence);

        debug!(
            "Decoded frame {} in {:.2}ms",
            encoded.sequence,
            start.elapsed().as_millis()
        );

        Ok(frame)
    }

    /// Decodes JPEG encoded frame
    fn decode_jpeg(encoded: &EncodedFrame) -> Result<Vec<u8>> {
        let cursor = Cursor::new(&encoded.data);
        let img = image::io::Reader::new(cursor)
            .with_guessed_format()
            .map_err(|e| RemoteDeskError::Generic(format!("Failed to read JPEG: {}", e)))?
            .decode()
            .map_err(|e| RemoteDeskError::Generic(format!("JPEG decoding failed: {}", e)))?;

        // Convert to RGBA
        let rgba_img = img.to_rgba8();
        Ok(rgba_img.into_raw())
    }

    /// Decodes PNG encoded frame
    fn decode_png(encoded: &EncodedFrame) -> Result<Vec<u8>> {
        let cursor = Cursor::new(&encoded.data);
        let img = image::io::Reader::new(cursor)
            .with_guessed_format()
            .map_err(|e| RemoteDeskError::Generic(format!("Failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| RemoteDeskError::Generic(format!("PNG decoding failed: {}", e)))?;

        // Convert to RGBA
        let rgba_img = img.to_rgba8();
        Ok(rgba_img.into_raw())
    }

    /// Returns the encoding format
    pub fn format(&self) -> FrameFormat {
        self.format
    }

    /// Returns the quality setting
    pub fn quality(&self) -> Quality {
        self.quality
    }

    /// Sets the quality
    pub fn set_quality(&mut self, quality: Quality) {
        self.quality = quality.clamp(JPEG_MIN_QUALITY, JPEG_MAX_QUALITY);
    }

    /// Sets the format
    pub fn set_format(&mut self, format: FrameFormat) {
        self.format = format;
    }
}

/// Compresses data using zstd
///
/// # Errors
///
/// Returns error if compression fails
pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level)
        .map_err(|e| RemoteDeskError::Generic(format!("zstd compression failed: {}", e)))
}

/// Decompresses zstd data
///
/// # Errors
///
/// Returns error if decompression fails
pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data)
        .map_err(|e| RemoteDeskError::Generic(format!("zstd decompression failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frame() -> Frame {
        // Create a simple gradient frame (red to blue)
        let width = 100;
        let height = 100;
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                let r = (x * 255 / width) as u8;
                let b = (y * 255 / height) as u8;
                data.push(r); // R
                data.push(0); // G
                data.push(b); // B
                data.push(255); // A
            }
        }

        Frame::new(width, height, data, 1)
    }

    #[test]
    fn test_raw_encoding() {
        let frame = create_test_frame();
        let encoder = FrameEncoder::raw();

        let encoded = encoder.encode(&frame).unwrap();
        assert_eq!(encoded.format, FrameFormat::Raw);
        assert_eq!(encoded.data.len(), frame.data.len());

        let decoded = FrameEncoder::decode(&encoded).unwrap();
        assert_eq!(decoded.data, frame.data);
    }

    #[test]
    fn test_jpeg_encoding() {
        let frame = create_test_frame();
        let encoder = FrameEncoder::jpeg(80);

        let encoded = encoder.encode(&frame).unwrap();
        assert_eq!(encoded.format, FrameFormat::Jpeg);
        assert!(encoded.data.len() < frame.data.len()); // Should be compressed

        let decoded = FrameEncoder::decode(&encoded).unwrap();
        assert_eq!(decoded.width, frame.width);
        assert_eq!(decoded.height, frame.height);
        // JPEG is lossy, so data won't match exactly
    }

    #[test]
    fn test_png_encoding() {
        let frame = create_test_frame();
        let encoder = FrameEncoder::png();

        let encoded = encoder.encode(&frame).unwrap();
        assert_eq!(encoded.format, FrameFormat::Png);

        let decoded = FrameEncoder::decode(&encoded).unwrap();
        assert_eq!(decoded.width, frame.width);
        assert_eq!(decoded.height, frame.height);
        // PNG is lossless but may have different representation
    }

    #[test]
    fn test_compression_ratio() {
        let frame = create_test_frame();
        let encoder = FrameEncoder::jpeg(80);

        let encoded = encoder.encode(&frame).unwrap();
        let ratio = encoded.compression_ratio();

        assert!(ratio < 1.0); // Should be compressed
        assert!(ratio > 0.0);

        println!(
            "Compression: {} bytes -> {} bytes ({:.1}%)",
            frame.size_bytes(),
            encoded.data.len(),
            encoded.compression_percentage()
        );
    }

    #[test]
    fn test_zstd_compression() {
        let data = vec![1u8; 1000]; // Highly compressible data
        let compressed = compress_zstd(&data, 3).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = decompress_zstd(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_quality_clamping() {
        let mut encoder = FrameEncoder::jpeg(150); // Invalid quality
        assert_eq!(encoder.quality(), JPEG_MAX_QUALITY);

        encoder.set_quality(0); // Invalid quality
        assert_eq!(encoder.quality(), JPEG_MIN_QUALITY);
    }
}
