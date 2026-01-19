//! Frame decoding for receiving remote desktop frames
//!
//! This module handles decoding frames received over the transport layer.

use crate::desktop::types::{EncodedFrame, Frame, FrameFormat};
use crate::error::{RemoteDeskError, Result, SessionError};
use crate::session::TransportFrame;
use image::ImageBuffer;
use image::Rgba;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;
use tracing::debug;

/// Statistics for frame decoding
#[derive(Debug, Clone, Default)]
pub struct DecoderStats {
    /// Total frames decoded
    pub frames_decoded: u64,
    /// Total frames dropped (failed to decode)
    pub frames_dropped: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes decoded (uncompressed)
    pub bytes_decoded: u64,
    /// Average decode time in milliseconds
    pub avg_decode_time_ms: f64,
    /// Last frame sequence number
    pub last_sequence: u64,
    /// Frames received out of order
    pub out_of_order_frames: u64,
}

impl DecoderStats {
    /// Returns the decode success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.frames_decoded + self.frames_dropped;
        if total == 0 {
            return 100.0;
        }
        (self.frames_decoded as f64 / total as f64) * 100.0
    }

    /// Returns the average compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_decoded == 0 {
            return 0.0;
        }
        self.bytes_received as f64 / self.bytes_decoded as f64
    }
}

/// Frame decoder for decoding received frames
pub struct FrameDecoder {
    /// Last decoded frame (for reference)
    last_frame: RwLock<Option<Frame>>,
    /// Expected next sequence number
    expected_sequence: AtomicU64,
    /// Decoding statistics
    stats: RwLock<DecoderStats>,
    /// Running total for average calculation
    decode_time_total_ms: RwLock<f64>,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    /// Creates a new frame decoder
    pub fn new() -> Self {
        Self {
            last_frame: RwLock::new(None),
            expected_sequence: AtomicU64::new(1),
            stats: RwLock::new(DecoderStats::default()),
            decode_time_total_ms: RwLock::new(0.0),
        }
    }

    /// Decodes an EncodedFrame to a Frame
    pub fn decode(&self, encoded: &EncodedFrame) -> Result<Frame> {
        let start = Instant::now();

        let result = self.decode_internal(encoded);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.update_stats(encoded, &result, elapsed_ms);

        result
    }

    /// Decodes a TransportFrame to a Frame
    pub fn decode_transport(&self, transport: &TransportFrame) -> std::result::Result<Frame, SessionError> {
        let start = Instant::now();

        // Convert TransportFrame to EncodedFrame
        let encoded = EncodedFrame {
            width: transport.width,
            height: transport.height,
            data: transport.data.clone(),
            sequence: transport.sequence,
            format: transport.format,
            original_size: transport.original_size,
        };

        let result = self.decode_internal(&encoded);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.update_stats(&encoded, &result, elapsed_ms);

        result.map_err(|e| SessionError::FrameDecodeError(e.to_string()))
    }

    /// Internal decode implementation
    fn decode_internal(&self, encoded: &EncodedFrame) -> Result<Frame> {
        let data = match encoded.format {
            FrameFormat::Raw => encoded.data.clone(),
            FrameFormat::Jpeg => Self::decode_jpeg(encoded)?,
            FrameFormat::Png => Self::decode_png(encoded)?,
            FrameFormat::WebP => {
                // WebP not implemented, attempt as JPEG
                Self::decode_jpeg(encoded)?
            }
        };

        let frame = Frame::new(encoded.width, encoded.height, data, encoded.sequence);

        // Store as last frame
        if let Ok(mut last) = self.last_frame.write() {
            *last = Some(frame.clone());
        }

        debug!(
            "Decoded frame {} ({}x{}, {} bytes)",
            encoded.sequence, encoded.width, encoded.height, encoded.data.len()
        );

        Ok(frame)
    }

    /// Decodes JPEG data
    fn decode_jpeg(encoded: &EncodedFrame) -> Result<Vec<u8>> {
        let cursor = Cursor::new(&encoded.data);
        let img = image::io::Reader::new(cursor)
            .with_guessed_format()
            .map_err(|e| RemoteDeskError::Generic(format!("Failed to read JPEG: {}", e)))?
            .decode()
            .map_err(|e| RemoteDeskError::Generic(format!("JPEG decoding failed: {}", e)))?;

        let rgba_img = img.to_rgba8();
        Ok(rgba_img.into_raw())
    }

    /// Decodes PNG data
    fn decode_png(encoded: &EncodedFrame) -> Result<Vec<u8>> {
        let cursor = Cursor::new(&encoded.data);
        let img = image::io::Reader::new(cursor)
            .with_guessed_format()
            .map_err(|e| RemoteDeskError::Generic(format!("Failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| RemoteDeskError::Generic(format!("PNG decoding failed: {}", e)))?;

        let rgba_img = img.to_rgba8();
        Ok(rgba_img.into_raw())
    }

    /// Updates statistics after decoding
    fn update_stats(&self, encoded: &EncodedFrame, result: &Result<Frame>, elapsed_ms: f64) {
        if let Ok(mut stats) = self.stats.write() {
            let expected = self.expected_sequence.load(Ordering::SeqCst);

            if result.is_ok() {
                stats.frames_decoded += 1;
                stats.bytes_received += encoded.data.len() as u64;
                stats.bytes_decoded += (encoded.width * encoded.height * 4) as u64;

                // Check for out-of-order
                if encoded.sequence != expected && encoded.sequence != 0 {
                    stats.out_of_order_frames += 1;
                }

                stats.last_sequence = encoded.sequence;
                self.expected_sequence.store(encoded.sequence + 1, Ordering::SeqCst);

                // Update average decode time
                if let Ok(mut total) = self.decode_time_total_ms.write() {
                    *total += elapsed_ms;
                    stats.avg_decode_time_ms = *total / stats.frames_decoded as f64;
                }
            } else {
                stats.frames_dropped += 1;
            }
        }
    }

    /// Returns the current statistics
    pub fn stats(&self) -> DecoderStats {
        self.stats.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// Returns the last decoded frame
    pub fn last_frame(&self) -> Option<Frame> {
        self.last_frame.read().ok().and_then(|f| f.clone())
    }

    /// Resets statistics
    pub fn reset_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            *stats = DecoderStats::default();
        }
        if let Ok(mut total) = self.decode_time_total_ms.write() {
            *total = 0.0;
        }
        self.expected_sequence.store(1, Ordering::SeqCst);
    }

    /// Creates an egui-compatible ColorImage from a Frame
    #[cfg(feature = "egui")]
    pub fn frame_to_color_image(frame: &Frame) -> egui::ColorImage {
        egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.data,
        )
    }

    /// Creates an RGBA ImageBuffer from a Frame (useful for further processing)
    pub fn frame_to_image_buffer(frame: &Frame) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_encoded_frame(format: FrameFormat) -> EncodedFrame {
        let width = 100u32;
        let height = 100u32;

        match format {
            FrameFormat::Raw => {
                // Create raw RGBA data
                let data = vec![128u8; (width * height * 4) as usize];
                EncodedFrame {
                    width,
                    height,
                    data,
                    sequence: 1,
                    format: FrameFormat::Raw,
                    original_size: (width * height * 4) as usize,
                }
            }
            FrameFormat::Jpeg | FrameFormat::Png | FrameFormat::WebP => {
                // Create a simple image and encode it
                let mut img_data = Vec::with_capacity((width * height * 4) as usize);
                for y in 0..height {
                    for x in 0..width {
                        let r = (x * 255 / width) as u8;
                        let g = (y * 255 / height) as u8;
                        img_data.push(r);
                        img_data.push(g);
                        img_data.push(128);
                        img_data.push(255);
                    }
                }

                let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::from_raw(width, height, img_data.clone()).unwrap();

                let mut buffer = Cursor::new(Vec::new());
                if format == FrameFormat::Png {
                    image::DynamicImage::ImageRgba8(img)
                        .write_to(&mut buffer, image::ImageFormat::Png)
                        .unwrap();
                } else {
                    let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
                    let mut encoder =
                        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
                    encoder
                        .encode(
                            rgb_img.as_raw(),
                            width,
                            height,
                            image::ColorType::Rgb8,
                        )
                        .unwrap();
                }

                EncodedFrame {
                    width,
                    height,
                    data: buffer.into_inner(),
                    sequence: 1,
                    format,
                    original_size: (width * height * 4) as usize,
                }
            }
        }
    }

    #[test]
    fn test_decode_raw() {
        let decoder = FrameDecoder::new();
        let encoded = create_test_encoded_frame(FrameFormat::Raw);

        let frame = decoder.decode(&encoded).unwrap();
        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 100);
        assert_eq!(frame.data.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_decode_jpeg() {
        let decoder = FrameDecoder::new();
        let encoded = create_test_encoded_frame(FrameFormat::Jpeg);

        let frame = decoder.decode(&encoded).unwrap();
        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 100);
    }

    #[test]
    fn test_decode_png() {
        let decoder = FrameDecoder::new();
        let encoded = create_test_encoded_frame(FrameFormat::Png);

        let frame = decoder.decode(&encoded).unwrap();
        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 100);
    }

    #[test]
    fn test_decode_transport_frame() {
        let decoder = FrameDecoder::new();
        let encoded = create_test_encoded_frame(FrameFormat::Jpeg);

        let transport = TransportFrame::new(
            encoded.sequence,
            encoded.width,
            encoded.height,
            encoded.format,
            encoded.data.clone(),
            encoded.original_size,
            0,
        );

        let frame = decoder.decode_transport(&transport).unwrap();
        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 100);
    }

    #[test]
    fn test_stats_tracking() {
        let decoder = FrameDecoder::new();

        for i in 1..=5 {
            let mut encoded = create_test_encoded_frame(FrameFormat::Raw);
            encoded.sequence = i;
            decoder.decode(&encoded).unwrap();
        }

        let stats = decoder.stats();
        assert_eq!(stats.frames_decoded, 5);
        assert_eq!(stats.frames_dropped, 0);
        assert_eq!(stats.last_sequence, 5);
        assert!(stats.success_rate() > 99.0);
    }

    #[test]
    fn test_out_of_order_detection() {
        let decoder = FrameDecoder::new();

        // Decode frames 1, 2, 3, then 5 (skipping 4)
        for i in [1, 2, 3, 5] {
            let mut encoded = create_test_encoded_frame(FrameFormat::Raw);
            encoded.sequence = i;
            decoder.decode(&encoded).unwrap();
        }

        let stats = decoder.stats();
        assert_eq!(stats.out_of_order_frames, 1); // Frame 5 was out of order
    }

    #[test]
    fn test_last_frame() {
        let decoder = FrameDecoder::new();
        assert!(decoder.last_frame().is_none());

        let encoded = create_test_encoded_frame(FrameFormat::Raw);
        decoder.decode(&encoded).unwrap();

        let last = decoder.last_frame().unwrap();
        assert_eq!(last.width, 100);
    }

    #[test]
    fn test_reset_stats() {
        let decoder = FrameDecoder::new();

        let encoded = create_test_encoded_frame(FrameFormat::Raw);
        decoder.decode(&encoded).unwrap();

        let stats = decoder.stats();
        assert_eq!(stats.frames_decoded, 1);

        decoder.reset_stats();

        let stats = decoder.stats();
        assert_eq!(stats.frames_decoded, 0);
    }
}
