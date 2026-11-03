#![deny(unsafe_code)]
//! Primary Color Grading, 3D .cube LUTs & Real-Time Video Scopes
//!
//! Provides broadcast-standard color manipulation:
//! - 3-Way Color Wheels (Lift, Gamma, Gain)
//! - Exposure, Contrast, Saturation, Temperature, and Tint
//! - Adobe 3D `.cube` LUT parser with trilinear interpolation
//! - Video Scope analysis: RGB Histogram, Luma Waveform, and Vectorscope (Cb/Cr)

use crate::models::ColorGradingSettings;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Color grading and analysis errors
#[derive(Debug, Error)]
pub enum ColorError {
    #[error("Invalid .cube LUT file: {0}")]
    InvalidCubeLut(String),
    #[error("Image dimensions mismatch with buffer size: {0}x{1} vs {2} bytes")]
    BufferDimensionMismatch(u32, u32, usize),
}

// ---------------------------------------------------------------------------
// 3D .cube LUT Engine
// ---------------------------------------------------------------------------

/// 3D Lookup Table for cinematic film emulation
#[derive(Debug, Clone, PartialEq)]
pub struct CubeLut {
    pub title: String,
    pub size: usize,
    pub table: Vec<[f32; 3]>, // Flat array of [r, g, b] with size^3 elements
}

impl CubeLut {
    /// Parse an Adobe standard 3D `.cube` text representation
    pub fn parse_cube_text(text: &str) -> Result<Self, ColorError> {
        let mut title = "Untitled LUT".to_string();
        let mut size = 0usize;
        let mut table = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(t) = trimmed.strip_prefix("TITLE ") {
                title = t.trim_matches('"').to_string();
            } else if let Some(s) = trimmed.strip_prefix("LUT_3D_SIZE ") {
                size = s
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| ColorError::InvalidCubeLut("Invalid LUT_3D_SIZE".to_string()))?;
            } else if trimmed.starts_with("LUT_1D_SIZE") {
                return Err(ColorError::InvalidCubeLut(
                    "1D LUTs not supported; 3D required".to_string(),
                ));
            } else {
                // Must be RGB float triples
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() == 3 {
                    let r = parts[0].parse::<f32>().map_err(|_| {
                        ColorError::InvalidCubeLut(format!("Invalid float in: {}", trimmed))
                    })?;
                    let g = parts[1].parse::<f32>().map_err(|_| {
                        ColorError::InvalidCubeLut(format!("Invalid float in: {}", trimmed))
                    })?;
                    let b = parts[2].parse::<f32>().map_err(|_| {
                        ColorError::InvalidCubeLut(format!("Invalid float in: {}", trimmed))
                    })?;
                    table.push([r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]);
                }
            }
        }

        if size == 0 || table.len() != size * size * size {
            return Err(ColorError::InvalidCubeLut(format!(
                "Table entry count {} does not match size^3 (expected {})",
                table.len(),
                size * size * size
            )));
        }

        Ok(Self { title, size, table })
    }

    /// Sample color through 3D LUT using trilinear interpolation
    pub fn sample_trilinear(&self, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let s = (self.size - 1) as f32;
        let r_coord = (r.clamp(0.0, 1.0) * s).clamp(0.0, s);
        let g_coord = (g.clamp(0.0, 1.0) * s).clamp(0.0, s);
        let b_coord = (b.clamp(0.0, 1.0) * s).clamp(0.0, s);

        let r0 = r_coord.floor() as usize;
        let g0 = g_coord.floor() as usize;
        let b0 = b_coord.floor() as usize;

        let r1 = (r0 + 1).min(self.size - 1);
        let g1 = (g0 + 1).min(self.size - 1);
        let b1 = (b0 + 1).min(self.size - 1);

        let dr = r_coord - r0 as f32;
        let dg = g_coord - g0 as f32;
        let db = b_coord - b0 as f32;

        let idx = |ri: usize, gi: usize, bi: usize| -> [f32; 3] {
            // Standard .cube indexing: r + g * size + b * size * size
            let index = ri + gi * self.size + bi * self.size * self.size;
            self.table[index]
        };

        let c000 = idx(r0, g0, b0);
        let c100 = idx(r1, g0, b0);
        let c010 = idx(r0, g1, b0);
        let c110 = idx(r1, g1, b0);
        let c001 = idx(r0, g0, b1);
        let c101 = idx(r1, g0, b1);
        let c011 = idx(r0, g1, b1);
        let c111 = idx(r1, g1, b1);

        let interp = |i: usize| -> f32 {
            let c00 = c000[i] * (1.0 - dr) + c100[i] * dr;
            let c01 = c001[i] * (1.0 - dr) + c101[i] * dr;
            let c10 = c010[i] * (1.0 - dr) + c110[i] * dr;
            let c11 = c011[i] * (1.0 - dr) + c111[i] * dr;

            let c0 = c00 * (1.0 - dg) + c10 * dg;
            let c1 = c01 * (1.0 - dg) + c11 * dg;

            c0 * (1.0 - db) + c1 * db
        };

        (interp(0), interp(1), interp(2))
    }
}

// ---------------------------------------------------------------------------
// Primary Color Grading Engine
// ---------------------------------------------------------------------------

pub struct ColorGradingEngine;

impl ColorGradingEngine {
    /// Apply full color grading pipeline to 32-bit RGBA pixel buffer in-place
    pub fn apply(
        rgba_buffer: &mut [u8],
        width: u32,
        height: u32,
        settings: &ColorGradingSettings,
        lut: Option<&CubeLut>,
    ) -> Result<(), ColorError> {
        let expected_len = (width as usize) * (height as usize) * 4;
        if rgba_buffer.len() != expected_len {
            return Err(ColorError::BufferDimensionMismatch(
                width,
                height,
                rgba_buffer.len(),
            ));
        }

        // Fast path: if no LUT and all color grading parameters are identity, no-op
        if lut.is_none() && settings.is_identity() {
            return Ok(());
        }

        // Precompute color grading constants
        let exposure_factor = 2.0f32.powf(settings.exposure);
        let temp_r_mult = 1.0 + (settings.temperature / 100.0) * 0.2;
        let temp_b_mult = 1.0 - (settings.temperature / 100.0) * 0.2;
        let tint_g_mult = 1.0 - (settings.tint / 100.0) * 0.2;

        for chunk in rgba_buffer.chunks_exact_mut(4) {
            let mut r = chunk[0] as f32 / 255.0;
            let mut g = chunk[1] as f32 / 255.0;
            let mut b = chunk[2] as f32 / 255.0;

            // 1. Exposure adjustment
            if settings.exposure != 0.0 {
                r *= exposure_factor;
                g *= exposure_factor;
                b *= exposure_factor;
            }

            // 2. White balance (temperature / tint)
            if settings.temperature != 0.0 || settings.tint != 0.0 {
                r *= temp_r_mult;
                g *= tint_g_mult;
                b *= temp_b_mult;
            }

            // 3. Contrast adjustment around midtone 0.5
            if (settings.contrast - 1.0).abs() > 0.001 {
                r = (r - 0.5) * settings.contrast + 0.5;
                g = (g - 0.5) * settings.contrast + 0.5;
                b = (b - 0.5) * settings.contrast + 0.5;
            }

            // 4. 3-Way Color Wheels: Lift (Shadows), Gamma (Midtones), Gain (Highlights)
            // Compute ITU-R BT.709 relative luminance
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;

            // Lift weight: 1.0 at black, 0.0 at white
            let lift_weight = (1.0 - luma).clamp(0.0, 1.0);
            // Gain weight: 0.0 at black, 1.0 at white
            let gain_weight = luma.clamp(0.0, 1.0);
            // Gamma weight: bell curve peaking at midtone 0.5
            let gamma_weight = (1.0 - (2.0 * luma - 1.0).powi(2)).clamp(0.0, 1.0);

            r += settings.lift.r * lift_weight
                + settings.gamma.r * gamma_weight
                + settings.gain.r * gain_weight;
            g += settings.lift.g * lift_weight
                + settings.gamma.g * gamma_weight
                + settings.gain.g * gain_weight;
            b += settings.lift.b * lift_weight
                + settings.gamma.b * gamma_weight
                + settings.gain.b * gain_weight;

            // 5. Saturation adjustment (luma-preserving)
            if (settings.saturation - 1.0).abs() > 0.001 {
                let post_luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                r = post_luma + (r - post_luma) * settings.saturation;
                g = post_luma + (g - post_luma) * settings.saturation;
                b = post_luma + (b - post_luma) * settings.saturation;
            }

            // 6. Optional 3D .cube LUT application
            if let Some(l) = lut {
                let (lr, lg, lb) = l.sample_trilinear(r, g, b);
                r = lr;
                g = lg;
                b = lb;
            }

            // Clamp and write back to 8-bit integer
            chunk[0] = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
            chunk[1] = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
            chunk[2] = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
            // Alpha chunk[3] preserved unchanged
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Video Scope Analysis (Histogram, Waveform, Vectorscope)
// ---------------------------------------------------------------------------

/// Broadcast video scope analysis telemetry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoScopeData {
    pub histogram_r: Vec<u32>,
    pub histogram_g: Vec<u32>,
    pub histogram_b: Vec<u32>,
    pub histogram_luma: Vec<u32>,
    /// Downsampled Luma waveform columns (128 columns x 64 intensity levels)
    pub waveform_matrix: Vec<u8>,
    pub waveform_cols: u32,
    pub waveform_rows: u32,
    /// 64x64 Cb/Cr chroma vectorscope polar distribution grid
    pub vectorscope_grid: Vec<u8>,
    pub vectorscope_dim: u32,
}

pub struct VideoScopeGenerator;

impl VideoScopeGenerator {
    /// Compute real-time RGB histogram, Luma waveform, and Cb/Cr vectorscope
    pub fn compute_scopes(rgba_buffer: &[u8], width: u32, height: u32) -> VideoScopeData {
        let mut hist_r = vec![0u32; 256];
        let mut hist_g = vec![0u32; 256];
        let mut hist_b = vec![0u32; 256];
        let mut hist_luma = vec![0u32; 256];

        let wf_cols = 128u32;
        let wf_rows = 64u32;
        let mut waveform = vec![0u8; (wf_cols * wf_rows) as usize];

        let vec_dim = 64u32;
        let mut vectorscope = vec![0u8; (vec_dim * vec_dim) as usize];

        let x_scale = wf_cols as f32 / width.max(1) as f32;
        let y_scale = (wf_rows - 1) as f32 / 255.0;

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if idx + 3 >= rgba_buffer.len() {
                    break;
                }

                let r = rgba_buffer[idx];
                let g = rgba_buffer[idx + 1];
                let b = rgba_buffer[idx + 2];

                hist_r[r as usize] += 1;
                hist_g[g as usize] += 1;
                hist_b[b as usize] += 1;

                // ITU-R BT.709 Luma
                let luma_f = 0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32);
                let luma = luma_f.round().clamp(0.0, 255.0) as u8;
                hist_luma[luma as usize] += 1;

                // Waveform column accumulation
                let col = ((x as f32) * x_scale) as u32;
                let row = (wf_rows - 1).saturating_sub(((luma as f32) * y_scale) as u32);
                if col < wf_cols && row < wf_rows {
                    let w_idx = (row * wf_cols + col) as usize;
                    waveform[w_idx] = waveform[w_idx].saturating_add(2);
                }

                // Vectorscope: Cb/Cr chroma calculation
                // Cb = -0.1146*R - 0.3854*G + 0.5000*B
                // Cr =  0.5000*R - 0.4542*G - 0.0458*B
                let cb = -0.1146 * (r as f32) - 0.3854 * (g as f32) + 0.5000 * (b as f32);
                let cr = 0.5000 * (r as f32) - 0.4542 * (g as f32) - 0.0458 * (b as f32);

                // Map Cb, Cr from [-128..127] to [0..vec_dim-1]
                let cb_idx = (((cb + 128.0) / 256.0) * (vec_dim as f32)) as u32;
                let cr_idx = (((cr + 128.0) / 256.0) * (vec_dim as f32)) as u32;

                if cb_idx < vec_dim && cr_idx < vec_dim {
                    let v_idx = (cr_idx * vec_dim + cb_idx) as usize;
                    vectorscope[v_idx] = vectorscope[v_idx].saturating_add(3);
                }
            }
        }

        VideoScopeData {
            histogram_r: hist_r,
            histogram_g: hist_g,
            histogram_b: hist_b,
            histogram_luma: hist_luma,
            waveform_matrix: waveform,
            waveform_cols: wf_cols,
            waveform_rows: wf_rows,
            vectorscope_grid: vectorscope,
            vectorscope_dim: vec_dim,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cube_lut_and_interpolation() {
        let cube_text = r#"
# Simple 2x2x2 Identity LUT test
TITLE "Test Identity"
LUT_3D_SIZE 2

0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
"#;

        let lut = CubeLut::parse_cube_text(cube_text).unwrap();
        assert_eq!(lut.title, "Test Identity");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.table.len(), 8);

        // Test boundary samples
        let (r, g, b) = lut.sample_trilinear(0.0, 0.0, 0.0);
        assert!((r - 0.0).abs() < 1e-4);
        assert!((g - 0.0).abs() < 1e-4);
        assert!((b - 0.0).abs() < 1e-4);

        let (r, g, b) = lut.sample_trilinear(1.0, 1.0, 1.0);
        assert!((r - 1.0).abs() < 1e-4);
        assert!((g - 1.0).abs() < 1e-4);
        assert!((b - 1.0).abs() < 1e-4);

        // Test midpoint trilinear interpolation
        let (r, g, b) = lut.sample_trilinear(0.5, 0.5, 0.5);
        assert!((r - 0.5).abs() < 1e-3);
        assert!((g - 0.5).abs() < 1e-3);
        assert!((b - 0.5).abs() < 1e-3);
    }

    #[test]
    fn test_color_grading_pipeline_exposure_and_saturation() {
        // Create 2x2 red frame (255, 0, 0, 255)
        let mut buffer = vec![
            255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ];

        // 1. Desaturate to grayscale
        let desat_settings = ColorGradingSettings {
            saturation: 0.0,
            ..Default::default()
        };
        ColorGradingEngine::apply(&mut buffer, 2, 2, &desat_settings, None).unwrap();

        // All channels should match BT.709 red luminance (~0.2126 * 255 = ~54)
        assert_eq!(buffer[0], buffer[1]);
        assert_eq!(buffer[1], buffer[2]);
        assert!((buffer[0] as i32 - 54).abs() <= 2);
    }

    #[test]
    fn test_video_scope_generation() {
        // Gradient image 4x4
        let mut buffer = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let v = ((x + y) * 32) as u8;
                buffer.extend_from_slice(&[v, v, v, 255]);
            }
        }

        let scopes = VideoScopeGenerator::compute_scopes(&buffer, 4, 4);
        let total_hist: u32 = scopes.histogram_luma.iter().sum();
        assert_eq!(total_hist, 16); // 16 pixels
        assert_eq!(scopes.waveform_cols, 128);
        assert_eq!(scopes.vectorscope_dim, 64);
    }
}
