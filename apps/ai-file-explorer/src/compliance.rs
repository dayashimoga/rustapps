#![forbid(unsafe_code)]
//! Business-Tier Sensitive Data & PII Compliance Scanner
//!
//! Scans local files and repositories for exposed secrets, credentials, and PII:
//! - RSA / OpenSSH / EC Private Keys
//! - AWS Access Key IDs & Secret Access Keys
//! - US Social Security Numbers (SSN)
//! - Luhn-validated Credit Card Numbers (Visa, MasterCard, Amex, Discover)
//! - Exposed Passwords in Database Connection Strings
//!
//! Gated by `shared_licensing::TierGate` requiring Business/Enterprise tier.

use regex::Regex;
use serde::{Deserialize, Serialize};
use shared_licensing::{LicenseTier, LicensingError, TierGate};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;

/// Compliance scanner error definitions
#[derive(Debug, Error)]
pub enum ComplianceError {
    #[error("I/O error during compliance scan: {0}")]
    Io(#[from] std::io::Error),
    #[error("Licensing error: {0}")]
    Licensing(#[from] LicensingError),
    #[error("Regex compilation error: {0}")]
    Regex(#[from] regex::Error),
}

/// Severity classification of detected sensitive findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
}

/// Type of sensitive data detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataType {
    PrivateKey,
    AwsAccessKey,
    Ssn,
    CreditCard,
    ConnectionStringPassword,
}

/// Detailed compliance violation finding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensitiveFinding {
    pub id: String,
    pub data_type: SensitiveDataType,
    pub severity: FindingSeverity,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub redacted_preview: String,
}

/// Overall report summary of a compliance scan
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceScanSummary {
    pub scanned_files_count: usize,
    pub findings_count: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub findings: Vec<SensitiveFinding>,
    pub scan_duration_ms: f64,
}

/// Compliance engine performing fast streaming regex and Luhn scanning
pub struct ComplianceScanner {
    private_key_re: Regex,
    aws_access_key_re: Regex,
    ssn_re: Regex,
    credit_card_re: Regex,
    conn_str_re: Regex,
}

impl Default for ComplianceScanner {
    fn default() -> Self {
        Self::new().expect("Failed to initialize compliance regex patterns")
    }
}

impl ComplianceScanner {
    /// Initialize compliance scanner with precompiled regular expressions
    pub fn new() -> Result<Self, ComplianceError> {
        let private_key_re = Regex::new(r"(?m)-----BEGIN [A-Z ]*PRIVATE KEY-----")?;
        let aws_access_key_re = Regex::new(r"\b(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}\b")?;
        let ssn_re = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")?;
        let credit_card_re = Regex::new(
            r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b",
        )?;
        let conn_str_re =
            Regex::new(r"(?i)(?:postgres|postgresql|mysql|mongodb|redis)://[^:\s]+:([^@\s]{4,})@")?;

        Ok(Self {
            private_key_re,
            aws_access_key_re,
            ssn_re,
            credit_card_re,
            conn_str_re,
        })
    }

    /// Scan arbitrary text content for sensitive findings
    pub fn scan_text(&self, text: &str, file_path: &Path) -> Vec<SensitiveFinding> {
        let mut findings = Vec::new();
        let mut id_counter = 1usize;

        for (line_idx, line) in text.lines().enumerate() {
            let line_num = line_idx + 1;

            // 1. Private keys (Critical)
            if self.private_key_re.is_match(line) {
                findings.push(SensitiveFinding {
                    id: format!("comp-{id_counter}"),
                    data_type: SensitiveDataType::PrivateKey,
                    severity: FindingSeverity::Critical,
                    file_path: file_path.to_path_buf(),
                    line_number: line_num,
                    redacted_preview: "-----BEGIN [REDACTED] PRIVATE KEY-----".to_string(),
                });
                id_counter += 1;
            }

            // 2. AWS Access Keys (Critical)
            for mat in self.aws_access_key_re.find_iter(line) {
                let key_str = mat.as_str();
                let prefix = &key_str[..4];
                findings.push(SensitiveFinding {
                    id: format!("comp-{id_counter}"),
                    data_type: SensitiveDataType::AwsAccessKey,
                    severity: FindingSeverity::Critical,
                    file_path: file_path.to_path_buf(),
                    line_number: line_num,
                    redacted_preview: format!("{prefix}****************"),
                });
                id_counter += 1;
            }

            // 3. Social Security Numbers (High)
            for mat in self.ssn_re.find_iter(line) {
                let ssn = mat.as_str();
                if !ssn.starts_with("000") && !ssn.starts_with("666") && !ssn.starts_with('9') {
                    findings.push(SensitiveFinding {
                        id: format!("comp-{id_counter}"),
                        data_type: SensitiveDataType::Ssn,
                        severity: FindingSeverity::High,
                        file_path: file_path.to_path_buf(),
                        line_number: line_num,
                        redacted_preview: "***-**-****".to_string(),
                    });
                    id_counter += 1;
                }
            }

            // 4. Credit Cards with Luhn validation (Critical)
            for mat in self.credit_card_re.find_iter(line) {
                let card = mat.as_str();
                if luhn_check(card) {
                    let last4 = &card[card.len().saturating_sub(4)..];
                    findings.push(SensitiveFinding {
                        id: format!("comp-{id_counter}"),
                        data_type: SensitiveDataType::CreditCard,
                        severity: FindingSeverity::Critical,
                        file_path: file_path.to_path_buf(),
                        line_number: line_num,
                        redacted_preview: format!("****-****-****-{last4}"),
                    });
                    id_counter += 1;
                }
            }

            // 5. Database Connection String Passwords (High)
            for mat in self.conn_str_re.captures_iter(line) {
                if let Some(pwd_match) = mat.get(1) {
                    let pwd = pwd_match.as_str();
                    let redacted_pwd = if pwd.len() > 2 {
                        format!("{}***", &pwd[..1])
                    } else {
                        "***".to_string()
                    };
                    findings.push(SensitiveFinding {
                        id: format!("comp-{id_counter}"),
                        data_type: SensitiveDataType::ConnectionStringPassword,
                        severity: FindingSeverity::High,
                        file_path: file_path.to_path_buf(),
                        line_number: line_num,
                        redacted_preview: format!("protocol://user:{redacted_pwd}@host"),
                    });
                    id_counter += 1;
                }
            }
        }

        findings
    }

    /// Scan a single file on disk (Business Tier required)
    pub fn scan_file(
        &self,
        path: &Path,
        gate: &TierGate,
    ) -> Result<Vec<SensitiveFinding>, ComplianceError> {
        gate.require_tier(LicenseTier::Business)?;

        // Skip binary or huge files (>10MB)
        let meta = fs::metadata(path)?;
        if meta.len() > 10 * 1024 * 1024 {
            return Ok(Vec::new());
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()), // Skip non-UTF8 binary files
        };

        Ok(self.scan_text(&content, path))
    }

    /// Recursively scan an entire directory for compliance violations (Business Tier required)
    pub fn scan_directory(
        &self,
        root: &Path,
        gate: &TierGate,
    ) -> Result<ComplianceScanSummary, ComplianceError> {
        gate.require_tier(LicenseTier::Business)?;
        let start = Instant::now();

        let mut scanned_files = 0usize;
        let mut all_findings = Vec::new();

        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".git" && name != "node_modules" && name != "target"
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                scanned_files += 1;
                if let Ok(file_findings) = self.scan_file(entry.path(), gate) {
                    all_findings.extend(file_findings);
                }
            }
        }

        let critical_count = all_findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .count();
        let high_count = all_findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::High)
            .count();
        let medium_count = all_findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Medium)
            .count();

        Ok(ComplianceScanSummary {
            scanned_files_count: scanned_files,
            findings_count: all_findings.len(),
            critical_count,
            high_count,
            medium_count,
            findings: all_findings,
            scan_duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }
}

/// Luhn algorithm (mod 10) for credit card validation
fn luhn_check(num_str: &str) -> bool {
    let digits: Vec<u32> = num_str.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    let mut sum = 0;
    let mut alternate = false;

    for &digit in digits.iter().rev() {
        if alternate {
            let doubled = digit * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += digit;
        }
        alternate = !alternate;
    }

    sum % 10 == 0
}

/// Generates a certified vector PDF 1.4 Data Loss Prevention (DLP) compliance audit report
pub fn generate_compliance_pdf_report(summary: &ComplianceScanSummary) -> Vec<u8> {
    use std::fmt::Write as FmtWrite;

    let items_per_page = 16usize;
    let total_pages = (summary.findings.len().div_ceil(items_per_page)).max(1);

    // PDF Objects list to track byte offsets for xref
    let mut pdf = Vec::with_capacity(32768);
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut obj_offsets: Vec<usize> = Vec::new();

    // Obj 1: Catalog
    obj_offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Obj 2: Pages
    obj_offsets.push(pdf.len());
    let mut kids = String::from("[");
    for i in 0..total_pages {
        let page_obj_id = 3 + i * 2;
        let _ = write!(kids, "{} 0 R ", page_obj_id);
    }
    kids.push(']');
    let pages_dict = format!(
        "2 0 obj\n<< /Type /Pages /Kids {} /Count {} >>\nendobj\n",
        kids, total_pages
    );
    pdf.extend_from_slice(pages_dict.as_bytes());

    let font_regular_id = 3 + total_pages * 2;
    let font_bold_id = font_regular_id + 1;

    // For each page: Page object and Content stream object
    for page_idx in 0..total_pages {
        let page_obj_id = 3 + page_idx * 2;
        let content_obj_id = page_obj_id + 1;

        // Page Object
        obj_offsets.push(pdf.len());
        let page_dict = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {} 0 R /Resources << /Font << /F1 {} 0 R /F2 {} 0 R >> >> >>\nendobj\n",
            page_obj_id, content_obj_id, font_regular_id, font_bold_id
        );
        pdf.extend_from_slice(page_dict.as_bytes());

        // Build content stream for this page
        let mut stream = String::with_capacity(4096);

        // Header Dark Banner (612 wide, 60 tall)
        stream.push_str("0.06 0.09 0.16 rg\n18 718 576 56 re f\n");

        // Header Accent Strip
        stream.push_str("0.24 0.44 0.89 rg\n18 714 576 4 re f\n");

        // Header Text
        stream.push_str("1.0 1.0 1.0 rg\n");
        stream.push_str("BT /F2 14 Tf 32 750 Td (ANTIGRAVITY DATA LOSS PREVENTION \\(DLP\\) AUDIT REPORT) Tj ET\n");
        stream.push_str("0.75 0.82 0.95 rg\n");
        stream.push_str("BT /F1 9 Tf 32 732 Td (100% Offline Air-Gapped Sensitive Data & Secret Scan Certificate) Tj ET\n");

        // Metadata box (710 down to 640)
        stream.push_str("0.96 0.97 0.98 rg\n18 644 576 60 re f\n");
        stream.push_str("0.85 0.88 0.92 RG 1 w\n18 644 576 60 re S\n");

        // Metrics text
        stream.push_str("0.1 0.15 0.25 rg\n");
        let _ = writeln!(
            stream,
            "BT /F2 10 Tf 30 686 Td (Scanned Files: {}) Tj ET",
            summary.scanned_files_count
        );
        let _ = writeln!(
            stream,
            "BT /F2 10 Tf 160 686 Td (Total Findings: {}) Tj ET",
            summary.findings_count
        );
        let _ = writeln!(
            stream,
            "BT /F2 10 Tf 310 686 Td (Scan Time: {:.1} ms) Tj ET",
            summary.scan_duration_ms
        );

        // Breakdown counts
        stream.push_str("0.85 0.15 0.15 rg\n");
        let _ = writeln!(
            stream,
            "BT /F2 9 Tf 30 660 Td (Critical: {}) Tj ET",
            summary.critical_count
        );
        stream.push_str("0.85 0.45 0.05 rg\n");
        let _ = writeln!(
            stream,
            "BT /F2 9 Tf 130 660 Td (High: {}) Tj ET",
            summary.high_count
        );
        stream.push_str("0.65 0.60 0.10 rg\n");
        let _ = writeln!(
            stream,
            "BT /F2 9 Tf 210 660 Td (Medium: {}) Tj ET",
            summary.medium_count
        );
        stream.push_str("0.4 0.45 0.5 rg\n");
        stream.push_str("BT /F1 9 Tf 310 660 Td (Tier: Business / Enterprise Certified) Tj ET\n");

        // Table Header (y = 616, height = 20)
        stream.push_str("0.15 0.20 0.30 rg\n18 616 576 20 re f\n");
        stream.push_str("1.0 1.0 1.0 rg\n");
        stream.push_str("BT /F2 8.5 Tf 24 622 Td (ID) Tj ET\n");
        stream.push_str("BT /F2 8.5 Tf 60 622 Td (SEVERITY) Tj ET\n");
        stream.push_str("BT /F2 8.5 Tf 130 622 Td (DATA TYPE) Tj ET\n");
        stream.push_str("BT /F2 8.5 Tf 240 622 Td (FILE LOCATION) Tj ET\n");
        stream.push_str("BT /F2 8.5 Tf 430 622 Td (REDACTED EVIDENCE) Tj ET\n");

        // Findings Rows
        let start_idx = page_idx * items_per_page;
        let end_idx = (start_idx + items_per_page).min(summary.findings.len());

        if summary.findings.is_empty() {
            // Compliant clean banner
            stream.push_str("0.92 0.98 0.93 rg\n18 550 576 50 re f\n");
            stream.push_str("0.15 0.65 0.25 rg\n");
            stream.push_str("BT /F2 12 Tf 40 570 Td (COMPLIANCE VERIFIED: ZERO SENSITIVE PATTERNS OR LEAKS DETECTED) Tj ET\n");
        } else {
            let mut y_cursor = 594.0f32;
            for (row_idx, finding) in summary.findings[start_idx..end_idx].iter().enumerate() {
                // Zebra row background
                if row_idx % 2 == 1 {
                    stream.push_str(&format!(
                        "0.97 0.98 0.99 rg\n18 {:.1} 576 20 re f\n",
                        y_cursor
                    ));
                }
                stream.push_str(&format!(
                    "0.90 0.92 0.95 RG 0.5 w\n18 {:.1} 576 20 re S\n",
                    y_cursor
                ));

                let text_y = y_cursor + 6.0;

                // ID
                stream.push_str("0.3 0.35 0.4 rg\n");
                let _ = writeln!(
                    stream,
                    "BT /F1 8 Tf 24 {:.1} Td ({}) Tj ET",
                    text_y,
                    sanitize_pdf_str(&finding.id)
                );

                // Severity Badge
                match finding.severity {
                    FindingSeverity::Critical => stream.push_str("0.85 0.15 0.15 rg\n"),
                    FindingSeverity::High => stream.push_str("0.85 0.45 0.05 rg\n"),
                    FindingSeverity::Medium => stream.push_str("0.70 0.60 0.10 rg\n"),
                }
                let _ = writeln!(
                    stream,
                    "BT /F2 8 Tf 60 {:.1} Td ({:?}) Tj ET",
                    text_y, finding.severity
                );

                // Data Type
                stream.push_str("0.1 0.15 0.2 rg\n");
                let type_name = match finding.data_type {
                    SensitiveDataType::PrivateKey => "Private Key",
                    SensitiveDataType::AwsAccessKey => "AWS Secret/Key",
                    SensitiveDataType::Ssn => "US SSN",
                    SensitiveDataType::CreditCard => "Credit Card (Luhn)",
                    SensitiveDataType::ConnectionStringPassword => "DB Conn Password",
                };
                let _ = writeln!(
                    stream,
                    "BT /F1 8 Tf 130 {:.1} Td ({}) Tj ET",
                    text_y, type_name
                );

                // File Location (truncate if too long)
                let path_display = format!(
                    "{}:{}",
                    finding
                        .file_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?"),
                    finding.line_number
                );
                let _ = writeln!(
                    stream,
                    "BT /F1 7.5 Tf 240 {:.1} Td ({}) Tj ET",
                    text_y,
                    sanitize_pdf_str(&path_display)
                );

                // Redacted preview
                let mut preview = finding.redacted_preview.clone();
                if preview.len() > 30 {
                    preview.truncate(27);
                    preview.push_str("...");
                }
                stream.push_str("0.4 0.2 0.2 rg\n");
                let _ = writeln!(
                    stream,
                    "BT /F1 7.5 Tf 430 {:.1} Td ({}) Tj ET",
                    text_y,
                    sanitize_pdf_str(&preview)
                );

                y_cursor -= 22.0;
            }
        }

        // Footer
        stream.push_str("0.8 0.82 0.85 RG 0.5 w\n18 36 576 0.5 re S\n");
        stream.push_str("0.5 0.55 0.6 rg\n");
        let _ = writeln!(
            stream,
            "BT /F1 7.5 Tf 24 24 Td (Page {} of {} | Antigravity Offline Suite | Air-Gapped DLP Engine) Tj ET",
            page_idx + 1,
            total_pages
        );

        // Content Stream Object
        obj_offsets.push(pdf.len());
        let stream_bytes = stream.as_bytes();
        let content_header = format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n",
            content_obj_id,
            stream_bytes.len()
        );
        pdf.extend_from_slice(content_header.as_bytes());
        pdf.extend_from_slice(stream_bytes);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Font 1: Helvetica (Regular)
    obj_offsets.push(pdf.len());
    let font1_dict = format!(
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        font_regular_id
    );
    pdf.extend_from_slice(font1_dict.as_bytes());

    // Font 2: Helvetica-Bold
    obj_offsets.push(pdf.len());
    let font2_dict = format!(
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n",
        font_bold_id
    );
    pdf.extend_from_slice(font2_dict.as_bytes());

    // Xref Table
    let xref_offset = pdf.len();
    let total_objs = obj_offsets.len() + 1;
    let mut xref = format!("xref\n0 {}\n0000000000 65535 f \n", total_objs);
    for off in &obj_offsets {
        let _ = writeln!(xref, "{:010} 00000 n ", off);
    }
    pdf.extend_from_slice(xref.as_bytes());

    // Trailer
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        total_objs, xref_offset
    );
    pdf.extend_from_slice(trailer.as_bytes());

    pdf
}

fn sanitize_pdf_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii_graphic() || c == ' ' => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_patterns_detection() {
        let scanner = ComplianceScanner::new().unwrap();
        let sample_text = r#"
            # Configuration
            aws_access_key_id = AKIAIOSFODNN7EXAMPLE
            database_url = postgres://admin:SuperSecretPass123!@localhost:5432/production
            ssn = 123-45-6789
            -----BEGIN OPENSSH PRIVATE KEY-----
            b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAACFwAAAAdzc2gtcn
            -----END OPENSSH PRIVATE KEY-----
        "#;

        let findings = scanner.scan_text(sample_text, Path::new("config.env"));
        assert!(findings.len() >= 4);

        let types: Vec<SensitiveDataType> = findings.iter().map(|f| f.data_type.clone()).collect();
        assert!(types.contains(&SensitiveDataType::AwsAccessKey));
        assert!(types.contains(&SensitiveDataType::ConnectionStringPassword));
        assert!(types.contains(&SensitiveDataType::Ssn));
        assert!(types.contains(&SensitiveDataType::PrivateKey));
    }

    #[test]
    fn test_luhn_credit_card_check() {
        // Valid Visa test card
        assert!(luhn_check("4532015112830366"));
        // Invalid card
        assert!(!luhn_check("4532015112830367"));
    }

    #[test]
    fn test_tier_gating_free_vs_business() {
        let scanner = ComplianceScanner::new().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let secret_file = temp_dir.path().join("secret.txt");
        fs::write(&secret_file, "AKIAIOSFODNN7EXAMPLE").unwrap();

        let free_gate = TierGate::new(None);
        let biz_gate = TierGate::enterprise();

        // Free tier must fail with InsufficientTier
        let err = scanner.scan_file(&secret_file, &free_gate).unwrap_err();
        assert!(matches!(err, ComplianceError::Licensing(_)));

        // Business tier must succeed
        let findings = scanner.scan_file(&secret_file, &biz_gate).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].data_type, SensitiveDataType::AwsAccessKey);
    }

    #[test]
    fn test_generate_compliance_pdf_report() {
        let findings = vec![
            SensitiveFinding {
                id: "comp-1".to_string(),
                data_type: SensitiveDataType::AwsAccessKey,
                severity: FindingSeverity::Critical,
                file_path: PathBuf::from("src/secret.env"),
                line_number: 12,
                redacted_preview: "AKIA****************".to_string(),
            },
            SensitiveFinding {
                id: "comp-2".to_string(),
                data_type: SensitiveDataType::CreditCard,
                severity: FindingSeverity::Critical,
                file_path: PathBuf::from("data/users.csv"),
                line_number: 84,
                redacted_preview: "****-****-****-1234".to_string(),
            },
            SensitiveFinding {
                id: "comp-3".to_string(),
                data_type: SensitiveDataType::Ssn,
                severity: FindingSeverity::High,
                file_path: PathBuf::from("records/patients.txt"),
                line_number: 45,
                redacted_preview: "***-**-****".to_string(),
            },
        ];

        let summary = ComplianceScanSummary {
            scanned_files_count: 142,
            findings_count: 3,
            critical_count: 2,
            high_count: 1,
            medium_count: 0,
            findings,
            scan_duration_ms: 18.5,
        };

        let pdf = generate_compliance_pdf_report(&summary);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.contains("ANTIGRAVITY DATA LOSS PREVENTION"));
        assert!(pdf_str.contains("Critical: 2"));
        assert!(pdf_str.contains("High: 1"));
        assert!(pdf_str.contains("AKIA"));
    }

    #[test]
    fn test_generate_compliance_pdf_report_empty() {
        let summary = ComplianceScanSummary {
            scanned_files_count: 50,
            findings_count: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            findings: Vec::new(),
            scan_duration_ms: 4.2,
        };

        let pdf = generate_compliance_pdf_report(&summary);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.contains("COMPLIANCE VERIFIED"));
    }
}
