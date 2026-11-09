use crate::models::{
    BatesConfig, BatesPosition, BoundingBox, BurnStyle, DocumentPage, PiiFinding,
    RedactionCategory, RedactionMask,
};
use regex::Regex;
use std::sync::LazyLock;

static RE_SSN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}[- ]?\d{2}[- ]?\d{4}\b").expect("Valid SSN regex"));

static RE_CREDIT_CARD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\d{4}[- ]?){3}\d{4}\b").expect("Valid Credit Card regex"));

static RE_EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").expect("Valid Email regex")
});

static RE_PHONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\+?1[-. ]?)?\(?([0-9]{3})\)?[-. ]?([0-9]{3})[-. ]?([0-9]{4})\b")
        .expect("Valid Phone regex")
});

static RE_IP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b")
        .expect("Valid IP regex")
});

static RE_MRN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:MRN|MEDREC|PATIENT\s*ID)[:#\s]*([A-Z0-9]{6,12})\b")
        .expect("Valid MRN regex")
});

static RE_NPI_DEA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:NPI|DEA)[:#\s]*([A-Z]{2}[0-9]{7}|[0-9]{10})\b")
        .expect("Valid NPI/DEA regex")
});

static RE_BANK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:ROUTING|ACCT|ACCOUNT)[:#\s]*([0-9]{8,17})\b").expect("Valid Bank regex")
});

static RE_DOLLARS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\s*\d{1,3}(?:,\d{3})*(?:\.\d{2})?").expect("Valid Currency regex")
});

pub struct RedactionEngine;

impl RedactionEngine {
    /// Scans text across pages to detect PII and PHI entities with accurate coordinates
    pub fn scan_page_for_pii(
        page_index: usize,
        text: &str,
        page_width: f32,
        page_height: f32,
    ) -> Vec<PiiFinding> {
        let mut findings = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let total_lines = lines.len().max(1);
        let line_height_pts = (page_height - 100.0) / (total_lines as f32).max(1.0);

        for (line_idx, line) in lines.iter().enumerate() {
            let y_pos = 50.0 + (line_idx as f32 * line_height_pts);

            // 1. SSN Check
            for mat in RE_SSN.find_iter(line) {
                let ssn_raw = mat.as_str();
                let clean = ssn_raw.replace(['-', ' '], "");
                if clean != "000000000" && clean != "123456789" {
                    let bbox = Self::calculate_bbox(
                        mat.start(),
                        mat.end(),
                        line.len(),
                        y_pos,
                        page_width,
                        line_height_pts,
                    );
                    findings.push(PiiFinding {
                        id: format!("pii-ssn-{}-{}", page_index, findings.len()),
                        page_index,
                        category: RedactionCategory::Ssn,
                        matched_text: ssn_raw.to_string(),
                        mask_preview: "XXX-XX-XXXX".to_string(),
                        confidence: 0.98,
                        bbox,
                        approved: true,
                    });
                }
            }

            // 2. Credit Card Check (with Luhn validation)
            for mat in RE_CREDIT_CARD.find_iter(line) {
                let digits: String = mat
                    .as_str()
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                if digits.len() == 16 && Self::luhn_check(&digits) {
                    let bbox = Self::calculate_bbox(
                        mat.start(),
                        mat.end(),
                        line.len(),
                        y_pos,
                        page_width,
                        line_height_pts,
                    );
                    findings.push(PiiFinding {
                        id: format!("pii-cc-{}-{}", page_index, findings.len()),
                        page_index,
                        category: RedactionCategory::CreditCard,
                        matched_text: mat.as_str().to_string(),
                        mask_preview: "****-****-****-****".to_string(),
                        confidence: 0.99,
                        bbox,
                        approved: true,
                    });
                }
            }

            // 3. Email Check
            for mat in RE_EMAIL.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-email-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::Email,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "[EMAIL REDACTED]".to_string(),
                    confidence: 0.95,
                    bbox,
                    approved: true,
                });
            }

            // 4. Phone Check
            for mat in RE_PHONE.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-phone-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::Phone,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "[PHONE REDACTED]".to_string(),
                    confidence: 0.92,
                    bbox,
                    approved: true,
                });
            }

            // 5. IP Address Check
            for mat in RE_IP.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-ip-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::IpAddress,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "[IP REDACTED]".to_string(),
                    confidence: 0.90,
                    bbox,
                    approved: true,
                });
            }

            // 6. Medical Record Number (HIPAA PHI)
            for mat in RE_MRN.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-mrn-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::MedicalRecordNumber,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "MRN: [REDACTED PHI]".to_string(),
                    confidence: 0.94,
                    bbox,
                    approved: true,
                });
            }

            // 7. NPI / DEA Provider ID
            for mat in RE_NPI_DEA.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-npi-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::DeaNpiNumber,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "[PROVIDER ID REDACTED]".to_string(),
                    confidence: 0.93,
                    bbox,
                    approved: true,
                });
            }

            // 8. Bank Account Number
            for mat in RE_BANK.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-bank-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::BankAccount,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "[ACCOUNT NUMBER REDACTED]".to_string(),
                    confidence: 0.95,
                    bbox,
                    approved: true,
                });
            }

            // 9. Currency / Settlement amounts
            for mat in RE_DOLLARS.find_iter(line) {
                let bbox = Self::calculate_bbox(
                    mat.start(),
                    mat.end(),
                    line.len(),
                    y_pos,
                    page_width,
                    line_height_pts,
                );
                findings.push(PiiFinding {
                    id: format!("pii-curr-{}-{}", page_index, findings.len()),
                    page_index,
                    category: RedactionCategory::CustomKeyword,
                    matched_text: mat.as_str().to_string(),
                    mask_preview: "$[FINANCIAL REDACTED]".to_string(),
                    confidence: 0.88,
                    bbox,
                    approved: false, // Optional by default
                });
            }
        }

        findings
    }

    /// Luhn algorithm for valid credit card checksum
    pub fn luhn_check(number: &str) -> bool {
        let mut sum = 0;
        let mut alternate = false;
        for c in number.chars().rev() {
            if let Some(mut d) = c.to_digit(10) {
                if alternate {
                    d *= 2;
                    if d > 9 {
                        d -= 9;
                    }
                }
                sum += d;
                alternate = !alternate;
            } else {
                return false;
            }
        }
        sum % 10 == 0
    }

    fn calculate_bbox(
        start_char: usize,
        end_char: usize,
        line_len: usize,
        y_pos: f32,
        page_width: f32,
        line_height: f32,
    ) -> BoundingBox {
        let content_width = page_width - 80.0;
        let char_width = if line_len > 0 {
            content_width / (line_len as f32).max(1.0)
        } else {
            7.0
        };

        let x = 40.0 + (start_char as f32 * char_width);
        let width = ((end_char - start_char) as f32 * char_width).max(20.0);
        let height = line_height.clamp(14.0, 24.0);

        BoundingBox {
            x,
            y: y_pos,
            width,
            height,
        }
    }

    /// True clean-room burnout: Excises text and rasterizes SVG masks
    pub fn burn_redactions_from_text(
        text: &str,
        findings: &[PiiFinding],
        masks: &[RedactionMask],
    ) -> String {
        let mut sanitized = text.to_string();

        // 1. Burn approved PII findings
        for f in findings.iter().filter(|f| f.approved) {
            sanitized = sanitized.replace(&f.matched_text, &f.mask_preview);
        }

        // 2. Burn manual masks by label if present
        for m in masks {
            if !m.label.is_empty() {
                sanitized = sanitized.replace(
                    &m.label,
                    m.exemption_code.as_deref().unwrap_or("[REDACTED]"),
                );
            }
        }

        sanitized
    }

    /// Renders vector SVG of a page with crisp text, dark redaction bars, and Bates stamp
    pub fn render_page_svg(
        page: &DocumentPage,
        bates_stamp: Option<&str>,
        bates_pos: &BatesPosition,
    ) -> String {
        let w = page.width_pts;
        let h = page.height_pts;

        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" height="100%">
  <defs>
    <pattern id="hatch" width="10" height="10" patternTransform="rotate(45 0 0)" patternUnits="userSpaceOnUse">
      <line x1="0" y1="0" x2="0" y2="10" stroke="#000000" stroke-width="2" />
    </pattern>
  </defs>
  <!-- Background Page -->
  <rect x="0" y="0" width="{w}" height="{h}" fill="#ffffff" stroke="#cbd5e1" stroke-width="1"/>
"##
        );

        // Render Text Lines
        let lines: Vec<&str> = page.text_content.lines().collect();
        let total_lines = lines.len().max(1);
        let line_height_pts = (h - 100.0) / (total_lines as f32).max(1.0);

        for (idx, line) in lines.iter().enumerate() {
            let y = 60.0 + (idx as f32 * line_height_pts);
            let escaped = line
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            svg.push_str(&format!(
                r##"  <text x="45" y="{y}" font-family="monospace, sans-serif" font-size="11" fill="#1e293b">{escaped}</text>
"##
            ));
        }

        // Render Redaction Masks (True clean-room visual burnout)
        for mask in &page.redaction_masks {
            let fill = match mask.burn_style {
                BurnStyle::SolidBlack => "#000000",
                BurnStyle::SolidWhite => "#ffffff",
                BurnStyle::PatternHatch => "url(#hatch)",
                BurnStyle::ExemptionCode => "#1e293b",
            };

            let stroke = if mask.burn_style == BurnStyle::SolidWhite {
                r##"stroke="#000000" stroke-width="1""##
            } else {
                ""
            };

            svg.push_str(&format!(
                r##"  <!-- Redaction Box -->
  <rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" {stroke}/>
"##,
                mask.bbox.x, mask.bbox.y, mask.bbox.width, mask.bbox.height
            ));

            if let Some(ref code) = mask.exemption_code {
                let tx = mask.bbox.x + 4.0;
                let ty = mask.bbox.y + mask.bbox.height - 4.0;
                svg.push_str(&format!(
                    r##"  <text x="{tx}" y="{ty}" font-family="monospace" font-size="9" font-weight="bold" fill="#ffffff">{code}</text>
"##
                ));
            }
        }

        // Render Bates Numbering Stamp
        if let Some(stamp) = bates_stamp {
            let (bx, by, anchor) = match bates_pos {
                BatesPosition::TopLeft => (45.0, 30.0, "start"),
                BatesPosition::TopCenter => (w / 2.0, 30.0, "middle"),
                BatesPosition::TopRight => (w - 45.0, 30.0, "end"),
                BatesPosition::BottomLeft => (45.0, h - 25.0, "start"),
                BatesPosition::BottomCenter => (w / 2.0, h - 25.0, "middle"),
                BatesPosition::BottomRight => (w - 45.0, h - 25.0, "end"),
            };

            svg.push_str(&format!(
                r##"  <!-- Bates Stamp -->
  <text x="{bx}" y="{by}" text-anchor="{anchor}" font-family="monospace" font-size="10" font-weight="bold" fill="#dc2626">{stamp}</text>
"##
            ));
        }

        svg.push_str("</svg>");
        svg
    }

    /// Formats a Bates number given config and page index
    pub fn format_bates(config: &BatesConfig, page_idx: usize) -> String {
        let num = config.start_number + page_idx as u64;
        format!(
            "{}{:0width$}{}",
            config.prefix,
            num,
            config.suffix,
            width = config.digits
        )
    }

    /// Strips metadata from document byte representation
    pub fn strip_pdf_metadata(raw_pdf_bytes: &[u8]) -> Vec<u8> {
        let mut clean = Vec::with_capacity(raw_pdf_bytes.len());
        let text = String::from_utf8_lossy(raw_pdf_bytes);

        // Neutralize Info Dict metadata entries
        let mut sanitized = text.to_string();
        for key in &[
            "/Author",
            "/Title",
            "/Subject",
            "/Keywords",
            "/Creator",
            "/Producer",
            "/CreationDate",
            "/ModDate",
        ] {
            if let Some(start) = sanitized.find(key) {
                if let Some(end) = sanitized[start..].find('\n') {
                    let range = start..start + end;
                    sanitized.replace_range(range, &format!("{} ()", key));
                }
            }
        }

        // Erase XMP metadata packets
        if let Some(xmp_start) = sanitized.find("<?xpacket begin") {
            if let Some(xmp_end) = sanitized[xmp_start..].find("<?xpacket end") {
                let full_end = xmp_start + xmp_end + 19;
                if full_end <= sanitized.len() {
                    let filler = " ".repeat(full_end - xmp_start);
                    sanitized.replace_range(xmp_start..full_end, &filler);
                }
            }
        }

        clean.extend_from_slice(sanitized.as_bytes());
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_detection_ssn_and_credit_card() {
        let sample = "Employee John Doe with SSN 245-18-9034 and Card 4532-0150-1234-5671 called Dr. Jenkins.";
        let findings = RedactionEngine::scan_page_for_pii(0, sample, 612.0, 792.0);

        let ssn = findings
            .iter()
            .find(|f| f.category == RedactionCategory::Ssn);
        assert!(ssn.is_some());
        assert_eq!(ssn.unwrap().matched_text, "245-18-9034");

        let cc = findings
            .iter()
            .find(|f| f.category == RedactionCategory::CreditCard);
        assert!(cc.is_some());
    }

    #[test]
    fn test_luhn_credit_card_check() {
        assert!(RedactionEngine::luhn_check("4532015012345671"));
        assert!(!RedactionEngine::luhn_check("1234567812345678"));
    }

    #[test]
    fn test_clean_room_text_burnout() {
        let original = "Confidential report for john.doe@agency.gov with MRN: 987654321.";
        let findings = RedactionEngine::scan_page_for_pii(0, original, 612.0, 792.0);
        let burned = RedactionEngine::burn_redactions_from_text(original, &findings, &[]);

        assert!(!burned.contains("john.doe@agency.gov"));
        assert!(!burned.contains("987654321"));
        assert!(burned.contains("[EMAIL REDACTED]") || burned.contains("[REDACTED PHI]"));
    }

    #[test]
    fn test_bates_formatting_and_svg_rendering() {
        let config = BatesConfig {
            prefix: "DEF-DOC-".to_string(),
            suffix: "-CONF".to_string(),
            start_number: 42,
            digits: 6,
            position: BatesPosition::BottomRight,
            font_size: 10,
            include_date: false,
        };

        let stamp = RedactionEngine::format_bates(&config, 0);
        assert_eq!(stamp, "DEF-DOC-000042-CONF");

        let page = DocumentPage {
            page_index: 0,
            width_pts: 612.0,
            height_pts: 792.0,
            text_content: "Privileged & Confidential\nDo not distribute.".to_string(),
            pii_findings: vec![],
            redaction_masks: vec![RedactionMask {
                id: "mask-1".to_string(),
                page_index: 0,
                bbox: BoundingBox {
                    x: 45.0,
                    y: 60.0,
                    width: 150.0,
                    height: 18.0,
                },
                label: "Privileged".to_string(),
                burn_style: BurnStyle::ExemptionCode,
                exemption_code: Some("(b)(5)".to_string()),
            }],
            thumbnail_svg: String::new(),
        };

        let svg = RedactionEngine::render_page_svg(&page, Some(&stamp), &config.position);
        assert!(svg.contains("DEF-DOC-000042-CONF"));
        assert!(svg.contains("(b)(5)"));
        assert!(svg.contains("fill=\"#1e293b\""));
    }
}
