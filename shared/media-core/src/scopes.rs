#![forbid(unsafe_code)]
//! Real-Time Video Scope Analyzers (RGB Histogram, Luma Waveform & Vectorscope)
//!
//! Provides video diagnostic metrics for color grading and broadcast signal compliance.

use serde::{Deserialize, Serialize};

/// 256-bin RGB and Luminance Histogram
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RgbHistogram {
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    pub luma: Vec<u32>,
    pub total_pixels: u32,
}

/// Horizontal Luma Waveform column profiling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LumaWaveformColumn {
    pub min_luma: u8,
    pub max_luma: u8,
    pub avg_luma: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LumaWaveform {
    pub width: usize,
    pub columns: Vec<LumaWaveformColumn>,
}

/// 128x128 2D Chrominance Vectorscope Density Grid
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorscopeData {
    pub grid: Vec<u16>, // 128x128 bin intensities
    pub grid_size: usize,
}

/// Video Scope Analysis Engine
pub struct VideoScopeAnalyzer;

impl VideoScopeAnalyzer {
    /// Compute 256-bin RGB and Luma histogram from interleaved RGBA pixel buffer
    pub fn compute_rgb_histogram(pixels: &[u8], is_rgba: bool) -> RgbHistogram {
        let mut red = vec![0u32; 256];
        let mut green = vec![0u32; 256];
        let mut blue = vec![0u32; 256];
        let mut luma = vec![0u32; 256];

        let stride = if is_rgba { 4 } else { 3 };
        let mut total_pixels = 0u32;

        for chunk in pixels.chunks_exact(stride) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let y = ((299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000) as u8;

            red[r as usize] += 1;
            green[g as usize] += 1;
            blue[b as usize] += 1;
            luma[y as usize] += 1;
            total_pixels += 1;
        }

        RgbHistogram {
            red,
            green,
            blue,
            luma,
            total_pixels,
        }
    }

    /// Compute per-column luminance waveform for horizontal frame analysis
    pub fn compute_luma_waveform(
        pixels: &[u8],
        width: usize,
        height: usize,
        is_rgba: bool,
    ) -> LumaWaveform {
        let stride = if is_rgba { 4 } else { 3 };
        let mut columns = Vec::with_capacity(width);

        for x in 0..width {
            let mut min_y = 255u8;
            let mut max_y = 0u8;
            let mut sum_y = 0u64;

            for y in 0..height {
                let idx = (y * width + x) * stride;
                if idx + 2 < pixels.len() {
                    let r = pixels[idx] as u32;
                    let g = pixels[idx + 1] as u32;
                    let b = pixels[idx + 2] as u32;
                    let luma = ((299 * r + 587 * g + 114 * b) / 1000) as u8;

                    min_y = min_y.min(luma);
                    max_y = max_y.max(luma);
                    sum_y += luma as u64;
                }
            }

            let avg_y = if height > 0 {
                (sum_y / height as u64) as u8
            } else {
                0
            };

            columns.push(LumaWaveformColumn {
                min_luma: min_y,
                max_luma: max_y,
                avg_luma: avg_y,
            });
        }

        LumaWaveform { width, columns }
    }

    /// Compute 128x128 Cb-Cr chrominance vectorscope distribution
    pub fn compute_vectorscope(pixels: &[u8], is_rgba: bool) -> VectorscopeData {
        let grid_size = 128usize;
        let mut grid = vec![0u16; grid_size * grid_size];
        let stride = if is_rgba { 4 } else { 3 };

        for chunk in pixels.chunks_exact(stride) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;

            // ITU-R BT.601 Cb and Cr chrominance coordinates [-128..127]
            let cb = -0.168736 * r - 0.331264 * g + 0.5 * b;
            let cr = 0.5 * r - 0.418688 * g - 0.081312 * b;

            // Map to 128x128 centered grid [0..127]
            let x = ((cb + 128.0) * (grid_size as f32 / 256.0)).clamp(0.0, 127.0) as usize;
            let y = ((cr + 128.0) * (grid_size as f32 / 256.0)).clamp(0.0, 127.0) as usize;

            let idx = y * grid_size + x;
            if idx < grid.len() {
                grid[idx] = grid[idx].saturating_add(1);
            }
        }

        VectorscopeData { grid, grid_size }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_histogram_and_waveform() {
        // 2x2 test image (4 pixels)
        let pixels = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            128, 128, 128, 255, // Gray
        ];

        let hist = VideoScopeAnalyzer::compute_rgb_histogram(&pixels, true);
        assert_eq!(hist.total_pixels, 4);
        assert_eq!(hist.red[255], 1);
        assert_eq!(hist.green[255], 1);
        assert_eq!(hist.blue[255], 1);

        let waveform = VideoScopeAnalyzer::compute_luma_waveform(&pixels, 2, 2, true);
        assert_eq!(waveform.columns.len(), 2);

        let vector = VideoScopeAnalyzer::compute_vectorscope(&pixels, true);
        assert_eq!(vector.grid.len(), 128 * 128);
    }
}
