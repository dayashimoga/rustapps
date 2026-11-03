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
}
