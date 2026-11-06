#![forbid(unsafe_code)]
//! ISO/IEC 18004 QR Code Generation, Vector/Raster Rendering & Scanning Engine
//!
//! Provides 100% pure-Rust, offline QR code matrix generation, Galois Field GF(2^8)
//! Reed-Solomon error correction, 8-mask evaluation, SVG/PNG rendering,
//! contrast ratio validation, and PNG/bitmap decoding.

use crate::models::{FinderStyle, ModuleShape, QrEccLevel, QrStyleConfig};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

// -----------------------------------------------------------------------------
// 1. Galois Field GF(2^8) Arithmetic
// -----------------------------------------------------------------------------

/// Primitive polynomial P(x) = x^8 + x^4 + x^3 + x^2 + 1 (0x11D = 285)
const GF256_POLY: usize = 0x11D;

/// Precomputed Galois Field exponential table
static GF_EXP: [u8; 512] = {
    let mut exp = [0u8; 512];
    let mut val = 1usize;
    let mut i = 0;
    while i < 255 {
        exp[i] = val as u8;
        exp[i + 255] = val as u8;
        val <<= 1;
        if val >= 256 {
            val ^= GF256_POLY;
        }
        i += 1;
    }
    exp
};

/// Precomputed Galois Field logarithm table
static GF_LOG: [u8; 256] = {
    let mut log = [0u8; 256];
    let mut i = 0;
    while i < 255 {
        log[GF_EXP[i] as usize] = i as u8;
        i += 1;
    }
    log
};

#[inline]
fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        let idx = (GF_LOG[a as usize] as usize) + (GF_LOG[b as usize] as usize);
        GF_EXP[idx % 255]
    }
}

/// Synthesize Reed-Solomon generator polynomial of degree `ec_len`
fn rs_generator_poly(ec_len: usize) -> Vec<u8> {
    let mut poly = vec![1u8];
    for &alpha in GF_EXP.iter().take(ec_len) {
        let factor = [1u8, alpha];
        let mut next = vec![0u8; poly.len() + 1];
        for (j, &p) in poly.iter().enumerate() {
            next[j] ^= p;
            next[j + 1] ^= gf_mul(p, factor[1]);
        }
        poly = next;
    }
    poly
}

/// Compute Reed-Solomon error correction remainder codewords
fn rs_compute_ec(data: &[u8], ec_len: usize) -> Vec<u8> {
    let gen = rs_generator_poly(ec_len);
    let mut remainder = vec![0u8; ec_len];

    for &byte in data {
        let lead = byte ^ remainder[0];
        remainder.remove(0);
        remainder.push(0);
        if lead != 0 {
            for (r, &g) in remainder.iter_mut().zip(gen.iter().skip(1)) {
                *r ^= gf_mul(lead, g);
            }
        }
    }
    remainder
}

// -----------------------------------------------------------------------------
// 2. ISO/IEC 18004 Tables & Parameters (Versions 1..=10)
// -----------------------------------------------------------------------------

/// Capacity specification per QR Version and ECC Level
#[derive(Debug, Clone, Copy)]
pub struct QrVersionSpec {
    pub version: u8,
    pub total_codewords: usize,
    pub ec_codewords: usize,
    pub blocks: usize,
    pub data_capacity_bytes: usize,
}

impl QrVersionSpec {
    /// Lookup capacity and block parameters for Version 1..=10
    pub fn lookup(version: u8, ecc: QrEccLevel) -> Option<Self> {
        let (total, ec, blocks) = match (version, ecc) {
            // Version 1 (21x21)
            (1, QrEccLevel::Low) => (26, 7, 1),
            (1, QrEccLevel::Medium) => (26, 10, 1),
            (1, QrEccLevel::Quartile) => (26, 13, 1),
            (1, QrEccLevel::High) => (26, 17, 1),

            // Version 2 (25x25)
            (2, QrEccLevel::Low) => (44, 10, 1),
            (2, QrEccLevel::Medium) => (44, 16, 1),
            (2, QrEccLevel::Quartile) => (44, 22, 1),
            (2, QrEccLevel::High) => (44, 28, 1),

            // Version 3 (29x29)
            (3, QrEccLevel::Low) => (70, 15, 1),
            (3, QrEccLevel::Medium) => (70, 26, 1),
            (3, QrEccLevel::Quartile) => (70, 36, 2),
            (3, QrEccLevel::High) => (70, 44, 2),

            // Version 4 (33x33)
            (4, QrEccLevel::Low) => (100, 20, 1),
            (4, QrEccLevel::Medium) => (100, 36, 2),
            (4, QrEccLevel::Quartile) => (100, 52, 2),
            (4, QrEccLevel::High) => (100, 64, 4),

            // Version 5 (37x37)
            (5, QrEccLevel::Low) => (134, 26, 1),
            (5, QrEccLevel::Medium) => (134, 48, 2),
            (5, QrEccLevel::Quartile) => (134, 72, 4),
            (5, QrEccLevel::High) => (134, 88, 4),

            // Version 6 (41x41)
            (6, QrEccLevel::Low) => (172, 36, 2),
            (6, QrEccLevel::Medium) => (172, 64, 4),
            (6, QrEccLevel::Quartile) => (172, 96, 4),
            (6, QrEccLevel::High) => (172, 112, 4),

            // Version 7 (45x45)
            (7, QrEccLevel::Low) => (196, 40, 2),
            (7, QrEccLevel::Medium) => (196, 72, 4),
            (7, QrEccLevel::Quartile) => (196, 108, 6),
            (7, QrEccLevel::High) => (196, 130, 5),

            // Version 8 (49x49)
            (8, QrEccLevel::Low) => (242, 48, 2),
            (8, QrEccLevel::Medium) => (242, 88, 4),
            (8, QrEccLevel::Quartile) => (242, 132, 6),
            (8, QrEccLevel::High) => (242, 156, 6),

            // Version 9 (53x53)
            (9, QrEccLevel::Low) => (292, 60, 2),
            (9, QrEccLevel::Medium) => (292, 110, 5),
            (9, QrEccLevel::Quartile) => (292, 160, 8),
            (9, QrEccLevel::High) => (292, 192, 8),

            // Version 10 (57x57)
            (10, QrEccLevel::Low) => (346, 72, 4),
            (10, QrEccLevel::Medium) => (346, 130, 5),
            (10, QrEccLevel::Quartile) => (346, 192, 8),
            (10, QrEccLevel::High) => (346, 224, 8),

            _ => return None,
        };

        let data_codewords = total - ec;
        // In 8-bit Byte mode, overhead is 4 bits (mode) + 8/16 bits (count)
        let overhead_bits = if version <= 9 { 12 } else { 20 };
        let data_bits_total: usize = data_codewords * 8;
        let max_data_bytes = data_bits_total.saturating_sub(overhead_bits) / 8;

        Some(Self {
            version,
            total_codewords: total,
            ec_codewords: ec,
            blocks,
            data_capacity_bytes: max_data_bytes,
        })
    }

    /// Automatically find the smallest version fitting payload
    pub fn find_fitting_version(payload_len: usize, ecc: QrEccLevel) -> Result<Self, String> {
        for v in 1..=10 {
            if let Some(spec) = Self::lookup(v, ecc) {
                if payload_len <= spec.data_capacity_bytes {
                    return Ok(spec);
                }
            }
        }
        Err(format!(
            "Payload length {} bytes exceeds max supported Version 10 capacity for ECC {:?}",
            payload_len, ecc
        ))
    }
}

/// Alignment pattern center coordinates per Version (ISO/IEC 18004 Table E.1)
fn alignment_pattern_coords(version: u8) -> Vec<usize> {
    match version {
        1 => vec![],
        2 => vec![6, 18],
        3 => vec![6, 22],
        4 => vec![6, 26],
        5 => vec![6, 30],
        6 => vec![6, 34],
        7 => vec![6, 22, 38],
        8 => vec![6, 24, 42],
        9 => vec![6, 26, 46],
        10 => vec![6, 28, 50],
        _ => vec![],
    }
}

// -----------------------------------------------------------------------------
// 3. Bitstream Packing & Mask Evaluation
// -----------------------------------------------------------------------------

/// Packs UTF-8 byte stream into ISO/IEC 18004 data codewords
fn pack_byte_data(payload: &[u8], version: u8, data_codewords: usize) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data_codewords * 8);

    // 1. Mode indicator: Byte mode (0100)
    bits.extend_from_slice(&[false, true, false, false]);

    // 2. Character count indicator (8 bits for V1-V9, 16 bits for V10+)
    let count = payload.len();
    let count_bits = if version <= 9 { 8 } else { 16 };
    for i in (0..count_bits).rev() {
        bits.push((count >> i) & 1 == 1);
    }

    // 3. Payload bytes
    for &b in payload {
        for i in (0..8).rev() {
            bits.push((b >> i) & 1 == 1);
        }
    }

    // 4. Terminator (up to 4 zeroes)
    let max_bits = data_codewords * 8;
    for _ in 0..4 {
        if bits.len() < max_bits {
            bits.push(false);
        }
    }

    // 5. Align to byte boundary
    while bits.len() % 8 != 0 && bits.len() < max_bits {
        bits.push(false);
    }

    // 6. Convert to bytes
    let mut bytes = Vec::with_capacity(data_codewords);
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            if bit {
                byte |= 1 << (7 - i);
            }
        }
        bytes.push(byte);
    }

    // 7. Pad alternating 0xEC and 0x11
    let mut pad = 0xECu8;
    while bytes.len() < data_codewords {
        bytes.push(pad);
        pad = if pad == 0xEC { 0x11 } else { 0xEC };
    }

    bytes
}

/// Represents the 2D boolean module matrix (true = dark, false = light)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrMatrix {
    pub dimension: usize,
    pub version: u8,
    pub ecc_level: QrEccLevel,
    pub mask_pattern: u8,
    pub modules: Vec<bool>,
    pub is_function: Vec<bool>,
}

impl QrMatrix {
    pub fn new(version: u8, ecc_level: QrEccLevel) -> Self {
        let dimension = 17 + (version as usize) * 4;
        let count = dimension * dimension;
        Self {
            dimension,
            version,
            ecc_level,
            mask_pattern: 0,
            modules: vec![false; count],
            is_function: vec![false; count],
        }
    }

    #[inline]
    pub fn get(&self, row: usize, col: usize) -> bool {
        self.modules[row * self.dimension + col]
    }

    #[inline]
    pub fn set(&mut self, row: usize, col: usize, val: bool) {
        self.modules[row * self.dimension + col] = val;
    }

    #[inline]
    pub fn set_function(&mut self, row: usize, col: usize, val: bool) {
        let idx = row * self.dimension + col;
        self.modules[idx] = val;
        self.is_function[idx] = true;
    }

    #[inline]
    pub fn is_func(&self, row: usize, col: usize) -> bool {
        self.is_function[row * self.dimension + col]
    }

    /// Place 3 Finder Patterns (7x7) and Separators
    fn place_finders(&mut self) {
        let dim = self.dimension;
        let coords = [(0, 0), (0, dim - 7), (dim - 7, 0)];

        for (r0, c0) in coords {
            for r in 0..7 {
                for c in 0..7 {
                    let is_dark = r == 0
                        || r == 6
                        || c == 0
                        || c == 6
                        || ((2..=4).contains(&r) && (2..=4).contains(&c));
                    self.set_function(r0 + r, c0 + c, is_dark);
                }
            }

            // Separators (white border)
            for r in 0..8 {
                for c in 0..8 {
                    let rr = if r0 == 0 { r } else { dim - 8 + r };
                    let cc = if c0 == 0 { c } else { dim - 8 + c };
                    if rr < dim && cc < dim && !self.is_func(rr, cc) {
                        self.set_function(rr, cc, false);
                    }
                }
            }
        }
    }

    /// Place Alignment Patterns (5x5) for Version >= 2
    fn place_alignment_patterns(&mut self) {
        let centers = alignment_pattern_coords(self.version);
        if centers.is_empty() {
            return;
        }

        for &r in &centers {
            for &c in &centers {
                // Skip if overlapping with Finder pattern areas
                if self.is_func(r, c) {
                    continue;
                }
                for dr in -2..=2 {
                    for dc in -2..=2 {
                        let rr = (r as isize + dr) as usize;
                        let cc = (c as isize + dc) as usize;
                        let is_dark =
                            dr == -2 || dr == 2 || dc == -2 || dc == 2 || (dr == 0 && dc == 0);
                        self.set_function(rr, cc, is_dark);
                    }
                }
            }
        }
    }

    /// Place Timing Patterns along row 6 and column 6
    fn place_timing_patterns(&mut self) {
        let dim = self.dimension;
        for i in 8..(dim - 8) {
            let is_dark = i % 2 == 0;
            if !self.is_func(6, i) {
                self.set_function(6, i, is_dark);
            }
            if !self.is_func(i, 6) {
                self.set_function(i, 6, is_dark);
            }
        }
    }

    /// Reserve dark module at (4*V + 9, 8) and Format Information areas
    fn reserve_format_and_dark_module(&mut self) {
        let dim = self.dimension;
        // Dark module
        let dark_row = 4 * (self.version as usize) + 9;
        self.set_function(dark_row, 8, true);

        // Format info reservation around finders
        for i in 0..9 {
            if !self.is_func(8, i) {
                self.is_function[8 * dim + i] = true;
            }
            if !self.is_func(i, 8) {
                self.is_function[i * dim + 8] = true;
            }
        }
        for i in (dim - 8)..dim {
            if !self.is_func(8, i) {
                self.is_function[8 * dim + i] = true;
            }
            if !self.is_func(i, 8) {
                self.is_function[i * dim + 8] = true;
            }
        }
    }

    /// Place interleaved data codewords in 2-column zigzag tracks
    fn place_data_bits(&mut self, codewords: &[u8]) {
        let dim = self.dimension;
        let mut bits = Vec::with_capacity(codewords.len() * 8);
        for &byte in codewords {
            for i in (0..8).rev() {
                bits.push((byte >> i) & 1 == 1);
            }
        }

        let mut bit_idx = 0;
        let mut col = (dim - 1) as isize;
        let mut upward = true;

        while col > 0 {
            // Column 6 is reserved for vertical timing pattern
            if col == 6 {
                col -= 1;
            }

            let rows: Vec<usize> = if upward {
                (0..dim).rev().collect()
            } else {
                (0..dim).collect()
            };

            for row in rows {
                for c in [col, col - 1] {
                    let col_u = c as usize;
                    if !self.is_func(row, col_u) {
                        let val = if bit_idx < bits.len() {
                            bits[bit_idx]
                        } else {
                            false
                        };
                        self.set(row, col_u, val);
                        bit_idx += 1;
                    }
                }
            }

            upward = !upward;
            col -= 2;
        }
    }

    /// Write 15-bit BCH (15, 5) masked format info
    fn write_format_info(&mut self, mask: u8) {
        let dim = self.dimension;
        let ecc_bits = self.ecc_level.format_bits();
        let format_val = ((ecc_bits as u32) << 3) | (mask as u32);

        // Calculate BCH remainder: generator G = 0x537 (10100110111)
        let mut rem = format_val << 10;
        for i in (0..5).rev() {
            if (rem >> (i + 10)) & 1 == 1 {
                rem ^= 0x537 << i;
            }
        }

        let bch_code = ((format_val << 10) | rem) ^ 0x5412; // XOR mask per ISO 18004

        // Unpack 15 bits
        let mut bits = [false; 15];
        for (i, bit) in bits.iter_mut().enumerate() {
            *bit = (bch_code >> (14 - i)) & 1 == 1;
        }

        // Top-left finder format placement
        self.set(8, 0, bits[0]);
        self.set(8, 1, bits[1]);
        self.set(8, 2, bits[2]);
        self.set(8, 3, bits[3]);
        self.set(8, 4, bits[4]);
        self.set(8, 5, bits[5]);
        self.set(8, 7, bits[6]);
        self.set(8, 8, bits[7]);
        self.set(7, 8, bits[8]);
        self.set(5, 8, bits[9]);
        self.set(4, 8, bits[10]);
        self.set(3, 8, bits[11]);
        self.set(2, 8, bits[12]);
        self.set(1, 8, bits[13]);
        self.set(0, 8, bits[14]);

        // Top-right and bottom-left split placement
        self.set(dim - 1, 8, bits[0]);
        self.set(dim - 2, 8, bits[1]);
        self.set(dim - 3, 8, bits[2]);
        self.set(dim - 4, 8, bits[3]);
        self.set(dim - 5, 8, bits[4]);
        self.set(dim - 6, 8, bits[5]);
        self.set(dim - 7, 8, bits[6]);

        self.set(8, dim - 8, bits[7]);
        self.set(8, dim - 7, bits[8]);
        self.set(8, dim - 6, bits[9]);
        self.set(8, dim - 5, bits[10]);
        self.set(8, dim - 4, bits[11]);
        self.set(8, dim - 3, bits[12]);
        self.set(8, dim - 2, bits[13]);
        self.set(8, dim - 1, bits[14]);
    }

    /// Evaluates 4 ISO penalty metrics to find the optimal mask
    fn compute_penalty(&self) -> u32 {
        let dim = self.dimension;
        let mut penalty = 0u32;

        // Metric N1: 5 or more consecutive same-color modules in row/col
        for r in 0..dim {
            let mut count = 0;
            let mut last = false;
            for c in 0..dim {
                let v = self.get(r, c);
                if c == 0 || v != last {
                    if count >= 5 {
                        penalty += 3 + (count - 5) as u32;
                    }
                    count = 1;
                    last = v;
                } else {
                    count += 1;
                }
            }
            if count >= 5 {
                penalty += 3 + (count - 5) as u32;
            }
        }

        // Metric N2: 2x2 same-color blocks
        for r in 0..(dim - 1) {
            for c in 0..(dim - 1) {
                let v = self.get(r, c);
                if v == self.get(r + 1, c) && v == self.get(r, c + 1) && v == self.get(r + 1, c + 1)
                {
                    penalty += 3;
                }
            }
        }

        // Metric N4: Dark module ratio deviation from 50%
        let dark_count = self.modules.iter().filter(|&&m| m).count();
        let total = self.modules.len();
        let percent = (dark_count * 100) / total;
        let diff = (percent as isize - 50).unsigned_abs();
        penalty += (diff / 5) as u32 * 10;

        penalty
    }
}

/// Applies one of the 8 ISO mask equations
fn is_masked(mask: u8, r: usize, c: usize) -> bool {
    match mask {
        0 => (r + c) % 2 == 0,
        1 => r % 2 == 0,
        2 => c % 3 == 0,
        3 => (r + c) % 3 == 0,
        4 => (r / 2 + c / 3) % 2 == 0,
        5 => ((r * c) % 2) + ((r * c) % 3) == 0,
        6 => (((r * c) % 2) + ((r * c) % 3)) % 2 == 0,
        7 => (((r + c) % 2) + ((r * c) % 3)) % 2 == 0,
        _ => false,
    }
}

/// Full QR Code Generator: converts payload into optimal masked QrMatrix
#[allow(clippy::needless_range_loop)]
pub fn generate_qr_matrix(payload: &[u8], ecc_level: QrEccLevel) -> Result<QrMatrix, String> {
    let spec = QrVersionSpec::find_fitting_version(payload.len(), ecc_level)?;
    let data_len = spec.total_codewords - spec.ec_codewords;

    // 1. Pack data codewords
    let data_bytes = pack_byte_data(payload, spec.version, data_len);

    // 2. Interleave blocks and compute Reed-Solomon error correction
    let block_count = spec.blocks;
    let data_per_block = data_len / block_count;
    let ec_per_block = spec.ec_codewords / block_count;

    let mut data_blocks = Vec::with_capacity(block_count);
    let mut ec_blocks = Vec::with_capacity(block_count);

    for b in 0..block_count {
        let start = b * data_per_block;
        let end = start + data_per_block;
        let block_data = &data_bytes[start..end];
        let block_ec = rs_compute_ec(block_data, ec_per_block);
        data_blocks.push(block_data);
        ec_blocks.push(block_ec);
    }

    // Interleave data codewords
    let mut final_codewords = Vec::with_capacity(spec.total_codewords);
    for i in 0..data_per_block {
        for b in 0..block_count {
            final_codewords.push(data_blocks[b][i]);
        }
    }
    // Interleave EC codewords
    for i in 0..ec_per_block {
        for b in 0..block_count {
            final_codewords.push(ec_blocks[b][i]);
        }
    }

    // 3. Evaluate all 8 masks to pick the lowest penalty
    let mut best_matrix = None;
    let mut lowest_penalty = u32::MAX;

    for mask in 0..8u8 {
        let mut mat = QrMatrix::new(spec.version, ecc_level);
        mat.place_finders();
        mat.place_alignment_patterns();
        mat.place_timing_patterns();
        mat.reserve_format_and_dark_module();
        mat.place_data_bits(&final_codewords);

        // Apply mask
        let dim = mat.dimension;
        for r in 0..dim {
            for c in 0..dim {
                if !mat.is_func(r, c) && is_masked(mask, r, c) {
                    let cur = mat.get(r, c);
                    mat.set(r, c, !cur);
                }
            }
        }

        mat.write_format_info(mask);
        mat.mask_pattern = mask;

        let penalty = mat.compute_penalty();
        if penalty < lowest_penalty {
            lowest_penalty = penalty;
            best_matrix = Some(mat);
        }
    }

    best_matrix.ok_or_else(|| "Failed to synthesize valid QR matrix".to_string())
}

// -----------------------------------------------------------------------------
// 4. Vector (SVG) & Raster (PNG) Renderers
// -----------------------------------------------------------------------------

/// Parse hex color (`#RRGGBB` or `#RGB`) into RGB tuple
pub fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(15);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(23);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(42);
        (r, g, b)
    } else {
        (15, 23, 42) // Slate dark default
    }
}

/// Calculate WCAG 2.1 relative luminance
fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    let to_linear = |val: u8| -> f64 {
        let v = val as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

/// Calculate contrast ratio between two colors
pub fn compute_contrast_ratio(fg_hex: &str, bg_hex: &str) -> (f64, bool) {
    let (fr, fg, fb) = parse_hex_color(fg_hex);
    let (br, bg, bb) = parse_hex_color(bg_hex);

    let l1 = relative_luminance(fr, fg, fb);
    let l2 = relative_luminance(br, bg, bb);

    let is_inverted = l1 > l2;
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    let ratio = (lighter + 0.05) / (darker + 0.05);
    (ratio, is_inverted)
}

/// Render QrMatrix as scalable vector SVG XML markup
pub fn render_svg(matrix: &QrMatrix, style: &QrStyleConfig) -> String {
    let dim = matrix.dimension;
    let quiet = style.quiet_zone_modules as usize;
    let total_modules = dim + quiet * 2;
    let module_size = 10.0;
    let view_size = (total_modules as f64) * module_size;

    let has_gradient = style.fg_gradient_end.is_some();
    let fill_style = if has_gradient {
        "url(#qr-gradient)"
    } else {
        &style.fg_color
    };

    let mut svg = String::with_capacity(8192);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0:.1} {0:.1}" width="{1}" height="{1}" shape-rendering="crispEdges">"#,
        view_size, style.size_px
    ));

    // Gradient definition if requested
    if let Some(ref grad_end) = style.fg_gradient_end {
        svg.push_str(&format!(
            r#"<defs><linearGradient id="qr-gradient" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient></defs>"#,
            style.fg_color, grad_end
        ));
    }

    // Background rect
    svg.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{}"/>"#,
        style.bg_color
    ));

    // Render modules
    for r in 0..dim {
        for c in 0..dim {
            if matrix.get(r, c) {
                let x = ((c + quiet) as f64) * module_size;
                let y = ((r + quiet) as f64) * module_size;

                let is_finder =
                    (r < 7 && c < 7) || (r < 7 && c >= dim - 7) || (r >= dim - 7 && c < 7);

                if is_finder {
                    match style.finder_style {
                        FinderStyle::Circle => {
                            let cx = x + module_size / 2.0;
                            let cy = y + module_size / 2.0;
                            svg.push_str(&format!(
                                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
                                cx,
                                cy,
                                module_size * 0.48,
                                fill_style
                            ));
                        }
                        FinderStyle::Rounded => {
                            svg.push_str(&format!(
                                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" ry="4" fill="{}"/>"#,
                                x, y, module_size, module_size, fill_style
                            ));
                        }
                        FinderStyle::Square => {
                            svg.push_str(&format!(
                                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
                                x, y, module_size, module_size, fill_style
                            ));
                        }
                    }
                } else {
                    match style.module_shape {
                        ModuleShape::Square => {
                            svg.push_str(&format!(
                                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
                                x, y, module_size, module_size, fill_style
                            ));
                        }
                        ModuleShape::Rounded => {
                            svg.push_str(&format!(
                                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="3" ry="3" fill="{}"/>"#,
                                x, y, module_size, module_size, fill_style
                            ));
                        }
                        ModuleShape::Dots => {
                            let cx = x + module_size / 2.0;
                            let cy = y + module_size / 2.0;
                            let rad = module_size * 0.45;
                            svg.push_str(&format!(
                                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
                                cx, cy, rad, fill_style
                            ));
                        }
                        ModuleShape::Diamond => {
                            let mx = x + module_size / 2.0;
                            let my = y + module_size / 2.0;
                            svg.push_str(&format!(
                                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}"/>"#,
                                mx, y, x + module_size, my, mx, y + module_size, x, my, fill_style
                            ));
                        }
                    }
                }
            }
        }
    }

    // Render center logo/text badge if provided
    if let Some(ref text) = style.center_logo_text {
        if !text.trim().is_empty() {
            let center_box = (view_size * 0.24).max(30.0);
            let cx = (view_size - center_box) / 2.0;
            let cy = (view_size - center_box) / 2.0;

            svg.push_str(&format!(
                r#"<rect x="{0:.1}" y="{1:.1}" width="{2:.1}" height="{2:.1}" rx="6" fill="{3}" stroke="{4}" stroke-width="2"/>"#,
                cx, cy, center_box, style.bg_color, style.fg_color
            ));
            svg.push_str(&format!(
                r#"<text x="{0:.1}" y="{1:.1}" font-family="system-ui, sans-serif" font-weight="bold" font-size="{2:.1}" fill="{3}" text-anchor="middle" dominant-baseline="central">{4}</text>"#,
                view_size / 2.0,
                view_size / 2.0,
                center_box * 0.35,
                style.fg_color,
                text.trim()
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Compute IEEE 802.3 CRC-32 checksum
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

/// Render QrMatrix as RFC 2083 PNG byte stream (using `flate2`)
pub fn render_png(matrix: &QrMatrix, style: &QrStyleConfig) -> Result<Vec<u8>, String> {
    let dim = matrix.dimension;
    let quiet = style.quiet_zone_modules as usize;
    let total_modules = dim + quiet * 2;
    let scale = (style.size_px as usize / total_modules).max(1);
    let img_size = total_modules * scale;

    let (fr, fg, fb) = parse_hex_color(&style.fg_color);
    let (br, bg, bb) = parse_hex_color(&style.bg_color);

    // Build raw RGB scanlines with PNG filter byte 0 (None) at the start of each row
    let row_stride = 1 + img_size * 3;
    let mut raw_scanlines = Vec::with_capacity(row_stride * img_size);

    for img_y in 0..img_size {
        raw_scanlines.push(0); // Filter type None
        let mod_y = img_y / scale;

        for img_x in 0..img_size {
            let mod_x = img_x / scale;
            let is_dark =
                if mod_y >= quiet && mod_y < quiet + dim && mod_x >= quiet && mod_x < quiet + dim {
                    matrix.get(mod_y - quiet, mod_x - quiet)
                } else {
                    false
                };

            let (r, g, b) = if is_dark { (fr, fg, fb) } else { (br, bg, bb) };
            raw_scanlines.extend_from_slice(&[r, g, b]);
        }
    }

    // Compress IDAT via flate2 zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&raw_scanlines)
        .map_err(|e| format!("PNG zlib compression error: {}", e))?;
    let compressed_idat = encoder
        .finish()
        .map_err(|e| format!("PNG zlib finish error: {}", e))?;

    // Assemble complete PNG format: Signature + IHDR + IDAT + IEND
    let mut png = Vec::with_capacity(compressed_idat.len() + 128);
    // 1. Signature
    png.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);

    // 2. IHDR Chunk
    let mut ihdr_data = Vec::with_capacity(13);
    ihdr_data.extend_from_slice(&(img_size as u32).to_be_bytes()); // Width
    ihdr_data.extend_from_slice(&(img_size as u32).to_be_bytes()); // Height
    ihdr_data.push(8); // Bit depth
    ihdr_data.push(2); // Color type: Truecolor (RGB)
    ihdr_data.push(0); // Compression method (zlib)
    ihdr_data.push(0); // Filter method
    ihdr_data.push(0); // Interlace method

    write_png_chunk(&mut png, b"IHDR", &ihdr_data);

    // 3. IDAT Chunk
    write_png_chunk(&mut png, b"IDAT", &compressed_idat);

    // 4. IEND Chunk
    write_png_chunk(&mut png, b"IEND", &[]);

    Ok(png)
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);

    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    let crc_val = crc32(&crc_input);
    out.extend_from_slice(&crc_val.to_be_bytes());
}

// -----------------------------------------------------------------------------
// 5. Image & PNG QR Decoder Pipeline
// -----------------------------------------------------------------------------

/// Decompresses and extracts raw pixel buffer from PNG bytes
pub fn decode_png_to_rgb(png_bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    if png_bytes.len() < 8 || png_bytes[0..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Err("Invalid PNG file signature".to_string());
    }

    let mut offset = 8;
    let mut width = 0usize;
    let mut height = 0usize;
    let mut color_type = 0u8;
    let mut idat_data = Vec::new();

    while offset + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[offset],
            png_bytes[offset + 1],
            png_bytes[offset + 2],
            png_bytes[offset + 3],
        ]) as usize;
        let chunk_type = &png_bytes[offset + 4..offset + 8];
        offset += 8;

        if offset + length + 4 > png_bytes.len() {
            return Err("Premature end of PNG chunk data".to_string());
        }

        let data = &png_bytes[offset..offset + length];
        offset += length + 4; // skip data + CRC

        if chunk_type == b"IHDR" {
            if data.len() < 13 {
                return Err("Corrupt IHDR chunk".to_string());
            }
            width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
            let bit_depth = data[8];
            color_type = data[9];
            if bit_depth != 8 {
                return Err(format!("Unsupported PNG bit depth {}", bit_depth));
            }
        } else if chunk_type == b"IDAT" {
            idat_data.extend_from_slice(data);
        } else if chunk_type == b"IEND" {
            break;
        }
    }

    if width == 0 || height == 0 || idat_data.is_empty() {
        return Err("Incomplete PNG missing valid dimensions or IDAT chunks".to_string());
    }

    // Decompress IDAT
    let mut decoder = ZlibDecoder::new(&idat_data[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Failed to decompress PNG IDAT: {}", e))?;

    // Unfilter scanlines into RGB
    let bytes_per_pixel = match color_type {
        0 => 1, // Grayscale
        2 => 3, // RGB
        6 => 4, // RGBA
        _ => return Err(format!("Unsupported PNG color type {}", color_type)),
    };

    let expected_row_stride = 1 + width * bytes_per_pixel;
    if decompressed.len() < expected_row_stride * height {
        return Err("Decompressed scanline buffer smaller than expected dimensions".to_string());
    }

    let mut rgb_pixels = Vec::with_capacity(width * height * 3);
    let mut prev_row = vec![0u8; width * bytes_per_pixel];

    for y in 0..height {
        let row_start = y * expected_row_stride;
        let filter = decompressed[row_start];
        let row_data = &decompressed[row_start + 1..row_start + expected_row_stride];

        let mut reconstructed_row = vec![0u8; row_data.len()];
        for i in 0..row_data.len() {
            let left = if i >= bytes_per_pixel {
                reconstructed_row[i - bytes_per_pixel]
            } else {
                0
            };
            let up = prev_row[i];

            let raw = match filter {
                0 => row_data[i],                                                     // None
                1 => row_data[i].wrapping_add(left),                                  // Sub
                2 => row_data[i].wrapping_add(up),                                    // Up
                3 => row_data[i].wrapping_add(((left as u16 + up as u16) / 2) as u8), // Average
                4 => {
                    // Paeth predictor
                    let p = left as i32 + up as i32
                        - (if i >= bytes_per_pixel {
                            prev_row[i - bytes_per_pixel] as i32
                        } else {
                            0
                        });
                    let pa = (p - left as i32).abs();
                    let pb = (p - up as i32).abs();
                    let pc = (p
                        - (if i >= bytes_per_pixel {
                            prev_row[i - bytes_per_pixel] as i32
                        } else {
                            0
                        }))
                    .abs();
                    let pr = if pa <= pb && pa <= pc {
                        left
                    } else if pb <= pc {
                        up
                    } else {
                        prev_row[i - bytes_per_pixel]
                    };
                    row_data[i].wrapping_add(pr)
                }
                _ => row_data[i],
            };
            reconstructed_row[i] = raw;
        }

        // Convert scanline to RGB
        for x in 0..width {
            let idx = x * bytes_per_pixel;
            let (r, g, b) = match color_type {
                0 => {
                    let gray = reconstructed_row[idx];
                    (gray, gray, gray)
                }
                2 => (
                    reconstructed_row[idx],
                    reconstructed_row[idx + 1],
                    reconstructed_row[idx + 2],
                ),
                6 => (
                    reconstructed_row[idx],
                    reconstructed_row[idx + 1],
                    reconstructed_row[idx + 2],
                ),
                _ => (0, 0, 0),
            };
            rgb_pixels.extend_from_slice(&[r, g, b]);
        }

        prev_row = reconstructed_row;
    }

    Ok((width, height, rgb_pixels))
}

/// Decodes QR matrix from raw RGB pixels by binarizing and sampling modules
#[allow(clippy::needless_range_loop)]
pub fn decode_qr_from_rgb(
    width: usize,
    height: usize,
    rgb: &[u8],
) -> Result<(String, u8, QrEccLevel, usize), String> {
    if width < 21 || height < 21 || rgb.len() < width * height * 3 {
        return Err("Input frame dimensions insufficient for QR decoding".to_string());
    }

    // 1. Convert to grayscale & compute average luminance threshold
    let mut total_lum = 0u64;
    let pixel_count = width * height;
    let mut gray = Vec::with_capacity(pixel_count);

    for chunk in rgb.chunks_exact(3) {
        let y = (chunk[0] as u32 * 299 + chunk[1] as u32 * 587 + chunk[2] as u32 * 114) / 1000;
        gray.push(y as u8);
        total_lum += y as u64;
    }
    let threshold = (total_lum / pixel_count as u64) as u8;

    // Detect if image is inverted (dark background with light modules)
    let corner_lum = (gray[0] as u32
        + gray[width.saturating_sub(1)] as u32
        + gray[(height.saturating_sub(1)) * width] as u32
        + gray[pixel_count.saturating_sub(1)] as u32)
        / 4;
    let is_inverted = (corner_lum as u8) < threshold;

    let is_module_dark = |pixel: u8| -> bool {
        if is_inverted {
            pixel >= threshold
        } else {
            pixel < threshold
        }
    };

    // 2. Locate active bounding box of dark modules
    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = height;
    let mut max_y = 0;

    for y in 0..height {
        for x in 0..width {
            if is_module_dark(gray[y * width + x]) {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }

    if max_x <= min_x || max_y <= min_y {
        return Err("No dark QR modules detected in image".to_string());
    }

    let qr_w = max_x - min_x + 1;
    let qr_h = max_y - min_y + 1;

    // 3. Estimate version by measuring top-left finder pattern width (7 modules)
    let mut finder_run = 0usize;
    while min_x + finder_run < width && is_module_dark(gray[min_y * width + (min_x + finder_run)]) {
        finder_run += 1;
    }
    let mod_size = if finder_run >= 7 {
        finder_run as f64 / 7.0
    } else {
        qr_w as f64 / 21.0
    };

    let est_dim = ((qr_w as f64 / mod_size) + 0.5) as usize;
    let candidate_version = ((est_dim.saturating_sub(17)) / 4).clamp(1, 10) as u8;
    let best_dim = 17 + (candidate_version as usize) * 4;

    // 4. Sample grid at module centers
    let mut sampled_modules = vec![false; best_dim * best_dim];
    let mod_w = qr_w as f64 / best_dim as f64;
    let mod_h = qr_h as f64 / best_dim as f64;

    for r in 0..best_dim {
        for c in 0..best_dim {
            let sample_x = min_x + ((c as f64 + 0.5) * mod_w) as usize;
            let sample_y = min_y + ((r as f64 + 0.5) * mod_h) as usize;
            if sample_x < width && sample_y < height {
                sampled_modules[r * best_dim + c] =
                    is_module_dark(gray[sample_y * width + sample_x]);
            }
        }
    }

    // 5. Decode format information from top-left area
    let mut raw_bch = 0u32;
    let format_coords = [
        (8, 0),
        (8, 1),
        (8, 2),
        (8, 3),
        (8, 4),
        (8, 5),
        (8, 7),
        (8, 8),
        (7, 8),
        (5, 8),
        (4, 8),
        (3, 8),
        (2, 8),
        (1, 8),
        (0, 8),
    ];
    for (i, &(r, c)) in format_coords.iter().enumerate() {
        if sampled_modules[r * best_dim + c] {
            raw_bch |= 1 << (14 - i);
        }
    }

    let unmasked_format = (raw_bch ^ 0x5412) >> 10;
    let ecc_bits = ((unmasked_format >> 3) & 0b11) as u8;
    let mask_pattern = (unmasked_format & 0b111) as u8;
    let ecc_level = QrEccLevel::from_format_bits(ecc_bits);

    // 6. Unmask sampled modules and extract data codewords
    let mut clean_matrix = QrMatrix::new(candidate_version, ecc_level);
    clean_matrix.place_finders();
    clean_matrix.place_alignment_patterns();
    clean_matrix.place_timing_patterns();
    clean_matrix.reserve_format_and_dark_module();

    for r in 0..best_dim {
        for c in 0..best_dim {
            let mut val = sampled_modules[r * best_dim + c];
            if !clean_matrix.is_func(r, c) && is_masked(mask_pattern, r, c) {
                val = !val;
            }
            clean_matrix.set(r, c, val);
        }
    }

    // 7. Extract data bits in zigzag tracks
    let mut extracted_bits = Vec::new();
    let mut col = (best_dim - 1) as isize;
    let mut upward = true;

    while col > 0 {
        if col == 6 {
            col -= 1;
        }
        let rows: Vec<usize> = if upward {
            (0..best_dim).rev().collect()
        } else {
            (0..best_dim).collect()
        };

        for row in rows {
            for c in [col, col - 1] {
                let col_u = c as usize;
                if !clean_matrix.is_func(row, col_u) {
                    extracted_bits.push(clean_matrix.get(row, col_u));
                }
            }
        }
        upward = !upward;
        col -= 2;
    }

    // 8. Convert bits to codewords and de-interleave across blocks
    let spec = QrVersionSpec::lookup(candidate_version, ecc_level).ok_or_else(|| {
        format!(
            "Invalid version/ecc: V{} {:?}",
            candidate_version, ecc_level
        )
    })?;
    let data_len = spec.total_codewords - spec.ec_codewords;
    let block_count = spec.blocks;
    let data_per_block = data_len / block_count;

    let mut raw_codewords = Vec::with_capacity(extracted_bits.len() / 8);
    for chunk in extracted_bits.chunks_exact(8) {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            if b {
                byte |= 1 << (7 - i);
            }
        }
        raw_codewords.push(byte);
    }

    // De-interleave data blocks
    let mut data_codewords = vec![0u8; data_len];
    if block_count > 1 {
        for i in 0..data_per_block {
            for b in 0..block_count {
                let interleaved_idx = i * block_count + b;
                if interleaved_idx < raw_codewords.len() {
                    data_codewords[b * data_per_block + i] = raw_codewords[interleaved_idx];
                }
            }
        }
    } else {
        let copy_len = data_len.min(raw_codewords.len());
        data_codewords[..copy_len].copy_from_slice(&raw_codewords[..copy_len]);
    }

    // Reconstruct sequential bitstream
    let mut data_bits = Vec::with_capacity(data_len * 8);
    for &byte in &data_codewords {
        for bit_i in (0..8).rev() {
            data_bits.push((byte >> bit_i) & 1 == 1);
        }
    }

    if data_bits.len() < 12 {
        return Err("Bitstream too short to contain valid QR payload".to_string());
    }

    // Verify Byte Mode (0100)
    let mode = (if data_bits[0] { 8 } else { 0 })
        | (if data_bits[1] { 4 } else { 0 })
        | (if data_bits[2] { 2 } else { 0 })
        | (if data_bits[3] { 1 } else { 0 });

    if mode != 0b0100 {
        return Err(format!("Unsupported QR mode indicator {:#06b}", mode));
    }

    let count_bits = if candidate_version <= 9 { 8 } else { 16 };
    let mut count = 0usize;
    for i in 0..count_bits {
        if data_bits[4 + i] {
            count |= 1 << (count_bits - 1 - i);
        }
    }

    let start_bit = 4 + count_bits;
    if data_bits.len() < start_bit + count * 8 {
        return Err("Truncated payload data bits".to_string());
    }

    let mut payload_bytes = Vec::with_capacity(count);
    for b in 0..count {
        let mut byte = 0u8;
        for bit_i in 0..8 {
            if data_bits[start_bit + b * 8 + bit_i] {
                byte |= 1 << (7 - bit_i);
            }
        }
        payload_bytes.push(byte);
    }

    let text = String::from_utf8(payload_bytes)
        .map_err(|e| format!("Decoded QR bytes are not valid UTF-8: {}", e))?;

    Ok((text, candidate_version, ecc_level, best_dim))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_generation_and_masks() {
        let payload = b"https://rustapps.offline";
        let matrix =
            generate_qr_matrix(payload, QrEccLevel::Medium).expect("Should generate QR matrix");
        assert_eq!(matrix.version, 2);
        assert_eq!(matrix.dimension, 25);
        assert!(matrix.mask_pattern < 8);
        assert!(matrix.modules.iter().any(|&m| m));
    }

    #[test]
    fn test_svg_and_png_rendering() {
        let payload = b"WIFI:T:WPA;S:HomeMesh;P:SuperSecret;;";
        let matrix = generate_qr_matrix(payload, QrEccLevel::High).expect("Should generate QR");
        let style = QrStyleConfig {
            ecc_level: QrEccLevel::High,
            module_shape: ModuleShape::Dots,
            finder_style: FinderStyle::Circle,
            fg_color: "#1E293B".to_string(),
            fg_gradient_end: Some("#3B82F6".to_string()),
            bg_color: "#FFFFFF".to_string(),
            quiet_zone_modules: 4,
            size_px: 256,
            center_logo_text: Some("WIFI".to_string()),
        };

        let svg = render_svg(&matrix, &style);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("url(#qr-gradient)"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("WIFI"));

        let png = render_png(&matrix, &style).expect("Should render PNG");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(png.len() > 500);

        let (contrast, inverted) = compute_contrast_ratio(&style.fg_color, &style.bg_color);
        assert!(contrast > 7.0);
        assert!(!inverted);
    }

    #[test]
    fn test_roundtrip_encode_and_decode() {
        let payloads = [
            "https://offline-suite.local/vault",
            "BEGIN:VCARD\nVERSION:3.0\nFN:Linus Torvalds\nEND:VCARD",
            "Simple inventory tag #492819",
        ];

        for &p in &payloads {
            let matrix = generate_qr_matrix(p.as_bytes(), QrEccLevel::Medium)
                .expect("Should generate matrix");
            let style = QrStyleConfig::default();
            let png_bytes = render_png(&matrix, &style).expect("Should render PNG");

            let (w, h, rgb) = decode_png_to_rgb(&png_bytes).expect("Should decode PNG to RGB");
            let (decoded_text, version, ecc, dim) =
                decode_qr_from_rgb(w, h, &rgb).expect("Should decode QR from RGB");

            assert_eq!(decoded_text, p);
            assert_eq!(version, matrix.version);
            assert_eq!(ecc, matrix.ecc_level);
            assert_eq!(dim, matrix.dimension);
        }
    }
}
