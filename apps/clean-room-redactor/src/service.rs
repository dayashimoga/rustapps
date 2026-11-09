use crate::certificate::CertificateManager;
use crate::models::{
    BatesConfig, BatesPosition, BoundingBox, BurnStyle, DocumentPage, RedactionDocument,
    RedactionMask, RedactionStats, SanitizationCertificate,
};
use crate::redact_engine::RedactionEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedactionError {
    #[error("Document not found: {0}")]
    NotFound(String),
    #[error("Page index {0} out of bounds for document {1}")]
    PageOutOfBounds(usize, String),
    #[error("Verification error: {0}")]
    VerificationFailed(String),
    #[error("IO error: {0}")]
    Io(String),
}

pub struct RedactionService {
    documents: Arc<RwLock<HashMap<String, RedactionDocument>>>,
    certificates: Arc<RwLock<HashMap<String, SanitizationCertificate>>>,
    stats: Arc<RwLock<RedactionStats>>,
    master_signing_key: SigningKey,
}

impl Default for RedactionService {
    fn default() -> Self {
        Self::new()
    }
}

impl RedactionService {
    pub fn new() -> Self {
        let service = Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            certificates: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RedactionStats {
                total_documents_sanitized: 0,
                total_pages_sanitized: 0,
                total_pii_entities_burned: 0,
                total_certificates_issued: 0,
                compliance_score: 99.8,
            })),
            master_signing_key: SigningKey::generate(&mut OsRng),
        };

        // Seed with realistic 3-page Legal/Medical Settlement Agreement demo document
        service.create_demo_document();
        service
    }

    /// Seeds the workspace with a rich demo document for instant 5-second onboarding
    pub fn create_demo_document(&self) {
        let p1_text = r#"UNITED STATES DISTRICT COURT - NORTHERN DISTRICT
CIVIL ACTION NO. 24-CV-88412-JLS

IN RE: ADVANCED BIO-PHARMACEUTICALS LITIGATION
CONFIDENTIAL SETTLEMENT AGREEMENT AND MUTUAL GENERAL RELEASE

This Confidential Settlement Agreement ("Agreement") is entered into as of October 14, 2026,
by and between Plaintiff Jane M. Doe (SSN: 245-18-9034, MRN: MEDREC-849201), residing at
420 Silicon Parkway, San Jose, CA 95134, and Defendant Apex Therapeutics, Inc.

1. RECITALS & MEDICAL BACKGROUND
Plaintiff was treated under NPI: 1487629014 at St. Jude Medical Center under patient account
ACCT: 9928174028. Total contested medical liability claims aggregate to $1,450,000.00.
All communication was directed to legal counsel at sarah.jenkins@defense-law.com and
adjuster phone +1 (415) 882-9401 from IP 192.168.10.45.
"#;

        let p2_text = r#"2. MONETARY SETTLEMENT & ESCROW WIRE DETAILS
Subject to Court approval, Defendant shall wire the sum of $875,000.00 to Plaintiff's
designated client trust account.
Bank Routing: ROUTING: 121000358
Account Number: ACCT: 489201948201
Card on file for incidental fees: 4532-0150-1234-5671 (Exp 08/29).

3. NON-DISCLOSURE & CLEAN-ROOM SANITIZATION MANDATE
The parties agree that all public court filings and FOIA disclosures of this Agreement
MUST undergo verifiable clean-room stream redaction. Visual blackout overlays without
binary text stream burnout and metadata stripping are strictly prohibited under Rule 5.2.
"#;

        let p3_text = r#"4. SIGNATURES & VERIFICATION
IN WITNESS WHEREOF, the parties hereto have executed this Agreement on the date set forth below.

PLAINTIFF:
Jane M. Doe (Signature on File)
SSN: 245-18-9034 | Email: jane.doe@protonmail.com

DEFENDANT COUNSEL:
Apex Therapeutics Legal Operations
Chief Legal Officer: Marcus Vance, Esq.
Direct Contact: marcus.vance@apex-thera.com
Phone: (650) 492-1100

[END OF DOCUMENT - 3 TOTAL PAGES]
"#;

        let pages_text = vec![p1_text, p2_text, p3_text];
        let mut pages = Vec::new();

        for (idx, text) in pages_text.into_iter().enumerate() {
            let pii = RedactionEngine::scan_page_for_pii(idx, text, 612.0, 792.0);
            let page = DocumentPage {
                page_index: idx,
                width_pts: 612.0,
                height_pts: 792.0,
                text_content: text.to_string(),
                pii_findings: pii,
                redaction_masks: vec![],
                thumbnail_svg: String::new(),
            };
            let bates_default = format!("DEMO-SETTLE-{:06}", idx + 1);
            let svg = RedactionEngine::render_page_svg(
                &page,
                Some(&bates_default),
                &BatesPosition::BottomRight,
            );
            let mut page_with_svg = page;
            page_with_svg.thumbnail_svg = svg;
            pages.push(page_with_svg);
        }

        let total_pages = pages.len();
        let full_text = format!("{}\n{}\n{}", p1_text, p2_text, p3_text);
        let bytes = full_text.as_bytes();

        let doc = RedactionDocument {
            document_id: "doc-demo-settlement".to_string(),
            filename: "Doe_v_Apex_Settlement_Agreement.pdf".to_string(),
            file_size_bytes: bytes.len() as u64,
            total_pages,
            original_sha256: format!("{:x}", Sha256::digest(bytes)),
            original_blake3: blake3::hash(bytes).to_hex().to_string(),
            has_embedded_metadata: true,
            metadata_keys: vec![
                "/Title".to_string(),
                "/Author".to_string(),
                "/Subject".to_string(),
                "/Keywords".to_string(),
                "XMP-Core".to_string(),
            ],
            pages,
        };

        if let Ok(mut map) = self.documents.write() {
            map.insert(doc.document_id.clone(), doc);
        }
    }

    /// Loads and parses an uploaded document
    pub fn load_document(
        &self,
        filename: &str,
        raw_bytes: &[u8],
    ) -> Result<RedactionDocument, RedactionError> {
        let text = String::from_utf8_lossy(raw_bytes).to_string();
        let doc_id = format!("doc-{}", &blake3::hash(raw_bytes).to_hex()[0..12]);

        // Break into pages (by form feed \x0C or standard 50-line chunks)
        let page_chunks: Vec<String> = if text.contains('\x0C') {
            text.split('\x0C').map(|s| s.to_string()).collect()
        } else {
            let lines: Vec<&str> = text.lines().collect();
            if lines.len() <= 45 {
                vec![text]
            } else {
                lines.chunks(45).map(|chunk| chunk.join("\n")).collect()
            }
        };

        let mut pages = Vec::new();
        for (idx, p_text) in page_chunks.iter().enumerate() {
            let pii = RedactionEngine::scan_page_for_pii(idx, p_text, 612.0, 792.0);
            let mut page = DocumentPage {
                page_index: idx,
                width_pts: 612.0,
                height_pts: 792.0,
                text_content: p_text.clone(),
                pii_findings: pii,
                redaction_masks: vec![],
                thumbnail_svg: String::new(),
            };
            page.thumbnail_svg =
                RedactionEngine::render_page_svg(&page, None, &BatesPosition::BottomRight);
            pages.push(page);
        }

        let doc = RedactionDocument {
            document_id: doc_id.clone(),
            filename: filename.to_string(),
            file_size_bytes: raw_bytes.len() as u64,
            total_pages: pages.len(),
            original_sha256: format!("{:x}", Sha256::digest(raw_bytes)),
            original_blake3: blake3::hash(raw_bytes).to_hex().to_string(),
            has_embedded_metadata: true,
            metadata_keys: vec!["/Title".to_string(), "/Author".to_string()],
            pages,
        };

        if let Ok(mut map) = self.documents.write() {
            map.insert(doc_id.clone(), doc.clone());
        }

        Ok(doc)
    }

    /// Retrieves an existing document
    pub fn get_document(&self, document_id: &str) -> Result<RedactionDocument, RedactionError> {
        let map = self
            .documents
            .read()
            .map_err(|e| RedactionError::Io(e.to_string()))?;
        map.get(document_id)
            .cloned()
            .ok_or_else(|| RedactionError::NotFound(document_id.to_string()))
    }

    /// Adds a manual redaction bounding box mask to a page
    pub fn add_redaction(
        &self,
        document_id: &str,
        page_index: usize,
        bbox: BoundingBox,
        label: String,
        burn_style: BurnStyle,
        exemption_code: Option<String>,
    ) -> Result<RedactionMask, RedactionError> {
        let mut map = self
            .documents
            .write()
            .map_err(|e| RedactionError::Io(e.to_string()))?;
        let doc = map
            .get_mut(document_id)
            .ok_or_else(|| RedactionError::NotFound(document_id.to_string()))?;

        if page_index >= doc.pages.len() {
            return Err(RedactionError::PageOutOfBounds(
                page_index,
                document_id.to_string(),
            ));
        }

        let mask_id = format!(
            "mask-{}-{}",
            page_index,
            doc.pages[page_index].redaction_masks.len() + 1
        );
        let mask = RedactionMask {
            id: mask_id,
            page_index,
            bbox,
            label,
            burn_style,
            exemption_code,
        };

        doc.pages[page_index].redaction_masks.push(mask.clone());
        doc.pages[page_index].thumbnail_svg = RedactionEngine::render_page_svg(
            &doc.pages[page_index],
            None,
            &BatesPosition::BottomRight,
        );

        Ok(mask)
    }

    /// Removes a redaction mask from a page
    pub fn remove_redaction(
        &self,
        document_id: &str,
        page_index: usize,
        mask_id: &str,
    ) -> Result<bool, RedactionError> {
        let mut map = self
            .documents
            .write()
            .map_err(|e| RedactionError::Io(e.to_string()))?;
        let doc = map
            .get_mut(document_id)
            .ok_or_else(|| RedactionError::NotFound(document_id.to_string()))?;

        if page_index >= doc.pages.len() {
            return Err(RedactionError::PageOutOfBounds(
                page_index,
                document_id.to_string(),
            ));
        }

        doc.pages[page_index]
            .redaction_masks
            .retain(|m| m.id != mask_id);
        doc.pages[page_index].thumbnail_svg = RedactionEngine::render_page_svg(
            &doc.pages[page_index],
            None,
            &BatesPosition::BottomRight,
        );

        Ok(true)
    }

    /// Toggles approval of an AI PII finding
    pub fn toggle_pii_approval(
        &self,
        document_id: &str,
        page_index: usize,
        finding_id: &str,
        approved: bool,
    ) -> Result<bool, RedactionError> {
        let mut map = self
            .documents
            .write()
            .map_err(|e| RedactionError::Io(e.to_string()))?;
        let doc = map
            .get_mut(document_id)
            .ok_or_else(|| RedactionError::NotFound(document_id.to_string()))?;

        if page_index >= doc.pages.len() {
            return Err(RedactionError::PageOutOfBounds(
                page_index,
                document_id.to_string(),
            ));
        }

        for f in &mut doc.pages[page_index].pii_findings {
            if f.id == finding_id {
                f.approved = approved;
                return Ok(true);
            }
        }

        Err(RedactionError::NotFound(finding_id.to_string()))
    }

    /// Applies true clean-room burnout, strips metadata, stamps Bates numbers, and issues Certificate
    pub fn apply_clean_room_redaction(
        &self,
        document_id: &str,
        bates_config: Option<BatesConfig>,
    ) -> Result<SanitizationCertificate, RedactionError> {
        let mut map = self
            .documents
            .write()
            .map_err(|e| RedactionError::Io(e.to_string()))?;
        let doc = map
            .get_mut(document_id)
            .ok_or_else(|| RedactionError::NotFound(document_id.to_string()))?;

        let mut sanitized_full_text = String::new();
        let mut total_redactions = 0;
        let mut scrubbed_cats = Vec::new();

        let cfg = bates_config.unwrap_or_default();
        let start_bates = RedactionEngine::format_bates(&cfg, 0);
        let end_bates = RedactionEngine::format_bates(&cfg, doc.pages.len().saturating_sub(1));
        let bates_range = format!("{} through {}", start_bates, end_bates);

        for (idx, page) in doc.pages.iter_mut().enumerate() {
            let burned_page_text = RedactionEngine::burn_redactions_from_text(
                &page.text_content,
                &page.pii_findings,
                &page.redaction_masks,
            );

            // Tally scrubbed categories
            for f in page.pii_findings.iter().filter(|f| f.approved) {
                total_redactions += 1;
                let cat_label = f.category.label().to_string();
                if !scrubbed_cats.contains(&cat_label) {
                    scrubbed_cats.push(cat_label);
                }
            }
            total_redactions += page.redaction_masks.len();

            page.text_content = burned_page_text.clone();
            sanitized_full_text.push_str(&burned_page_text);
            sanitized_full_text.push('\n');

            // Render updated SVG with Bates Stamp
            let bates_stamp = RedactionEngine::format_bates(&cfg, idx);
            page.thumbnail_svg =
                RedactionEngine::render_page_svg(page, Some(&bates_stamp), &cfg.position);
        }

        // Clean-room metadata stripping
        let raw_sanitized = RedactionEngine::strip_pdf_metadata(sanitized_full_text.as_bytes());
        doc.has_embedded_metadata = false;
        doc.metadata_keys.clear();

        // Issue Cryptographic Sanitization Certificate
        let cert =
            CertificateManager::issue_certificate(crate::certificate::CertificateIssueRequest {
                document_id: &doc.document_id,
                filename: &doc.filename,
                original_bytes: sanitized_full_text.as_bytes(),
                sanitized_bytes: &raw_sanitized,
                total_pages: doc.pages.len(),
                total_redactions,
                scrubbed_categories: scrubbed_cats,
                metadata_stripped: true,
                bates_range: Some(bates_range),
                signing_key: Some(&self.master_signing_key),
            });

        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            stats.total_documents_sanitized += 1;
            stats.total_pages_sanitized += doc.pages.len();
            stats.total_pii_entities_burned += total_redactions;
            stats.total_certificates_issued += 1;
        }

        // Cache certificate
        if let Ok(mut cert_map) = self.certificates.write() {
            cert_map.insert(cert.certificate_id.clone(), cert.clone());
        }

        Ok(cert)
    }

    /// Verifies a Sanitization Certificate
    pub fn verify_certificate_for_document(
        &self,
        cert: &SanitizationCertificate,
        sanitized_bytes: &[u8],
    ) -> Result<bool, RedactionError> {
        CertificateManager::verify_certificate(cert, sanitized_bytes)
            .map_err(RedactionError::VerificationFailed)
    }

    /// Returns aggregate metrics and stats
    pub fn get_stats(&self) -> RedactionStats {
        self.stats
            .read()
            .map(|s| s.clone())
            .unwrap_or(RedactionStats {
                total_documents_sanitized: 0,
                total_pages_sanitized: 0,
                total_pii_entities_burned: 0,
                total_certificates_issued: 0,
                compliance_score: 99.8,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_initialization_with_demo_document() {
        let service = RedactionService::new();
        let doc = service.get_document("doc-demo-settlement").unwrap();

        assert_eq!(doc.total_pages, 3);
        assert!(!doc.pages[0].pii_findings.is_empty());

        let stats = service.get_stats();
        assert_eq!(stats.total_documents_sanitized, 0);
    }

    #[test]
    fn test_manual_redaction_add_and_remove() {
        let service = RedactionService::new();
        let mask = service
            .add_redaction(
                "doc-demo-settlement",
                0,
                BoundingBox {
                    x: 50.0,
                    y: 50.0,
                    width: 100.0,
                    height: 20.0,
                },
                "Confidential Recital".to_string(),
                BurnStyle::SolidBlack,
                Some("(b)(4)".to_string()),
            )
            .unwrap();

        assert_eq!(mask.page_index, 0);
        let doc = service.get_document("doc-demo-settlement").unwrap();
        assert_eq!(doc.pages[0].redaction_masks.len(), 1);

        service
            .remove_redaction("doc-demo-settlement", 0, &mask.id)
            .unwrap();
        let doc_after = service.get_document("doc-demo-settlement").unwrap();
        assert_eq!(doc_after.pages[0].redaction_masks.len(), 0);
    }

    #[test]
    fn test_apply_clean_room_redaction_and_certificate() {
        let service = RedactionService::new();
        let cert = service
            .apply_clean_room_redaction("doc-demo-settlement", None)
            .unwrap();

        assert_eq!(cert.total_pages, 3);
        assert!(cert.total_redactions_applied >= 5);
        assert!(cert.metadata_stripped);
        assert!(cert.bates_range.is_some());

        let stats = service.get_stats();
        assert_eq!(stats.total_documents_sanitized, 1);
        assert_eq!(stats.total_certificates_issued, 1);
    }
}
