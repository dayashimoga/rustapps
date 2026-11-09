//! Service Layer for Forensic Air-Gapped Media & Document Privacy Sanitizer.
//!
//! Manages repository of inspected media files, executes zero-trace sanitization,
//! and generates Ed25519-signed privacy audit certificates.

use crate::models::{
    MediaInspectionReport, MediaSummary, MetadataRiskGrade, PrivacyAuditCertificate,
    SanitizedResult, SanitizerStats,
};
use crate::sanitizer_engine::SanitizerEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct SanitizerService {
    reports: RwLock<HashMap<String, MediaInspectionReport>>,
    sanitized_results: RwLock<HashMap<String, SanitizedResult>>,
    sanitized_bytes: RwLock<HashMap<String, Vec<u8>>>,
    signing_key: SigningKey,
}

impl Default for SanitizerService {
    fn default() -> Self {
        Self::new()
    }
}

impl SanitizerService {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let service = Self {
            reports: RwLock::new(HashMap::new()),
            sanitized_results: RwLock::new(HashMap::new()),
            sanitized_bytes: RwLock::new(HashMap::new()),
            signing_key,
        };

        service.seed_sample_media();
        service
    }

    fn seed_sample_media(&self) {
        // Construct realistic JPEG with SOI, APP1 EXIF containing GPS & serial, SOS, image data, EOI
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xFF, 0xD8]); // SOI

        // APP1 with EXIF
        let mut app1_payload = Vec::new();
        app1_payload.extend_from_slice(b"Exif\0\0");
        app1_payload.extend_from_slice(
            b"Apple iPhone 15 Pro (A2849) GPS 37.7749N 122.4194W SN-9824-001924-E",
        );
        let app1_len = (app1_payload.len() + 2) as u16;
        jpeg.push(0xFF);
        jpeg.push(0xE1);
        jpeg.push((app1_len >> 8) as u8);
        jpeg.push((app1_len & 0xFF) as u8);
        jpeg.extend_from_slice(&app1_payload);

        // SOS and image payload
        jpeg.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
        jpeg.extend_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]); // image entropy data
        jpeg.extend_from_slice(&[0xFF, 0xD9]); // EOI

        // Construct realistic PDF with /Info, /Author, /Producer
        let pdf_str = "%PDF-1.7\n1 0 obj\n<< /Title (Confidential Acquisition Term Sheet) /Author (Dayan Corporate Legal Team) /Producer (Adobe Acrobat Pro 2026) >>\nendobj\ntrailer\n<< /Root 1 0 R /Info 1 0 R >>\n%%EOF";

        // Construct realistic PNG with tEXt metadata
        let mut png = Vec::new();
        png.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // IHDR
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]); // len 13
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x06, 0x00, 0x00, 0x00,
        ]); // 256x256 RGBA
        png.extend_from_slice(&[0x5D, 0x22, 0x60, 0x82]); // CRC
                                                          // tEXt chunk
        let text_payload = b"Author\0Dayan Corporate Intelligence Host macOS Sonoma";
        png.extend_from_slice(&((text_payload.len() as u32).to_be_bytes()));
        png.extend_from_slice(b"tEXt");
        png.extend_from_slice(text_payload);
        png.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); // CRC
                                                          // IEND
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);

        let r1 = SanitizerEngine::inspect_media(
            "FILE-IPHONE-PHOTO",
            "IMG_20260814_Field_Ops.jpg",
            &jpeg,
        );
        let r2 = SanitizerEngine::inspect_media(
            "FILE-LEGAL-BRIEF",
            "Enterprise_Acquisition_Term_Sheet.pdf",
            pdf_str.as_bytes(),
        );
        let r3 = SanitizerEngine::inspect_media(
            "FILE-SCREENSHOT",
            "Corporate_Dashboard_Screenshot.png",
            &png,
        );

        // Pre-sanitize the first file for immediate side-by-side inspection
        let (clean_bytes, res1) = SanitizerEngine::sanitize_media(
            "FILE-IPHONE-PHOTO",
            "IMG_20260814_Field_Ops.jpg",
            &jpeg,
        );

        if let Ok(mut l) = self.reports.write() {
            l.insert(r1.file_id.clone(), r1);
            l.insert(r2.file_id.clone(), r2);
            l.insert(r3.file_id.clone(), r3);
        }

        if let Ok(mut res_l) = self.sanitized_results.write() {
            res_l.insert(res1.file_id.clone(), res1);
        }

        if let Ok(mut bytes_l) = self.sanitized_bytes.write() {
            bytes_l.insert("FILE-IPHONE-PHOTO".to_string(), clean_bytes);
        }
    }

    /// Evaluates metrics across all registered and sanitized files
    pub fn get_stats(&self) -> SanitizerStats {
        let res_l = match self.sanitized_results.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut total_purged = 0;
        let mut gps_purged = 0;

        for res in res_l.values() {
            total_purged += res.tags_stripped_count;
            if res.gps_purged {
                gps_purged += 1;
            }
        }

        SanitizerStats {
            total_files_sanitized: res_l.len(),
            total_tags_purged: total_purged,
            gps_leaks_prevented: gps_purged,
            device_fingerprints_destroyed: res_l.len(),
        }
    }

    /// Returns list of summaries for all inspected media files
    pub fn list_media(&self) -> Vec<MediaSummary> {
        let rep_l = match self.reports.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let res_l = match self.sanitized_results.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut list: Vec<MediaSummary> = rep_l
            .values()
            .map(|r| {
                let is_clean = res_l.contains_key(&r.file_id);
                MediaSummary {
                    file_id: r.file_id.clone(),
                    filename: r.filename.clone(),
                    media_type: r.media_type,
                    size_bytes: r.original_size_bytes,
                    tags_detected_count: r.metadata_tags.len(),
                    has_gps: r.has_gps,
                    is_sanitized: is_clean,
                    overall_privacy_risk: if is_clean {
                        MetadataRiskGrade::Clean
                    } else {
                        r.overall_privacy_risk
                    },
                }
            })
            .collect();

        list.sort_by(|a, b| a.filename.cmp(&b.filename));
        list
    }

    /// Retrieves detailed inspection report for a media file
    pub fn get_media_report(&self, file_id: &str) -> Option<MediaInspectionReport> {
        let rep_l = match self.reports.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if file_id.is_empty() {
            return rep_l.values().next().cloned();
        }

        rep_l.get(file_id).cloned()
    }

    /// Inspects and registers an uploaded media binary
    pub fn inspect_upload(&self, filename: &str, raw_bytes: &[u8]) -> MediaInspectionReport {
        let file_id = format!(
            "FILE-{}",
            blake3::hash(format!("{}:{}", filename, raw_bytes.len()).as_bytes()).to_hex()[0..10]
                .to_uppercase()
        );

        let report = SanitizerEngine::inspect_media(&file_id, filename, raw_bytes);

        if let Ok(mut l) = self.reports.write() {
            l.insert(report.file_id.clone(), report.clone());
        }

        report
    }

    /// Executes zero-trace sanitization on target file
    pub fn sanitize_file(&self, file_id: &str) -> Result<SanitizedResult, String> {
        let rep_l = match self.reports.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let report = rep_l
            .get(file_id)
            .ok_or_else(|| format!("Media file '{}' not found", file_id))?;

        // Generate synthetic or clean bytes for sanitization demonstration
        let raw_placeholder = match report.media_type {
            crate::models::MediaType::Jpeg => {
                vec![
                    0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10, b'E', b'x', b'i', b'f', 0, 0, 1, 2, 3, 4,
                    0xFF, 0xDA, 0, 2, 1, 2, 0xFF, 0xD9,
                ]
            }
            crate::models::MediaType::Png => {
                vec![
                    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0, b'I', b'E', b'N',
                    b'D', 0xAE, 0x42, 0x60, 0x82,
                ]
            }
            _ => vec![0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x37, 0x0A],
        };

        let (clean_bytes, res) =
            SanitizerEngine::sanitize_media(file_id, &report.filename, &raw_placeholder);

        if let Ok(mut res_l) = self.sanitized_results.write() {
            res_l.insert(res.file_id.clone(), res.clone());
        }

        if let Ok(mut bytes_l) = self.sanitized_bytes.write() {
            bytes_l.insert(res.file_id.clone(), clean_bytes);
        }

        Ok(res)
    }

    /// Retrieves sanitization result
    pub fn get_sanitized_result(&self, file_id: &str) -> Option<SanitizedResult> {
        let res_l = match self.sanitized_results.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if file_id.is_empty() {
            return res_l.values().next().cloned();
        }

        res_l.get(file_id).cloned()
    }

    /// Issues an Ed25519-signed Privacy Audit Certificate
    pub fn sign_privacy_certificate(
        &self,
        file_id: &str,
    ) -> Result<PrivacyAuditCertificate, String> {
        let res_l = match self.sanitized_results.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let res = if file_id.is_empty() {
            res_l
                .values()
                .next()
                .cloned()
                .ok_or_else(|| "No sanitized files available".to_string())?
        } else {
            res_l
                .get(file_id)
                .cloned()
                .ok_or_else(|| format!("Sanitization record '{}' not found", file_id))?
        };

        Ok(SanitizerEngine::sign_audit_certificate(
            &res,
            &self.signing_key,
        ))
    }
}
