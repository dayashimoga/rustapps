#![forbid(unsafe_code)]
//! Business-Tier Log PII and Secret Redaction Engine (ADR-0031)
//!
//! Scrubs sensitive data (emails, IPv4/IPv6, JWT tokens, AWS credentials, credit cards)
//! for safe forensics export, cryptographically gated by `shared-licensing`.

use crate::parsers::NormalizedLogRecord;
use regex::Regex;
use shared_licensing::{LicenseTier, LicensingError, TierGate};
use std::io::Write;
use thiserror::Error;

/// Redaction error definitions
#[derive(Debug, Error)]
pub enum RedactionError {
    #[error("Licensing error during redaction export: {0}")]
    Licensing(#[from] LicensingError),
    #[error("I/O error writing redacted stream: {0}")]
    Io(#[from] std::io::Error),
}

/// PII and Credential Redaction Engine
pub struct RedactionEngine {
    re_ipv4: Regex,
    re_ipv6: Regex,
    re_email: Regex,
    re_aws_key: Regex,
    re_jwt: Regex,
    re_card: Regex,
}

impl Default for RedactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RedactionEngine {
    pub fn new() -> Self {
        Self {
            re_ipv4: Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").expect("Valid regex"),
            re_ipv6: Regex::new(r"\b[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){7}\b")
                .expect("Valid regex"),
            re_email: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
                .expect("Valid regex"),
            re_aws_key: Regex::new(r"\b(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}\b").expect("Valid regex"),
            re_jwt: Regex::new(r"\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b")
                .expect("Valid regex"),
            re_card: Regex::new(r"\b(?:\d{4}[ -]?){3}\d{4}\b").expect("Valid regex"),
        }
    }

    /// Redact all sensitive patterns in text string
    pub fn redact_text(&self, text: &str) -> String {
        let s1 = self.re_email.replace_all(text, "[REDACTED_EMAIL]");
        let s2 = self.re_aws_key.replace_all(&s1, "[REDACTED_AWS_KEY]");
        let s3 = self.re_jwt.replace_all(&s2, "[REDACTED_JWT]");
        let s4 = self.re_ipv4.replace_all(&s3, "[REDACTED_IP]");
        let s5 = self.re_ipv6.replace_all(&s4, "[REDACTED_IPV6]");

        // Validate card candidates with Luhn algorithm
        let s6 = self.re_card.replace_all(&s5, |caps: &regex::Captures| {
            let matched = &caps[0];
            let digits: String = matched.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 16 && luhn_check(&digits) {
                "[REDACTED_CARD]".to_string()
            } else {
                matched.to_string()
            }
        });

        s6.into_owned()
    }

    /// Redact an entire NormalizedLogRecord, checking Business tier license gate
    pub fn redact_record(
        &self,
        record: &NormalizedLogRecord,
        gate: &TierGate,
    ) -> Result<NormalizedLogRecord, LicensingError> {
        gate.require_tier(LicenseTier::Business)?;

        let mut redacted = record.clone();
        redacted.message = self.redact_text(&record.message);
        redacted.raw = self.redact_text(&record.raw);

        for val in redacted.attributes.values_mut() {
            *val = self.redact_text(val);
        }

        Ok(redacted)
    }

    /// Stream records to a writer with full PII redaction (Business tier)
    pub fn export_redacted_stream<W: Write>(
        &self,
        records: &[NormalizedLogRecord],
        writer: &mut W,
        gate: &TierGate,
    ) -> Result<usize, RedactionError> {
        gate.require_tier(LicenseTier::Business)?;

        let mut count = 0;
        for record in records {
            let redacted = self.redact_record(record, gate)?;
            let line = format!(
                "{} [{:?}] {}\n",
                redacted.timestamp_raw.as_deref().unwrap_or("-"),
                redacted.severity,
                redacted.message
            );
            writer.write_all(line.as_bytes())?;
            count += 1;
        }
        writer.flush()?;
        Ok(count)
    }
}

/// Luhn checksum validation algorithm for 16-digit payment card numbers
fn luhn_check(digits: &str) -> bool {
    let mut sum = 0;
    let mut alternate = false;

    for c in digits.chars().rev() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::LogSeverity;
    use shared_licensing::{AppId, LicenseAuthority, LicenseParameters, LicenseVerifier};

    #[test]
    fn test_pii_redaction_patterns() {
        let engine = RedactionEngine::new();

        let raw = "User alice@company.org from 192.168.1.100 used AKIAIOSFODNN7EXAMPLE to login";
        let redacted = engine.redact_text(raw);

        assert!(!redacted.contains("alice@company.org"));
        assert!(!redacted.contains("192.168.1.100"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("[REDACTED_IP]"));
        assert!(redacted.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn test_business_tier_gating() {
        let engine = RedactionEngine::new();
        let record = NormalizedLogRecord {
            line_number: 1,
            raw: "Secret token test".to_string(),
            timestamp_raw: None,
            timestamp_epoch_ms: None,
            severity: LogSeverity::Info,
            message: "User bob@test.com logged in".to_string(),
            source: None,
            attributes: std::collections::BTreeMap::new(),
        };

        // Free tier gate fails
        let free_gate = TierGate::new(None);
        let err = engine.redact_record(&record, &free_gate).unwrap_err();
        assert_eq!(
            err,
            LicensingError::InsufficientTier {
                required: LicenseTier::Business,
                current: LicenseTier::Free,
            }
        );

        // Business tier license succeeds
        let authority = LicenseAuthority::generate();
        let verifier = LicenseVerifier::new(&authority.public_key_bytes()).unwrap();
        let cert = authority.sign_license(LicenseParameters {
            license_id: "BIZ-001".to_string(),
            licensee_name: "Acme Corp".to_string(),
            licensee_email: "acme@example.com".to_string(),
            tier: LicenseTier::Business,
            app_id: AppId::UniversalLogViewer,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None,
            max_seats: 50,
            features: vec!["redaction".to_string()],
        });

        let verified = verifier
            .verify(&cert, AppId::UniversalLogViewer, 1750000000)
            .unwrap();
        let biz_gate = TierGate::new(Some(verified));

        let redacted = engine.redact_record(&record, &biz_gate).unwrap();
        assert_eq!(redacted.message, "User [REDACTED_EMAIL] logged in");
    }
}
