#![forbid(unsafe_code)]
//! DevOps Secret Vault - On-Device Security Risk Scoring & Health Assessment
//!
//! Evaluates the 5-dimensional cryptographic and hygiene posture of the vault:
//! 1. Password entropy and character space complexity
//! 2. Zero-knowledge credential reuse detection via ephemeral in-memory keyed hashes
//! 3. Credential rotation age (>90 days, >180 days)
//! 4. Curated offline dictionary of default/compromised developer credentials
//! 5. X.509 certificate hygiene (expired, expiring <=30d, weak keys/signatures)

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cert_monitor::{inspect_x509_pem, CertStatus};
use crate::models::{calculate_entropy, DecryptedSecret, SecretPayload};

/// Recommendation severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Actionable remediation recommendation item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub secret_id: String,
    pub secret_title: String,
}

/// Comprehensive Vault Health & Risk Report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultHealthReport {
    pub overall_score: u32,
    pub grade: String,
    pub total_secrets: usize,
    pub compromised_count: usize,
    pub reused_count: usize,
    pub weak_count: usize,
    pub stale_count: usize,
    pub cert_issues_count: usize,
    pub recommendations: Vec<SecurityRecommendation>,
}

// ---------------------------------------------------------------------------
// Offline Curated Dictionary of Default / Exposed Credentials
// ---------------------------------------------------------------------------

const COMMON_DEFAULT_SECRETS: &[&str] = &[
    "admin",
    "password",
    "123456",
    "12345678",
    "1234",
    "0000",
    "qwerty",
    "root",
    "toor",
    "guest",
    "test",
    "secret",
    "master",
    "database",
    "letmein",
    "welcome",
    "changeme",
    "changeme123",
    "password123",
    "admin123",
    "postgres",
    "default",
    "mysecretpassword",
];

// ---------------------------------------------------------------------------
// Zero-Knowledge Keyed Hasher
// ---------------------------------------------------------------------------

/// Hash a sensitive secret string with an ephemeral in-memory salt.
/// Plaintext secrets are never stored or compared in matrices.
fn hash_secret_zero_knowledge(salt: &[u8; 32], secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(secret.as_bytes());
    let res = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&res);
    out
}

// ---------------------------------------------------------------------------
// Risk Scoring Engine
// ---------------------------------------------------------------------------

/// Evaluate security health and risk report across all unlocked vault secrets
pub fn evaluate_vault_health(secrets: &[DecryptedSecret]) -> VaultHealthReport {
    if secrets.is_empty() {
        return VaultHealthReport {
            overall_score: 100,
            grade: "A+".to_string(),
            total_secrets: 0,
            compromised_count: 0,
            reused_count: 0,
            weak_count: 0,
            stale_count: 0,
            cert_issues_count: 0,
            recommendations: Vec::new(),
        };
    }

    let mut penalty: u32 = 0;
    let mut recommendations = Vec::new();
    let mut compromised_count = 0;
    let mut weak_count = 0;
    let mut stale_count = 0;
    let mut cert_issues_count = 0;

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 1. Ephemeral salt for Zero-Knowledge Credential Reuse detection
    let mut run_salt = [0u8; 32];
    OsRng.fill_bytes(&mut run_salt);
    let mut hash_buckets: HashMap<[u8; 32], Vec<(String, String)>> = HashMap::new();

    for sec in secrets {
        let val = sec.payload.primary_secret_value();
        let val_trimmed = val.trim();

        // Check for empty or trivial secret
        if !val_trimmed.is_empty() {
            let bucket_hash = hash_secret_zero_knowledge(&run_salt, val_trimmed);
            hash_buckets
                .entry(bucket_hash)
                .or_default()
                .push((sec.metadata.id.clone(), sec.metadata.title.clone()));
        }

        // Check Default / Exposed passwords
        let is_default = COMMON_DEFAULT_SECRETS
            .iter()
            .any(|&common| common.eq_ignore_ascii_case(val_trimmed));

        if is_default {
            compromised_count += 1;
            penalty = penalty.saturating_add(25);
            recommendations.push(SecurityRecommendation {
                id: format!("rec-default-{}", sec.metadata.id),
                severity: Severity::Critical,
                title: format!("Default or Common Credential: {}", sec.metadata.title),
                description:
                    "This secret matches a well-known default credential. Change it immediately."
                        .to_string(),
                secret_id: sec.metadata.id.clone(),
                secret_title: sec.metadata.title.clone(),
            });
        }

        // Check Entropy and Length (for passwords / api keys)
        let entropy = calculate_entropy(val_trimmed).entropy_bits;
        if val_trimmed.len() < 12 || entropy < 40.0 {
            weak_count += 1;
            penalty = penalty.saturating_add(8);
            recommendations.push(SecurityRecommendation {
                id: format!("rec-entropy-{}", sec.metadata.id),
                severity: Severity::High,
                title: format!("Weak Secret Entropy: {}", sec.metadata.title),
                description: format!(
                    "Secret length ({}) or entropy ({:.1} bits) is below the recommended 12 chars / 50 bits threshold.",
                    val_trimmed.len(),
                    entropy
                ),
                secret_id: sec.metadata.id.clone(),
                secret_title: sec.metadata.title.clone(),
            });
        }

        // Check Rotation Age
        let age_seconds = now_epoch.saturating_sub(sec.metadata.updated_at);
        let age_days = age_seconds / 86400;

        if age_days > 180 {
            stale_count += 1;
            penalty = penalty.saturating_add(10);
            recommendations.push(SecurityRecommendation {
                id: format!("rec-stale-180-{}", sec.metadata.id),
                severity: Severity::Medium,
                title: format!("Critical Stale Credential: {}", sec.metadata.title),
                description: format!(
                    "Credential has not been rotated in {} days (exceeds 180-day policy).",
                    age_days
                ),
                secret_id: sec.metadata.id.clone(),
                secret_title: sec.metadata.title.clone(),
            });
        } else if age_days > 90 {
            stale_count += 1;
            penalty = penalty.saturating_add(4);
            recommendations.push(SecurityRecommendation {
                id: format!("rec-stale-90-{}", sec.metadata.id),
                severity: Severity::Low,
                title: format!("Stale Credential: {}", sec.metadata.title),
                description: format!(
                    "Credential has not been rotated in {} days (exceeds 90-day recommendation).",
                    age_days
                ),
                secret_id: sec.metadata.id.clone(),
                secret_title: sec.metadata.title.clone(),
            });
        }

        // Check X.509 Certificate Hygiene
        if let SecretPayload::Certificate {
            certificate_pem, ..
        } = &sec.payload
        {
            if let Ok(report) = inspect_x509_pem(certificate_pem) {
                if report.status == CertStatus::Expired {
                    cert_issues_count += 1;
                    penalty = penalty.saturating_add(20);
                    recommendations.push(SecurityRecommendation {
                        id: format!("rec-cert-expired-{}", sec.metadata.id),
                        severity: Severity::Critical,
                        title: format!("Expired Certificate: {}", sec.metadata.title),
                        description: format!(
                            "X.509 certificate for '{}' expired {} days ago.",
                            report.subject_cn,
                            report.days_until_expiration.abs()
                        ),
                        secret_id: sec.metadata.id.clone(),
                        secret_title: sec.metadata.title.clone(),
                    });
                } else if report.status == CertStatus::ExpiringSoon {
                    cert_issues_count += 1;
                    penalty = penalty.saturating_add(10);
                    recommendations.push(SecurityRecommendation {
                        id: format!("rec-cert-soon-{}", sec.metadata.id),
                        severity: Severity::High,
                        title: format!("Certificate Expiring Soon: {}", sec.metadata.title),
                        description: format!(
                            "X.509 certificate for '{}' expires in {} days.",
                            report.subject_cn, report.days_until_expiration
                        ),
                        secret_id: sec.metadata.id.clone(),
                        secret_title: sec.metadata.title.clone(),
                    });
                }

                if report.is_weak_signature || report.is_weak_key {
                    cert_issues_count += 1;
                    penalty = penalty.saturating_add(15);
                    recommendations.push(SecurityRecommendation {
                        id: format!("rec-cert-weak-{}", sec.metadata.id),
                        severity: Severity::High,
                        title: format!("Weak Cryptographic Certificate: {}", sec.metadata.title),
                        description: format!(
                            "Certificate uses weak algorithm ({}) or key size ({:?} bits).",
                            report.signature_algorithm, report.key_size_bits
                        ),
                        secret_id: sec.metadata.id.clone(),
                        secret_title: sec.metadata.title.clone(),
                    });
                }
            }
        }
    }

    // Evaluate Credential Reuse from hash buckets
    let mut reused_count = 0;
    for items in hash_buckets.values() {
        if items.len() > 1 {
            reused_count += items.len();
            penalty = penalty.saturating_add(15 * (items.len() as u32 - 1));
            let names: Vec<String> = items.iter().map(|(_, t)| t.clone()).collect();
            for (id, title) in items {
                recommendations.push(SecurityRecommendation {
                    id: format!("rec-reused-{}", id),
                    severity: Severity::High,
                    title: format!("Shared / Reused Credential: {}", title),
                    description: format!(
                        "Secret payload is identical across {} items: {}.",
                        items.len(),
                        names.join(", ")
                    ),
                    secret_id: id.clone(),
                    secret_title: title.clone(),
                });
            }
        }
    }

    // Sort recommendations by severity descending (Critical first)
    recommendations.sort_by_key(|b| std::cmp::Reverse(b.severity));

    let overall_score = 100u32.saturating_sub(penalty);
    let grade = match overall_score {
        95..=100 => "A+",
        90..=94 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    }
    .to_string();

    VaultHealthReport {
        overall_score,
        grade,
        total_secrets: secrets.len(),
        compromised_count,
        reused_count,
        weak_count,
        stale_count,
        cert_issues_count,
        recommendations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SecretMetadata, SecretType};

    #[test]
    fn test_empty_vault_health() {
        let report = evaluate_vault_health(&[]);
        assert_eq!(report.overall_score, 100);
        assert_eq!(report.grade, "A+");
        assert!(report.recommendations.is_empty());
    }

    #[test]
    fn test_compromised_and_reused_scoring() {
        let sec1 = DecryptedSecret {
            metadata: SecretMetadata {
                id: "s1".to_string(),
                title: "MySQL Prod".to_string(),
                secret_type: SecretType::Password,
                tags: vec![],
                username: Some("root".to_string()),
                url: None,
                notes: None,
                version: 1,
                created_at: 0,
                updated_at: 0,
            },
            payload: SecretPayload::Password {
                password: "admin".to_string(), // Default!
            },
        };

        let sec2 = DecryptedSecret {
            metadata: SecretMetadata {
                id: "s2".to_string(),
                title: "PostgreSQL Staging".to_string(),
                secret_type: SecretType::Password,
                tags: vec![],
                username: Some("postgres".to_string()),
                url: None,
                notes: None,
                version: 1,
                created_at: 0,
                updated_at: 0,
            },
            payload: SecretPayload::Password {
                password: "admin".to_string(), // Reused with s1!
            },
        };

        let report = evaluate_vault_health(&[sec1, sec2]);
        assert!(report.overall_score < 70);
        assert_eq!(report.compromised_count, 2);
        assert_eq!(report.reused_count, 2);
        assert!(!report.recommendations.is_empty());
        assert_eq!(report.recommendations[0].severity, Severity::Critical);
    }
}
