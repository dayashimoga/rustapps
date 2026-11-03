#![forbid(unsafe_code)]
//! Template and Topological 8x8 Glyph OCR Engine
//!
//! Provides genuine offline character classification and raster text rendering
//! using bitmap font glyph templates for numbers, uppercase/lowercase letters, and punctuation.

use crate::image_decoder::DecodedImage;
use std::collections::HashMap;

/// OCR character classification result
#[derive(Debug, Clone, PartialEq)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub word_count: usize,
}

/// 8x8 Bitmap Glyph Template Table
pub struct GlyphTable {
    glyphs: HashMap<char, [u8; 8]>,
}

impl Default for GlyphTable {
    fn default() -> Self {
        Self::new_standard()
    }
}

impl GlyphTable {
    pub fn new_standard() -> Self {
        let mut glyphs = HashMap::new();

        // 8x8 bitmapped representations: each byte is 1 row of 8 horizontal pixels
        glyphs.insert(' ', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        glyphs.insert('.', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18]);
        glyphs.insert(',', [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30]);
        glyphs.insert(':', [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00]);
        glyphs.insert(';', [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x30]);
        glyphs.insert('!', [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18]);
        glyphs.insert('?', [0x3c, 0x66, 0x0c, 0x18, 0x18, 0x00, 0x18, 0x18]);
        glyphs.insert('-', [0x00, 0x00, 0x00, 0x7e, 0x7e, 0x00, 0x00, 0x00]);
        glyphs.insert('_', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7e, 0x7e]);
        glyphs.insert('=', [0x00, 0x00, 0x7e, 0x00, 0x7e, 0x00, 0x00, 0x00]);
        glyphs.insert('+', [0x00, 0x18, 0x18, 0x7e, 0x7e, 0x18, 0x18, 0x00]);
        glyphs.insert('/', [0x02, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x40, 0x00]);

        // Digits 0-9
        glyphs.insert('0', [0x3c, 0x66, 0x6e, 0x76, 0x66, 0x66, 0x3c, 0x00]);
        glyphs.insert('1', [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7e, 0x00]);
        glyphs.insert('2', [0x3c, 0x66, 0x06, 0x0c, 0x18, 0x30, 0x7e, 0x00]);
        glyphs.insert('3', [0x3c, 0x66, 0x06, 0x1c, 0x06, 0x66, 0x3c, 0x00]);
        glyphs.insert('4', [0x0c, 0x1c, 0x3c, 0x6c, 0x7e, 0x0c, 0x0c, 0x00]);
        glyphs.insert('5', [0x7e, 0x60, 0x7c, 0x06, 0x06, 0x66, 0x3c, 0x00]);
        glyphs.insert('6', [0x3c, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x3c, 0x00]);
        glyphs.insert('7', [0x7e, 0x06, 0x0c, 0x18, 0x30, 0x30, 0x30, 0x00]);
        glyphs.insert('8', [0x3c, 0x66, 0x66, 0x3c, 0x66, 0x66, 0x3c, 0x00]);
        glyphs.insert('9', [0x3c, 0x66, 0x66, 0x3e, 0x06, 0x0c, 0x38, 0x00]);

        // Uppercase A-Z
        glyphs.insert('A', [0x3c, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00]);
        glyphs.insert('B', [0x7c, 0x66, 0x66, 0x7c, 0x66, 0x66, 0x7c, 0x00]);
        glyphs.insert('C', [0x3c, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3c, 0x00]);
        glyphs.insert('D', [0x78, 0x6c, 0x66, 0x66, 0x66, 0x6c, 0x78, 0x00]);
        glyphs.insert('E', [0x7e, 0x60, 0x60, 0x7c, 0x60, 0x60, 0x7e, 0x00]);
        glyphs.insert('F', [0x7e, 0x60, 0x60, 0x7c, 0x60, 0x60, 0x60, 0x00]);
        glyphs.insert('G', [0x3c, 0x66, 0x60, 0x6e, 0x66, 0x66, 0x3a, 0x00]);
        glyphs.insert('H', [0x66, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00]);
        glyphs.insert('I', [0x3c, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3c, 0x00]);
        glyphs.insert('J', [0x0e, 0x06, 0x06, 0x06, 0x06, 0x66, 0x3c, 0x00]);
        glyphs.insert('K', [0x66, 0x6c, 0x78, 0x70, 0x78, 0x6c, 0x66, 0x00]);
        glyphs.insert('L', [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7e, 0x00]);
        glyphs.insert('M', [0x63, 0x77, 0x7f, 0x6b, 0x63, 0x63, 0x63, 0x00]);
        glyphs.insert('N', [0x66, 0x76, 0x7e, 0x7e, 0x6e, 0x66, 0x66, 0x00]);
        glyphs.insert('O', [0x3c, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x00]);
        glyphs.insert('P', [0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60, 0x60, 0x00]);
        glyphs.insert('Q', [0x3c, 0x66, 0x66, 0x66, 0x6a, 0x6c, 0x36, 0x00]);
        glyphs.insert('R', [0x7c, 0x66, 0x66, 0x7c, 0x78, 0x6c, 0x66, 0x00]);
        glyphs.insert('S', [0x3c, 0x66, 0x60, 0x3c, 0x06, 0x66, 0x3c, 0x00]);
        glyphs.insert('T', [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00]);
        glyphs.insert('U', [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x00]);
        glyphs.insert('V', [0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x18, 0x00]);
        glyphs.insert('W', [0x63, 0x63, 0x63, 0x6b, 0x7f, 0x77, 0x63, 0x00]);
        glyphs.insert('X', [0x66, 0x66, 0x3c, 0x18, 0x3c, 0x66, 0x66, 0x00]);
        glyphs.insert('Y', [0x66, 0x66, 0x66, 0x3c, 0x18, 0x18, 0x18, 0x00]);
        glyphs.insert('Z', [0x7e, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x7e, 0x00]);

        // Lowercase a-z
        glyphs.insert('a', [0x00, 0x00, 0x3c, 0x06, 0x3e, 0x66, 0x3e, 0x00]);
        glyphs.insert('b', [0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x7c, 0x00]);
        glyphs.insert('c', [0x00, 0x00, 0x3c, 0x66, 0x60, 0x66, 0x3c, 0x00]);
        glyphs.insert('d', [0x06, 0x06, 0x3e, 0x66, 0x66, 0x66, 0x3e, 0x00]);
        glyphs.insert('e', [0x00, 0x00, 0x3c, 0x66, 0x7e, 0x60, 0x3c, 0x00]);
        glyphs.insert('f', [0x0e, 0x18, 0x3e, 0x18, 0x18, 0x18, 0x18, 0x00]);
        glyphs.insert('g', [0x00, 0x00, 0x3e, 0x66, 0x66, 0x3e, 0x06, 0x3c]);
        glyphs.insert('h', [0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x66, 0x00]);
        glyphs.insert('i', [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3c, 0x00]);
        glyphs.insert('j', [0x06, 0x00, 0x0e, 0x06, 0x06, 0x66, 0x3c, 0x00]);
        glyphs.insert('k', [0x60, 0x60, 0x66, 0x6c, 0x78, 0x6c, 0x66, 0x00]);
        glyphs.insert('l', [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3c, 0x00]);
        glyphs.insert('m', [0x00, 0x00, 0x66, 0x7f, 0x7f, 0x6b, 0x63, 0x00]);
        glyphs.insert('n', [0x00, 0x00, 0x7c, 0x66, 0x66, 0x66, 0x66, 0x00]);
        glyphs.insert('o', [0x00, 0x00, 0x3c, 0x66, 0x66, 0x66, 0x3c, 0x00]);
        glyphs.insert('p', [0x00, 0x00, 0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60]);
        glyphs.insert('q', [0x00, 0x00, 0x3e, 0x66, 0x66, 0x3e, 0x06, 0x06]);
        glyphs.insert('r', [0x00, 0x00, 0x7c, 0x66, 0x60, 0x60, 0x60, 0x00]);
        glyphs.insert('s', [0x00, 0x00, 0x3e, 0x60, 0x3c, 0x06, 0x7c, 0x00]);
        glyphs.insert('t', [0x18, 0x18, 0x7e, 0x18, 0x18, 0x18, 0x0e, 0x00]);
        glyphs.insert('u', [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3e, 0x00]);
        glyphs.insert('v', [0x00, 0x00, 0x66, 0x66, 0x66, 0x3c, 0x18, 0x00]);
        glyphs.insert('w', [0x00, 0x00, 0x63, 0x6b, 0x7f, 0x77, 0x63, 0x00]);
        glyphs.insert('x', [0x00, 0x00, 0x66, 0x3c, 0x18, 0x3c, 0x66, 0x00]);
        glyphs.insert('y', [0x00, 0x00, 0x66, 0x66, 0x66, 0x3e, 0x06, 0x3c]);
        glyphs.insert('z', [0x00, 0x00, 0x7e, 0x0c, 0x18, 0x30, 0x7e, 0x00]);

        Self { glyphs }
    }

    pub fn get_glyph(&self, c: char) -> Option<&[u8; 8]> {
        self.glyphs.get(&c)
    }

    pub fn all_glyphs(&self) -> &HashMap<char, [u8; 8]> {
        &self.glyphs
    }
}

/// Renders string into an authentic grayscale image canvas
#[allow(clippy::needless_range_loop)]
pub fn render_text_to_gray_image(text: &str, width: u32, height: u32) -> DecodedImage {
    let glyphs = GlyphTable::new_standard();
    let mut image = DecodedImage::new(width, height, vec![0u8; (width * height) as usize]);

    let mut cursor_x = 2u32;
    let mut cursor_y = 2u32;

    for c in text.chars() {
        if c == '\n' {
            cursor_x = 2;
            cursor_y += 10;
            continue;
        }

        if cursor_x + 8 > width {
            cursor_x = 2;
            cursor_y += 10;
        }

        if cursor_y + 8 > height {
            break;
        }

        if let Some(bitmap) = glyphs.get_glyph(c) {
            for row in 0..8 {
                let bits = bitmap[row];
                for col in 0..8 {
                    if (bits & (0x80 >> col)) != 0 {
                        image.set_pixel(cursor_x + col as u32, cursor_y + row as u32, 255);
                    }
                }
            }
        }

        cursor_x += 8;
    }

    image
}

/// Template matching OCR engine
pub struct TemplateOcrEngine {
    table: GlyphTable,
}

impl Default for TemplateOcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateOcrEngine {
    pub fn new() -> Self {
        Self {
            table: GlyphTable::new_standard(),
        }
    }

    /// Recognize rendered text from a grayscale DecodedImage
    #[allow(clippy::needless_range_loop)]
    pub fn recognize(&self, image: &DecodedImage) -> OcrResult {
        let mut recognized_text = String::new();
        let mut total_confidence = 0.0f32;
        let mut char_count = 0usize;

        // Scan in 8x8 character cells aligned to 8px grids or 10px line intervals
        let mut y = 2u32;
        while y + 8 <= image.height {
            let mut x = 2u32;
            let mut line_has_content = false;

            while x + 8 <= image.width {
                // Extract 8x8 candidate window
                let mut candidate = [0u8; 8];
                let mut has_ink = false;

                for row in 0..8 {
                    let mut row_bits = 0u8;
                    for col in 0..8 {
                        let px = image.get_pixel(x + col as u32, y + row as u32).unwrap_or(0);
                        if px > 100 {
                            row_bits |= 0x80 >> col;
                            has_ink = true;
                        }
                    }
                    candidate[row] = row_bits;
                }

                if !has_ink {
                    if line_has_content && !recognized_text.ends_with(' ') {
                        recognized_text.push(' ');
                    }
                    x += 8;
                    continue;
                }

                // Match candidate against glyph templates
                let mut best_char = '?';
                let mut best_score = 0.0f32;

                for (&ch, &template) in &self.table.glyphs {
                    if ch == ' ' {
                        continue;
                    }
                    let mut matching_bits = 0u32;
                    let mut total_active_bits = 0u32;

                    for r in 0..8 {
                        let c_byte = candidate[r];
                        let t_byte = template[r];
                        let same = !(c_byte ^ t_byte);
                        matching_bits += same.count_ones();
                        total_active_bits += (c_byte | t_byte).count_ones();
                    }

                    let score = if total_active_bits > 0 {
                        let intersection = matching_bits - (64 - total_active_bits);
                        (intersection as f32) / (total_active_bits as f32)
                    } else {
                        0.0
                    };

                    if score > best_score {
                        best_score = score;
                        best_char = ch;
                    }
                }

                if best_score > 0.60 {
                    recognized_text.push(best_char);
                    total_confidence += best_score;
                    char_count += 1;
                    line_has_content = true;
                }

                x += 8;
            }

            if line_has_content {
                recognized_text.push('\n');
            }
            y += 10;
        }

        let trimmed = recognized_text.trim().to_string();
        let word_count = trimmed.split_whitespace().count();
        let confidence = if char_count > 0 {
            total_confidence / char_count as f32
        } else {
            0.0
        };

        OcrResult {
            text: trimmed,
            confidence,
            word_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_and_ocr_roundtrip() {
        let input_text = "ERROR 404";
        let image = render_text_to_gray_image(input_text, 120, 20);

        let ocr = TemplateOcrEngine::new();
        let result = ocr.recognize(&image);

        assert_eq!(result.text, input_text);
        assert!(
            result.confidence > 0.90,
            "Expected high OCR confidence, got {}",
            result.confidence
        );
        assert_eq!(result.word_count, 2);
    }
}
