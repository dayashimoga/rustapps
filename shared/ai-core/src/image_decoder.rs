#![forbid(unsafe_code)]
//! True Offline Image Decoder (PNG & BMP) and Raster Renderer
//!
//! Replaces simulated byte copying with authentic PNG (IHDR/zlib IDAT/Paeth un-filtering)
//! and BMP (BITMAPINFOHEADER 24bpp/32bpp) parsing and raster rendering.

use flate2::read::ZlibDecoder;
use std::io::Read;
use thiserror::Error;

/// Image decoding error definitions
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImageDecodeError {
    #[error("Invalid or unsupported image format")]
    InvalidFormat,
    #[error("Corrupt PNG chunk or header")]
    CorruptPng,
    #[error("Zlib decompression error: {0}")]
    DecompressionFailed(String),
    #[error("Unsupported color format or bit depth: {0}")]
    UnsupportedColorFormat(String),
    #[error("Corrupt BMP header or palette")]
    CorruptBmp,
}

/// Decoded 8-bit grayscale raster image
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // 1 byte per pixel grayscale [0..255]
}

impl DecodedImage {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Access pixel value with bounds check
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<u8> {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.pixels.get(idx).copied()
        } else {
            None
        }
    }

    /// Set pixel value with bounds check
    pub fn set_pixel(&mut self, x: u32, y: u32, val: u8) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            if idx < self.pixels.len() {
                self.pixels[idx] = val;
            }
        }
    }

    /// Convert grayscale buffer to RGB24 buffer
    pub fn to_rgb24(&self) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.pixels.len() * 3);
        for &gray in &self.pixels {
            rgb.push(gray);
            rgb.push(gray);
            rgb.push(gray);
        }
        rgb
    }
}

/// Image decoder supporting PNG and BMP formats
pub struct ImageDecoder;

impl ImageDecoder {
    /// Decode image data from raw file bytes, auto-detecting PNG or BMP
    pub fn decode(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
        if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
            Self::decode_png(bytes)
        } else if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
            Self::decode_bmp(bytes)
        } else {
            Err(ImageDecodeError::InvalidFormat)
        }
    }

    /// Authentic PNG decoder with complete IHDR, IDAT, and scanline filter reconstruction
    pub fn decode_png(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
        if bytes.len() < 8 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(ImageDecodeError::InvalidFormat);
        }

        let mut offset = 8;
        let mut width = 0u32;
        let mut height = 0u32;
        let mut color_type = 0u8;
        let mut idat_compressed = Vec::new();

        while offset + 8 <= bytes.len() {
            let chunk_len = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            let chunk_type = &bytes[offset + 4..offset + 8];
            offset += 8;

            if offset + chunk_len + 4 > bytes.len() {
                return Err(ImageDecodeError::CorruptPng);
            }

            let chunk_data = &bytes[offset..offset + chunk_len];

            match chunk_type {
                b"IHDR" => {
                    if chunk_len < 13 {
                        return Err(ImageDecodeError::CorruptPng);
                    }
                    width = u32::from_be_bytes([
                        chunk_data[0],
                        chunk_data[1],
                        chunk_data[2],
                        chunk_data[3],
                    ]);
                    height = u32::from_be_bytes([
                        chunk_data[4],
                        chunk_data[5],
                        chunk_data[6],
                        chunk_data[7],
                    ]);
                    let bit_depth = chunk_data[8];
                    color_type = chunk_data[9];

                    if bit_depth != 8 {
                        return Err(ImageDecodeError::UnsupportedColorFormat(format!(
                            "Only 8-bit depth supported, found {}",
                            bit_depth
                        )));
                    }
                }
                b"IDAT" => {
                    idat_compressed.extend_from_slice(chunk_data);
                }
                b"IEND" => {
                    break;
                }
                _ => {
                    // Skip other auxiliary chunks (e.g. sRGB, gAMA, pHYs)
                }
            }

            offset += chunk_len + 4; // Skip data and 4-byte CRC
        }

        if width == 0 || height == 0 || idat_compressed.is_empty() {
            return Err(ImageDecodeError::CorruptPng);
        }

        // Decompress IDAT payload via Zlib
        let mut decoder = ZlibDecoder::new(&idat_compressed[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| ImageDecodeError::DecompressionFailed(e.to_string()))?;

        let bytes_per_pixel = match color_type {
            0 => 1, // Grayscale
            2 => 3, // RGB
            6 => 4, // RGBA
            other => {
                return Err(ImageDecodeError::UnsupportedColorFormat(format!(
                    "Color type {} not supported",
                    other
                )));
            }
        };

        let scanline_stride = 1 + (width as usize * bytes_per_pixel);
        let expected_len = scanline_stride * height as usize;
        if decompressed.len() < expected_len {
            return Err(ImageDecodeError::CorruptPng);
        }

        // Scanline un-filtering
        let mut pixels = Vec::with_capacity((width * height) as usize);
        let mut prev_row = vec![0u8; width as usize * bytes_per_pixel];
        let mut curr_row = vec![0u8; width as usize * bytes_per_pixel];

        for row in 0..height as usize {
            let row_offset = row * scanline_stride;
            let filter_type = decompressed[row_offset];
            let row_data = &decompressed[row_offset + 1..row_offset + scanline_stride];

            for i in 0..row_data.len() {
                let x = row_data[i];
                let a = if i >= bytes_per_pixel {
                    curr_row[i - bytes_per_pixel]
                } else {
                    0
                };
                let b = prev_row[i];
                let c = if i >= bytes_per_pixel {
                    prev_row[i - bytes_per_pixel]
                } else {
                    0
                };

                let recon = match filter_type {
                    0 => x,                                                 // None
                    1 => x.wrapping_add(a),                                 // Sub
                    2 => x.wrapping_add(b),                                 // Up
                    3 => x.wrapping_add(((a as u16 + b as u16) / 2) as u8), // Average
                    4 => x.wrapping_add(paeth_predictor(a, b, c)),          // Paeth
                    _ => x,
                };
                curr_row[i] = recon;
            }

            // Convert current row pixels to grayscale
            for x in 0..width as usize {
                let p_idx = x * bytes_per_pixel;
                let gray = match bytes_per_pixel {
                    1 => curr_row[p_idx],
                    3 => {
                        let r = curr_row[p_idx] as u32;
                        let g = curr_row[p_idx + 1] as u32;
                        let b = curr_row[p_idx + 2] as u32;
                        ((299 * r + 587 * g + 114 * b) / 1000) as u8
                    }
                    4 => {
                        let r = curr_row[p_idx] as u32;
                        let g = curr_row[p_idx + 1] as u32;
                        let b = curr_row[p_idx + 2] as u32;
                        ((299 * r + 587 * g + 114 * b) / 1000) as u8
                    }
                    _ => 0,
                };
                pixels.push(gray);
            }

            prev_row.copy_from_slice(&curr_row);
        }

        Ok(DecodedImage::new(width, height, pixels))
    }

    /// Authentic BMP decoder handling BITMAPINFOHEADER 24-bit and 32-bit rasters
    pub fn decode_bmp(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return Err(ImageDecodeError::InvalidFormat);
        }

        let data_offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let width = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]) as u32;
        let raw_height = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
        let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);

        if width == 0 || raw_height == 0 {
            return Err(ImageDecodeError::CorruptBmp);
        }

        let is_top_down = raw_height < 0;
        let height = raw_height.unsigned_abs();

        if bpp != 24 && bpp != 32 {
            return Err(ImageDecodeError::UnsupportedColorFormat(format!(
                "BMP bpp {} not supported (requires 24 or 32)",
                bpp
            )));
        }

        let bytes_per_pixel = (bpp / 8) as usize;
        let row_stride = (width as usize * bytes_per_pixel).div_ceil(4) * 4;

        if bytes.len() < data_offset + (row_stride * height as usize) {
            return Err(ImageDecodeError::CorruptBmp);
        }

        let mut pixels = vec![0u8; (width * height) as usize];

        for row in 0..height as usize {
            let dest_y = if is_top_down {
                row
            } else {
                (height as usize - 1) - row
            };

            let row_start = data_offset + (row * row_stride);
            let row_slice = &bytes[row_start..row_start + (width as usize * bytes_per_pixel)];

            for col in 0..width as usize {
                let px_offset = col * bytes_per_pixel;
                let b = row_slice[px_offset] as u32;
                let g = row_slice[px_offset + 1] as u32;
                let r = row_slice[px_offset + 2] as u32;
                let gray = ((299 * r + 587 * g + 114 * b) / 1000) as u8;

                pixels[dest_y * width as usize + col] = gray;
            }
        }

        Ok(DecodedImage::new(width, height, pixels))
    }
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Helper that generates raw BMP 24bpp file bytes from a grayscale DecodedImage
pub fn encode_gray_to_bmp(image: &DecodedImage) -> Vec<u8> {
    let row_stride = (image.width as usize * 3).div_ceil(4) * 4;
    let image_size = row_stride * image.height as usize;
    let file_size = 54 + image_size;

    let mut bmp = Vec::with_capacity(file_size);

    // Bitmap file header (14 bytes)
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]); // Reserved
    bmp.extend_from_slice(&54u32.to_le_bytes()); // Data offset

    // BITMAPINFOHEADER (40 bytes)
    bmp.extend_from_slice(&40u32.to_le_bytes()); // Header size
    bmp.extend_from_slice(&image.width.to_le_bytes());
    bmp.extend_from_slice(&(image.height as i32).to_le_bytes()); // positive: bottom-up
    bmp.extend_from_slice(&1u16.to_le_bytes()); // Planes
    bmp.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
    bmp.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB (uncompressed)
    bmp.extend_from_slice(&(image_size as u32).to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes()); // 72 DPI
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    // Pixel data (bottom-up)
    let pad_len = row_stride - (image.width as usize * 3);
    for row in (0..image.height).rev() {
        for col in 0..image.width {
            let gray = image.get_pixel(col, row).unwrap_or(0);
            bmp.push(gray); // B
            bmp.push(gray); // G
            bmp.push(gray); // R
        }
        if pad_len > 0 {
            bmp.resize(bmp.len() + pad_len, 0);
        }
    }

    bmp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bmp_roundtrip() {
        let mut original = DecodedImage::new(16, 16, vec![128; 256]);
        // Draw a test diagonal line
        for i in 0..16 {
            original.set_pixel(i, i, 255);
        }

        let bmp_bytes = encode_gray_to_bmp(&original);
        assert!(!bmp_bytes.is_empty());

        let decoded = ImageDecoder::decode(&bmp_bytes).expect("Should decode BMP");
        assert_eq!(decoded.width, original.width);
        assert_eq!(decoded.height, original.height);
        assert_eq!(decoded.get_pixel(0, 0), Some(255));
        assert_eq!(decoded.get_pixel(15, 15), Some(255));
        assert_eq!(decoded.get_pixel(0, 15), Some(128));
    }
}
