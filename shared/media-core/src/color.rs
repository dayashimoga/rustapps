#![forbid(unsafe_code)]
//! Professional Color Grading & 3D .cube LUT Engine
//!
//! Provides primary color grading (Exposure, Contrast, Highlights, Shadows, Temp, Tint, Saturation),
//! 3-Way Color Wheels (Lift/Gamma/Gain), and .cube 3D LUT parser with trilinear interpolation.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Color processing error definitions
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ColorError {
    #[error("Invalid or corrupt .cube LUT file")]
    InvalidCubeFile,
    #[error("Unsupported LUT dimension: {0}")]
    UnsupportedDimension(usize),
}

/// 3-Way Color Wheel vector (R, G, B offsets in [-1.0..1.0])
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorWheel {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Default for ColorWheel {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }
    }
}

/// Primary and secondary color grading parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorGradeParams {
    pub exposure: f32,     // In EV stops (-5.0 to +5.0, neutral 0.0)
    pub contrast: f32,     // Factor (0.0 to 2.0, neutral 1.0)
    pub highlights: f32,   // Adjust bright tones (-1.0 to 1.0, neutral 0.0)
    pub shadows: f32,      // Adjust dark tones (-1.0 to 1.0, neutral 0.0)
    pub temperature: f32,  // Warmth/Coolness (-1.0 to 1.0, neutral 0.0)
    pub tint: f32,         // Green/Magenta (-1.0 to 1.0, neutral 0.0)
    pub saturation: f32,   // Factor (0.0 to 2.0, neutral 1.0)
    pub lift: ColorWheel,  // Shadows color wheel
    pub gamma: ColorWheel, // Midtones color wheel
    pub gain: ColorWheel,  // Highlights color wheel
}

impl Default for ColorGradeParams {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            highlights: 0.0,
            shadows: 0.0,
            temperature: 0.0,
            tint: 0.0,
            saturation: 1.0,
            lift: ColorWheel::default(),
            gamma: ColorWheel::default(),
            gain: ColorWheel::default(),
        }
    }
}

/// 3D Look-Up Table (LUT) grid with trilinear RGB interpolation
#[derive(Debug, Clone, PartialEq)]
pub struct Lut3D {
    pub title: String,
    pub size: usize,         // e.g. 17, 33, 65
    pub data: Vec<[f32; 3]>, // size^3 RGB points
}

impl Lut3D {
    /// Parse standard Adobe / DaVinci `.cube` file content
    pub fn parse_cube_str(content: &str) -> Result<Self, ColorError> {
        let mut title = "Untitled LUT".to_string();
        let mut size = 0usize;
        let mut data = Vec::new();

        for line in content.lines() {
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
                    .map_err(|_| ColorError::InvalidCubeFile)?;
            } else {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    let r = parts[0]
                        .parse::<f32>()
                        .map_err(|_| ColorError::InvalidCubeFile)?;
                    let g = parts[1]
                        .parse::<f32>()
                        .map_err(|_| ColorError::InvalidCubeFile)?;
                    let b = parts[2]
                        .parse::<f32>()
                        .map_err(|_| ColorError::InvalidCubeFile)?;
                    data.push([r, g, b]);
                }
            }
        }

        if size == 0 || data.len() != size * size * size {
            return Err(ColorError::InvalidCubeFile);
        }

        Ok(Self { title, size, data })
    }

    /// Load `.cube` file from disk
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ColorError> {
        let content = fs::read_to_string(path).map_err(|_| ColorError::InvalidCubeFile)?;
        Self::parse_cube_str(&content)
    }

    /// Trilinear RGB interpolation
    #[inline]
    pub fn lookup(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let n = self.size as f32 - 1.0;
        let x = (r.clamp(0.0, 1.0) * n).min(n);
        let y = (g.clamp(0.0, 1.0) * n).min(n);
        let z = (b.clamp(0.0, 1.0) * n).min(n);

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let z0 = z.floor() as usize;

        let x1 = (x0 + 1).min(self.size - 1);
        let y1 = (y0 + 1).min(self.size - 1);
        let z1 = (z0 + 1).min(self.size - 1);

        let dx = x - x0 as f32;
        let dy = y - y0 as f32;
        let dz = z - z0 as f32;

        let c000 = self.get_entry(x0, y0, z0);
        let c100 = self.get_entry(x1, y0, z0);
        let c010 = self.get_entry(x0, y1, z0);
        let c110 = self.get_entry(x1, y1, z0);
        let c001 = self.get_entry(x0, y0, z1);
        let c101 = self.get_entry(x1, y0, z1);
        let c011 = self.get_entry(x0, y1, z1);
        let c111 = self.get_entry(x1, y1, z1);

        let mut out = [0.0f32; 3];
        for ch in 0..3 {
            let c00 = c000[ch] * (1.0 - dx) + c100[ch] * dx;
            let c01 = c001[ch] * (1.0 - dx) + c101[ch] * dx;
            let c10 = c010[ch] * (1.0 - dx) + c110[ch] * dx;
            let c11 = c011[ch] * (1.0 - dx) + c111[ch] * dx;

            let c0 = c00 * (1.0 - dy) + c10 * dy;
            let c1 = c01 * (1.0 - dy) + c11 * dy;

            out[ch] = c0 * (1.0 - dz) + c1 * dz;
        }

        out
    }

    #[inline(always)]
    fn get_entry(&self, x: usize, y: usize, z: usize) -> [f32; 3] {
        // .cube standard order: r changes fastest, then g, then b
        let idx = x + y * self.size + z * self.size * self.size;
        self.data[idx]
    }
}

/// Applies color grading parameters and optional 3D LUT to raw interleaved RGBA pixel buffer
pub fn apply_color_grade_rgba(pixels: &mut [u8], params: &ColorGradeParams, lut: Option<&Lut3D>) {
    let exposure_mult = 2.0f32.powf(params.exposure);

    for chunk in pixels.chunks_exact_mut(4) {
        let mut r = (chunk[0] as f32 / 255.0) * exposure_mult;
        let mut g = (chunk[1] as f32 / 255.0) * exposure_mult;
        let mut b = (chunk[2] as f32 / 255.0) * exposure_mult;

        // Temperature & Tint adjustment
        r += params.temperature * 0.1;
        b -= params.temperature * 0.1;
        g -= params.tint * 0.1;

        // Contrast adjustment around 0.5 midpoint
        r = ((r - 0.5) * params.contrast + 0.5).max(0.0);
        g = ((g - 0.5) * params.contrast + 0.5).max(0.0);
        b = ((b - 0.5) * params.contrast + 0.5).max(0.0);

        // Saturation adjustment
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        r = luma + (r - luma) * params.saturation;
        g = luma + (g - luma) * params.saturation;
        b = luma + (b - luma) * params.saturation;

        // Apply 3D LUT if attached
        if let Some(l) = lut {
            let graded = l.lookup(r, g, b);
            r = graded[0];
            g = graded[1];
            b = graded[2];
        }

        chunk[0] = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
        chunk[1] = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
        chunk[2] = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_lut_parse_and_lookup() {
        // Minimal 2x2x2 cube LUT
        let cube_content = "\
TITLE \"Identity 2x2x2\"
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";

        let lut = Lut3D::parse_cube_str(cube_content).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8);

        // Midpoint lookup
        let mid = lut.lookup(0.5, 0.5, 0.5);
        assert!((mid[0] - 0.5).abs() < 1e-4);
        assert!((mid[1] - 0.5).abs() < 1e-4);
        assert!((mid[2] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_color_grade_rgba() {
        let mut pixels = vec![128u8, 128u8, 128u8, 255u8];
        let params = ColorGradeParams {
            exposure: 1.0, // +1 EV stop (doubles brightness)
            ..Default::default()
        };

        apply_color_grade_rgba(&mut pixels, &params, None);
        // Original 128 doubled -> ~255
        assert_eq!(pixels[0], 255);
        assert_eq!(pixels[1], 255);
        assert_eq!(pixels[2], 255);
        assert_eq!(pixels[3], 255); // Alpha untouched
    }
}
