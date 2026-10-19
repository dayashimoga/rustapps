#![forbid(unsafe_code)]
//! Instant Screenshot Search & Visual Perceptual Hashing Engine (ADR-0027)
//!
//! Combines 64-bit difference perceptual hashing (dHash) with OCR text extraction:
//! - Sub-microsecond Hamming distance similarity matching: `(h1 ^ h2).count_ones()`
//! - Scale- and compression-invariant visual fingerprints
//! - Dual-modality search: Visual similarity + OCR text keywords

use crate::models::{ScreenshotFingerprint, ScreenshotSearchResult};
use crate::ocr::{decode_to_grayscale, GrayImage, OfflineOcrEngine};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Standard dHash grid dimensions: 9 columns, 8 rows (producing 64 gradient bits)
pub const DHASH_WIDTH: u32 = 9;
pub const DHASH_HEIGHT: u32 = 8;

/// Default Hamming distance threshold for "visually similar" screenshots (approx 85% match)
pub const DEFAULT_SIMILARITY_THRESHOLD: u32 = 10;

/// In-memory catalog of screenshot fingerprints and indexed OCR text
pub struct ScreenshotCatalog {
    fingerprints: RwLock<HashMap<PathBuf, ScreenshotFingerprint>>,
}

impl Default for ScreenshotCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotCatalog {
    pub fn new() -> Self {
        Self {
            fingerprints: RwLock::new(HashMap::new()),
        }
    }

    /// Indexes a screenshot file by computing its dHash fingerprint and OCR text
    pub fn index_screenshot<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<ScreenshotFingerprint, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read image: {}", e))?;
        let ext = path
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let gray = decode_to_grayscale(&bytes, &ext)?;
        let dhash = compute_dhash(&gray);
        let dimensions = (gray.width, gray.height);

        // Run offline OCR extraction
        let ocr_result = OfflineOcrEngine::recognize_gray_image(&gray, std::time::Instant::now())
            .unwrap_or_else(|_| crate::models::OcrResult {
                text: String::new(),
                words: Vec::new(),
                confidence: 1.0,
                line_count: 0,
                processing_time_ms: 0.0,
            });

        let fp = ScreenshotFingerprint {
            path: path.to_path_buf(),
            dhash,
            ocr_text: ocr_result.text,
            dimensions,
        };

        if let Ok(mut lock) = self.fingerprints.write() {
            lock.insert(path.to_path_buf(), fp.clone());
        }

        Ok(fp)
    }

    /// Inserts a precomputed screenshot fingerprint
    pub fn insert_fingerprint(&self, fp: ScreenshotFingerprint) {
        if let Ok(mut lock) = self.fingerprints.write() {
            lock.insert(fp.path.clone(), fp);
        }
    }

    /// Returns the total count of indexed screenshots
    pub fn len(&self) -> usize {
        self.fingerprints.read().map(|l| l.len()).unwrap_or(0)
    }

    /// Returns whether the catalog is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Searches for screenshots visually similar to a target 64-bit dHash
    pub fn search_by_visual_hash(
        &self,
        target_hash: u64,
        max_distance: u32,
        limit: usize,
    ) -> Vec<ScreenshotSearchResult> {
        let lock = match self.fingerprints.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        let mut matches: Vec<ScreenshotSearchResult> = lock
            .values()
            .filter_map(|fp| {
                let dist = compute_hamming_distance(target_hash, fp.dhash);
                if dist <= max_distance {
                    let similarity_pct = ((64 - dist) as f32 / 64.0) * 100.0;
                    let snippet = if fp.ocr_text.is_empty() {
                        None
                    } else {
                        Some(fp.ocr_text.chars().take(120).collect())
                    };
                    Some(ScreenshotSearchResult {
                        path: fp.path.clone(),
                        dhash: fp.dhash,
                        hamming_distance: dist,
                        similarity_pct,
                        ocr_snippet: snippet,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort ascending by Hamming distance (closest visual match first)
        matches.sort_by_key(|m| m.hamming_distance);
        matches.truncate(limit);
        matches
    }

    /// Searches for screenshots containing matching OCR text keywords
    pub fn search_by_ocr_text(&self, query: &str, limit: usize) -> Vec<ScreenshotSearchResult> {
        let lock = match self.fingerprints.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        let query_lower = query.to_lowercase();
        let mut matches: Vec<ScreenshotSearchResult> = lock
            .values()
            .filter_map(|fp| {
                let text_lower = fp.ocr_text.to_lowercase();
                if text_lower.contains(&query_lower) {
                    let snippet = if fp.ocr_text.is_empty() {
                        None
                    } else {
                        Some(fp.ocr_text.chars().take(120).collect())
                    };
                    Some(ScreenshotSearchResult {
                        path: fp.path.clone(),
                        dhash: fp.dhash,
                        hamming_distance: 0,
                        similarity_pct: 100.0,
                        ocr_snippet: snippet,
                    })
                } else {
                    None
                }
            })
            .collect();

        matches.truncate(limit);
        matches
    }
}

// ---------------------------------------------------------------------------
// dHash Algorithm & Hamming Distance Calculation
// ---------------------------------------------------------------------------

/// Computes a 64-bit difference hash (dHash) from a grayscale image
pub fn compute_dhash(image: &GrayImage) -> u64 {
    if image.width < 2 || image.height < 1 {
        return 0;
    }

    // Downscale to 9x8 grid using box sampling
    let mut downscaled = [0u8; (DHASH_WIDTH * DHASH_HEIGHT) as usize];
    for dy in 0..DHASH_HEIGHT {
        let src_y = ((dy as f64 + 0.5) * (image.height as f64) / (DHASH_HEIGHT as f64)) as u32;
        let clamped_y = src_y.min(image.height - 1);
        for dx in 0..DHASH_WIDTH {
            let src_x = ((dx as f64 + 0.5) * (image.width as f64) / (DHASH_WIDTH as f64)) as u32;
            let clamped_x = src_x.min(image.width - 1);
            downscaled[(dy * DHASH_WIDTH + dx) as usize] = image.get_pixel(clamped_x, clamped_y);
        }
    }

    // Compute gradient: bit = 1 if left pixel < right pixel
    let mut hash = 0u64;
    for y in 0..DHASH_HEIGHT {
        for x in 0..DHASH_WIDTH - 1 {
            let left = downscaled[(y * DHASH_WIDTH + x) as usize];
            let right = downscaled[(y * DHASH_WIDTH + x + 1) as usize];
            if left < right {
                let bit_index = y * 8 + x;
                hash |= 1u64 << bit_index;
            }
        }
    }

    hash
}

/// Computes the bitwise Hamming distance between two 64-bit perceptual hashes
#[inline]
pub fn compute_hamming_distance(h1: u64, h2: u64) -> u32 {
    (h1 ^ h2).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dhash_identical_images_zero_distance() {
        let mut pixels = vec![0u8; 100];
        for (x, p) in pixels.iter_mut().enumerate().take(10) {
            *p = (x * 25) as u8;
        }
        let img1 = GrayImage::new(10, 10, pixels.clone());
        let img2 = GrayImage::new(10, 10, pixels);

        let h1 = compute_dhash(&img1);
        let h2 = compute_dhash(&img2);

        assert_eq!(compute_hamming_distance(h1, h2), 0);
    }

    #[test]
    fn test_dhash_different_images_high_distance() {
        // Gradient left to right
        let mut pixels1 = vec![0u8; 100];
        for y in 0..10 {
            for x in 0..10 {
                pixels1[y * 10 + x] = (x * 25) as u8;
            }
        }
        // Gradient right to left (opposite)
        let mut pixels2 = vec![0u8; 100];
        for y in 0..10 {
            for x in 0..10 {
                pixels2[y * 10 + x] = ((9 - x) * 25) as u8;
            }
        }

        let img1 = GrayImage::new(10, 10, pixels1);
        let img2 = GrayImage::new(10, 10, pixels2);

        let h1 = compute_dhash(&img1);
        let h2 = compute_dhash(&img2);

        let dist = compute_hamming_distance(h1, h2);
        assert!(
            dist >= 50,
            "Opposite gradient should produce high Hamming distance, got {}",
            dist
        );
    }

    #[test]
    fn test_screenshot_catalog_search() {
        let catalog = ScreenshotCatalog::new();
        let fp1 = ScreenshotFingerprint {
            path: PathBuf::from("/screenshots/terminal_err.png"),
            dhash: 0xAAAA_AAAA_AAAA_AAAA,
            ocr_text: "Connection refused on port 8080".to_string(),
            dimensions: (1920, 1080),
        };
        let fp2 = ScreenshotFingerprint {
            path: PathBuf::from("/screenshots/aws_console.png"),
            dhash: 0x5555_5555_5555_5555,
            ocr_text: "AWS Management Console S3 Bucket".to_string(),
            dimensions: (1920, 1080),
        };

        catalog.insert_fingerprint(fp1);
        catalog.insert_fingerprint(fp2);

        assert_eq!(catalog.len(), 2);

        // Search by OCR text keyword
        let ocr_hits = catalog.search_by_ocr_text("8080", 10);
        assert_eq!(ocr_hits.len(), 1);
        assert_eq!(
            ocr_hits[0].path,
            PathBuf::from("/screenshots/terminal_err.png")
        );

        // Search by visual hash (close to 0xAAAA_AAAA_AAAA_AAAA)
        let visual_hits = catalog.search_by_visual_hash(0xAAAA_AAAA_AAAA_AAAB, 5, 10);
        assert_eq!(visual_hits.len(), 1);
        assert_eq!(
            visual_hits[0].path,
            PathBuf::from("/screenshots/terminal_err.png")
        );
    }
}
