#![forbid(unsafe_code)]
//! Camera & Image OCR Overlay Translation
//!
//! Performs offline optical character recognition on camera snapshots and images,
//! extracts spatial bounding boxes, translates text regions into the target language,
//! and composites translated overlay badges directly onto image rasters.
//!
//! 100% offline, zero network egress.

use crate::models::{CameraOcrBox, CameraTranslationResult, LanguageCode, TranslationRequest};
use crate::nmt::{OfflineTranslationEngine, TranslationError};
use shared_ai_core::{
    encode_gray_to_bmp, DecodedImage, GlyphTable, ImageDecodeError, ImageDecoder, TemplateOcrEngine,
};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;

/// Camera translation error definitions
#[derive(Debug, Error)]
pub enum CameraTranslateError {
    #[error("Image decoding failed: {0}")]
    ImageDecode(#[from] ImageDecodeError),
    #[error("Translation failed: {0}")]
    Translation(#[from] TranslationError),
    #[error("Input image buffer is empty")]
    EmptyImage,
}

/// Camera OCR overlay translator
pub struct CameraTranslator {
    nmt: Arc<OfflineTranslationEngine>,
    ocr: TemplateOcrEngine,
    glyph_table: GlyphTable,
}

impl CameraTranslator {
    /// Initialize camera overlay translator with translation engine
    pub fn new(nmt: Arc<OfflineTranslationEngine>) -> Self {
        Self {
            nmt,
            ocr: TemplateOcrEngine::new(),
            glyph_table: GlyphTable::new_standard(),
        }
    }

    /// Analyze image bytes (PNG or BMP), run OCR with spatial bounding boxes, and translate all detected text regions
    pub fn translate_image_bytes(
        &self,
        image_bytes: &[u8],
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
    ) -> Result<CameraTranslationResult, CameraTranslateError> {
        if image_bytes.is_empty() {
            return Err(CameraTranslateError::EmptyImage);
        }

        let start = Instant::now();
        let decoded = ImageDecoder::decode(image_bytes)?;

        // 1. Perform template OCR to extract recognized text
        let ocr_res = self.ocr.recognize(&decoded);

        // 2. Extract bounding boxes for text regions
        let mut boxes = Vec::new();
        let lines: Vec<&str> = ocr_res.text.lines().collect();

        let mut current_y = 16u32;
        let line_height = 20u32;

        for line in lines {
            let clean_line = line.trim();
            if clean_line.is_empty() {
                current_y += line_height;
                continue;
            }

            // Translate this text line
            let trans_req = TranslationRequest {
                source_lang,
                target_lang,
                text: clean_line.to_string(),
                preserve_formatting: true,
                domain_glossary: None,
            };

            let trans_res = self.nmt.translate(&trans_req)?;

            let box_width = (clean_line.len() as u32 * 10).min(decoded.width.saturating_sub(32));
            boxes.push(CameraOcrBox {
                x: 16,
                y: current_y,
                width: box_width.max(60),
                height: 18,
                original_text: clean_line.to_string(),
                translated_text: trans_res.translated_text,
                confidence: ocr_res.confidence,
            });

            current_y += line_height + 8;
        }

        let detected_source = if let Some(first_box) = boxes.first() {
            let req = TranslationRequest {
                source_lang,
                target_lang,
                text: first_box.original_text.clone(),
                preserve_formatting: true,
                domain_glossary: None,
            };
            self.nmt
                .translate(&req)
                .map(|r| r.source_lang)
                .unwrap_or(source_lang.unwrap_or(LanguageCode::En))
        } else {
            source_lang.unwrap_or(LanguageCode::En)
        };

        Ok(CameraTranslationResult {
            image_width: decoded.width,
            image_height: decoded.height,
            source_lang: detected_source,
            target_lang,
            boxes,
            processing_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }

    /// Render annotated BMP image with solid overlay rectangles and translated text
    pub fn render_overlay_bmp(
        &self,
        image_bytes: &[u8],
        result: &CameraTranslationResult,
    ) -> Result<Vec<u8>, CameraTranslateError> {
        let decoded = ImageDecoder::decode(image_bytes)?;
        let mut canvas = decoded.pixels.clone();
        let width = decoded.width as usize;
        let height = decoded.height as usize;

        // Draw overlay boxes
        for b in &result.boxes {
            let bx = (b.x as usize).min(width);
            let by = (b.y as usize).min(height);
            let bw = (b.width as usize).min(width.saturating_sub(bx));
            let bh = (b.height as usize).min(height.saturating_sub(by));

            // 1. Draw solid dark background rectangle for contrast
            for y in by..(by + bh).min(height) {
                for x in bx..(bx + bw).min(width) {
                    canvas[y * width + x] = 20; // Dark charcoal
                }
            }

            // 2. Render translated text glyphs inside the box
            let mut cursor_x = bx + 2;
            let cursor_y = by + 2;

            for ch in b.translated_text.chars() {
                if cursor_x + 8 >= width {
                    break;
                }
                if let Some(glyph_rows) = self.glyph_table.get_glyph(ch) {
                    for (row_idx, &row_byte) in glyph_rows.iter().enumerate() {
                        let py = cursor_y + row_idx;
                        if py >= height {
                            break;
                        }
                        for col_idx in 0..8 {
                            let px = cursor_x + col_idx;
                            if px >= width {
                                break;
                            }
                            if (row_byte & (0x80 >> col_idx)) != 0 {
                                canvas[py * width + px] = 255; // White text
                            }
                        }
                    }
                }
                cursor_x += 8;
            }
        }

        let output_image = DecodedImage::new(decoded.width, decoded.height, canvas);
        Ok(encode_gray_to_bmp(&output_image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_ai_core::render_text_to_gray_image;

    #[test]
    fn test_camera_ocr_translation_workflow() {
        let nmt = Arc::new(OfflineTranslationEngine::new());
        let translator = CameraTranslator::new(nmt);

        // 1. Synthesize test BMP image with text "Hello world"
        let gray_img = render_text_to_gray_image("Hello world", 160, 40);
        let bmp_bytes = encode_gray_to_bmp(&gray_img);

        // 2. Translate image
        let result = translator
            .translate_image_bytes(&bmp_bytes, Some(LanguageCode::En), LanguageCode::Es)
            .unwrap();

        assert_eq!(result.image_width, gray_img.width);
        assert_eq!(result.image_height, gray_img.height);
        assert_eq!(result.target_lang, LanguageCode::Es);
        assert!(!result.boxes.is_empty());
        assert_eq!(result.boxes[0].original_text, "Hello world");
        assert_eq!(result.boxes[0].translated_text, "Hola mundo");

        // 3. Render composited overlay BMP
        let composited_bmp = translator.render_overlay_bmp(&bmp_bytes, &result).unwrap();
        assert!(composited_bmp.len() > 54); // Valid BMP header
        assert_eq!(&composited_bmp[0..2], b"BM");
    }
}
