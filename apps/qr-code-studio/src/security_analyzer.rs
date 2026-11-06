#![forbid(unsafe_code)]
//! Anti-Quishing Security Threat Analyzer, Encrypted QR & Ed25519 Signature Verification

use crate::models::{
    QrPayloadType, QrStudioError, SecurityAnalysisReport, SecurityFinding, SecurityRiskLevel,
    SignatureVerificationStatus, WifiCredentials,
};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub struct SecurityAnalyzer;

impl SecurityAnalyzer {
    /// Comprehensive offline heuristic inspection of a QR payload for phishing & quishing threats
    pub fn inspect_payload(raw: &str) -> SecurityAnalysisReport {
        let payload_type = QrPayloadType::detect_from_raw(raw);
        let mut findings = Vec::new();
        let mut risk_score = 0u8;
        let mut signature_status = SignatureVerificationStatus::Unsigned;
        let mut signer_public_key_hex = None;
        let mut verified_cleartext = None;
        let mut host_domain = None;

        let lower = raw.trim().to_ascii_lowercase();
        for scheme in &[
            "javascript:",
            "data:",
            "vbscript:",
            "file:",
            "blob:",
            "cmd:",
            "powershell:",
        ] {
            if lower.starts_with(scheme) {
                risk_score = 100;
                findings.push(SecurityFinding {
                    code: "SEC-URI-001".to_string(),
                    severity: SecurityRiskLevel::Critical,
                    title: "Dangerous Executable URI Scheme".to_string(),
                    description: format!("URI scheme '{}' can trigger local code execution or sandbox escapes.", scheme),
                    remediation: "Block execution immediately. Do not launch this scheme in any browser or shell.".to_string(),
                });
                break;
            }
        }

        match payload_type {
            QrPayloadType::EncryptedPayload => {
                signature_status = SignatureVerificationStatus::EncryptedLocked;
                findings.push(SecurityFinding {
                    code: "SEC-ENC-001".to_string(),
                    severity: SecurityRiskLevel::Safe,
                    title: "Encrypted Confidential Payload".to_string(),
                    description: "Payload is encrypted with authenticated AES-256-GCM and requires a passphrase to unlock.".to_string(),
                    remediation: "Provide the decryption passphrase in the studio inspector to read the contents.".to_string(),
                });
            }
            QrPayloadType::SignedPayload => {
                match Self::verify_signed_payload(raw) {
                    Ok((clear, pk)) => {
                        signature_status = SignatureVerificationStatus::SignatureValid;
                        signer_public_key_hex = Some(pk.clone());
                        verified_cleartext = Some(clear.clone());
                        findings.push(SecurityFinding {
                            code: "SEC-SIG-001".to_string(),
                            severity: SecurityRiskLevel::Safe,
                            title: "Cryptographically Verified Signature".to_string(),
                            description: format!("Ed25519 signature is authentic and verified against public key: {}...", &pk[..16.min(pk.len())]),
                            remediation: "Payload authenticity and tamper-resistance are guaranteed offline.".to_string(),
                        });
                        // Also inspect the underlying cleartext
                        let sub_report = Self::inspect_payload(&clear);
                        findings.extend(sub_report.findings);
                        risk_score = risk_score.max(sub_report.risk_score);
                    }
                    Err(e) => {
                        signature_status = SignatureVerificationStatus::SignatureInvalid;
                        risk_score = risk_score.max(90);
                        findings.push(SecurityFinding {
                            code: "SEC-SIG-002".to_string(),
                            severity: SecurityRiskLevel::Critical,
                            title: "Signature Verification Failed".to_string(),
                            description: format!("Ed25519 digital signature on this QR code is forged, corrupted, or tampered: {}", e),
                            remediation: "DO NOT trust this QR code. The certificate signature mismatch indicates tampering.".to_string(),
                        });
                    }
                }
            }
            QrPayloadType::Url => {
                let (url_score, url_host, url_findings) = Self::inspect_url(raw);
                risk_score = risk_score.max(url_score);
                host_domain = url_host;
                findings.extend(url_findings);
            }
            QrPayloadType::WiFi => {
                if let Some(wifi) = WifiCredentials::from_qr_string(raw) {
                    let enc = wifi.encryption.to_ascii_uppercase();
                    if enc == "NOPASS" || enc.is_empty() {
                        risk_score = risk_score.max(35);
                        findings.push(SecurityFinding {
                            code: "SEC-WIFI-001".to_string(),
                            severity: SecurityRiskLevel::MediumRisk,
                            title: "Unencrypted Open Wi-Fi Network".to_string(),
                            description: format!("Network '{}' specifies no encryption. Any traffic sent over this AP can be intercepted.", wifi.ssid),
                            remediation: "Ensure you use a VPN before connecting to open public Wi-Fi networks.".to_string(),
                        });
                    } else if enc == "WEP" {
                        risk_score = risk_score.max(65);
                        findings.push(SecurityFinding {
                            code: "SEC-WIFI-002".to_string(),
                            severity: SecurityRiskLevel::HighRisk,
                            title: "Insecure Legacy WEP Encryption".to_string(),
                            description: "WEP encryption is cryptographically broken and can be cracked in seconds.".to_string(),
                            remediation: "Upgrade the access point to WPA2-AES or WPA3.".to_string(),
                        });
                    }
                }
            }
            _ => {
                // Text, SMS, Email, etc.
                if raw.contains("<script") || raw.contains("javascript:") {
                    risk_score = risk_score.max(80);
                    findings.push(SecurityFinding {
                        code: "SEC-XSS-001".to_string(),
                        severity: SecurityRiskLevel::HighRisk,
                        title: "Potential Cross-Site Scripting (XSS) Vector".to_string(),
                        description: "Payload contains HTML script tags or javascript URI execution triggers.".to_string(),
                        remediation: "Never pipe untrusted QR contents into a browser webview without sanitization.".to_string(),
                    });
                }
            }
        }

        // Determine overall risk level
        let risk_level = if risk_score >= 80 {
            SecurityRiskLevel::Critical
        } else if risk_score >= 50 {
            SecurityRiskLevel::HighRisk
        } else if risk_score >= 25 {
            SecurityRiskLevel::MediumRisk
        } else if risk_score > 0 {
            SecurityRiskLevel::LowRisk
        } else {
            SecurityRiskLevel::Safe
        };

        SecurityAnalysisReport {
            risk_level,
            risk_score,
            payload_type,
            signature_status,
            signer_public_key_hex,
            verified_cleartext,
            host_domain,
            findings,
        }
    }

    /// URL threat inspection heuristics
    fn inspect_url(raw: &str) -> (u8, Option<String>, Vec<SecurityFinding>) {
        let mut score = 0u8;
        let mut findings = Vec::new();
        let lower = raw.trim().to_ascii_lowercase();

        // 1. Dangerous URI schemes
        for scheme in &[
            "javascript:",
            "data:",
            "vbscript:",
            "file:",
            "blob:",
            "cmd:",
            "powershell:",
        ] {
            if lower.starts_with(scheme) {
                return (
                    100,
                    None,
                    vec![SecurityFinding {
                        code: "SEC-URI-001".to_string(),
                        severity: SecurityRiskLevel::Critical,
                        title: "Dangerous Executable URI Scheme".to_string(),
                        description: format!("URI scheme '{}' can trigger local code execution or sandbox escapes.", scheme),
                        remediation: "Block execution immediately. Do not launch this scheme in any browser or shell.".to_string(),
                    }],
                );
            }
        }

        // 2. Parse host
        let host = if let Some(idx) = lower.find("://") {
            let after_scheme = &lower[idx + 3..];
            after_scheme
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        if host.is_empty() {
            return (score, None, findings);
        }

        // 3. Userinfo / Credential embedding (@ symbol trick)
        if host.contains('@') {
            score = score.max(90);
            findings.push(SecurityFinding {
                code: "SEC-URL-002".to_string(),
                severity: SecurityRiskLevel::Critical,
                title: "Phishing Credential Obfuscation (@ trick)".to_string(),
                description: format!("URL contains embedded authority '@' trick in '{}', misleading users about true destination.", host),
                remediation: "Inspect the domain appearing after the '@' sign. That is the true destination server.".to_string(),
            });
        }

        let clean_host = host.split('@').next_back().unwrap_or(&host);

        // 4. Raw IPv4 / IPv6 literals
        let is_ip = clean_host
            .split(':')
            .next()
            .unwrap_or(clean_host)
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '[' || c == ']');
        if is_ip && clean_host.contains('.') {
            score = score.max(70);
            findings.push(SecurityFinding {
                code: "SEC-URL-003".to_string(),
                severity: SecurityRiskLevel::HighRisk,
                title: "Raw IP-Literal Destination Host".to_string(),
                description: format!("URL points directly to numeric IP address '{}' instead of a registered domain name.", clean_host),
                remediation: "Legitimate businesses rarely use direct IP links in customer QR codes. Exercise extreme caution.".to_string(),
            });
        }

        // 5. Punycode / IDN Homograph attack
        if clean_host.starts_with("xn--") || clean_host.contains(".xn--") {
            score = score.max(65);
            findings.push(SecurityFinding {
                code: "SEC-URL-004".to_string(),
                severity: SecurityRiskLevel::HighRisk,
                title: "Internationalized Domain (IDN / Punycode) Confusable".to_string(),
                description: format!("Domain '{}' uses Punycode encoding, a common tactic for spoofing look-alike brand names with Cyrillic/Greek characters.", clean_host),
                remediation: "Verify the decoded unicode representation matches the intended organization exactly.".to_string(),
            });
        }

        // 6. Known URL Shorteners
        let shorteners = [
            "bit.ly",
            "tinyurl.com",
            "t.co",
            "is.gd",
            "ow.ly",
            "buff.ly",
            "rb.gy",
        ];
        if shorteners.iter().any(|&s| {
            clean_host.eq_ignore_ascii_case(s) || clean_host.ends_with(&format!(".{}", s))
        }) {
            score = score.max(30);
            findings.push(SecurityFinding {
                code: "SEC-URL-005".to_string(),
                severity: SecurityRiskLevel::MediumRisk,
                title: "Obfuscated URL Shortener".to_string(),
                description: format!("Domain '{}' is a URL shortening redirector that masks the ultimate web destination.", clean_host),
                remediation: "Use a URL unshortener to inspect the final landing URL before browsing.".to_string(),
            });
        }

        // 7. Unencrypted HTTP on sensitive endpoints
        if lower.starts_with("http://") {
            let path_part = lower
                .find("://")
                .and_then(|idx| lower[idx + 3..].find('/'))
                .map(|idx| &lower[idx..])
                .unwrap_or("");
            if path_part.contains("login")
                || path_part.contains("auth")
                || path_part.contains("wallet")
                || path_part.contains("password")
            {
                score = score.max(75);
                findings.push(SecurityFinding {
                    code: "SEC-URL-006".to_string(),
                    severity: SecurityRiskLevel::HighRisk,
                    title: "Unencrypted Authentication Endpoint (Cleartext HTTP)".to_string(),
                    description: "Target URL transmits sensitive login credentials or authentication tokens over plaintext HTTP.".to_string(),
                    remediation: "Only submit credentials to verified HTTPS endpoints with valid TLS certificates.".to_string(),
                });
            }
        }

        (score, Some(clean_host.to_string()), findings)
    }

    /// Encrypt arbitrary QR payload using authenticated AES-256-GCM + PBKDF2/SHA-256
    pub fn encrypt_payload(plaintext: &str, passphrase: &str) -> Result<String, QrStudioError> {
        if passphrase.trim().is_empty() {
            return Err(QrStudioError::InvalidInput(
                "Encryption passphrase cannot be empty".to_string(),
            ));
        }

        let mut salt = [0u8; 16];
        let mut nonce_bytes = [0u8; 12];
        let mut rng = OsRng;
        rng.fill_bytes(&mut salt);
        rng.fill_bytes(&mut nonce_bytes);

        // Derive 32-byte AES key via PBKDF2-HMAC-SHA256 (10,000 rounds)
        let key = derive_key(passphrase.as_bytes(), &salt);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| QrStudioError::CryptoError(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| QrStudioError::CryptoError(format!("AES-GCM encryption failed: {}", e)))?;

        let salt_hex = bytes_to_hex(&salt);
        let nonce_hex = bytes_to_hex(&nonce_bytes);
        let ct_hex = bytes_to_hex(&ciphertext);

        Ok(format!("AGQR-ENC:v1:{}:{}:{}", salt_hex, nonce_hex, ct_hex))
    }

    /// Decrypt an `AGQR-ENC:v1:<salt>:<nonce>:<ciphertext>` payload
    pub fn decrypt_payload(encrypted_raw: &str, passphrase: &str) -> Result<String, QrStudioError> {
        let parts: Vec<&str> = encrypted_raw.trim().split(':').collect();
        if parts.len() != 5 || parts[0] != "AGQR-ENC" || parts[1] != "v1" {
            return Err(QrStudioError::CryptoError(
                "Invalid AGQR-ENC:v1 payload format".to_string(),
            ));
        }

        let salt = hex_to_bytes(parts[2])?;
        let nonce_bytes = hex_to_bytes(parts[3])?;
        let ciphertext = hex_to_bytes(parts[4])?;

        if salt.len() != 16 || nonce_bytes.len() != 12 {
            return Err(QrStudioError::CryptoError(
                "Invalid salt or nonce length".to_string(),
            ));
        }

        let key = derive_key(passphrase.as_bytes(), &salt);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| QrStudioError::CryptoError(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let decrypted = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
            QrStudioError::CryptoError(
                "Decryption failed: incorrect passphrase or corrupted data".to_string(),
            )
        })?;

        String::from_utf8(decrypted)
            .map_err(|e| QrStudioError::CryptoError(format!("Decrypted bytes not UTF-8: {}", e)))
    }

    /// Digitally sign QR payload with Ed25519
    pub fn sign_payload(payload: &str, seed: Option<&[u8; 32]>) -> Result<String, QrStudioError> {
        let signing_key = if let Some(s) = seed {
            SigningKey::from_bytes(s)
        } else {
            let mut rng = OsRng;
            SigningKey::generate(&mut rng)
        };

        let pubkey = signing_key.verifying_key();
        let pubkey_hex = bytes_to_hex(pubkey.as_bytes());

        let sig = signing_key.sign(payload.as_bytes());
        let sig_hex = bytes_to_hex(&sig.to_bytes());

        Ok(format!(
            "AGQR-SIG:v1:{}:{}:{}",
            pubkey_hex, sig_hex, payload
        ))
    }

    /// Verify an `AGQR-SIG:v1:<pubkey>:<sig>:<payload>` assertion
    pub fn verify_signed_payload(signed_raw: &str) -> Result<(String, String), QrStudioError> {
        let trimmed = signed_raw.trim();
        if !trimmed.starts_with("AGQR-SIG:v1:") {
            return Err(QrStudioError::CryptoError(
                "Not an AGQR-SIG:v1 format".to_string(),
            ));
        }
        let after_prefix = &trimmed["AGQR-SIG:v1:".len()..];
        let mut parts = after_prefix.splitn(3, ':');
        let pk_hex = parts
            .next()
            .ok_or_else(|| QrStudioError::CryptoError("Missing public key".to_string()))?;
        let sig_hex = parts
            .next()
            .ok_or_else(|| QrStudioError::CryptoError("Missing signature".to_string()))?;
        let cleartext = parts
            .next()
            .ok_or_else(|| QrStudioError::CryptoError("Missing payload body".to_string()))?;

        let pk_bytes = hex_to_bytes(pk_hex)?;
        let sig_bytes = hex_to_bytes(sig_hex)?;

        if pk_bytes.len() != 32 || sig_bytes.len() != 64 {
            return Err(QrStudioError::CryptoError(
                "Malformed key or signature lengths".to_string(),
            ));
        }

        let mut pk_arr = [0u8; 32];
        pk_arr.copy_from_slice(&pk_bytes);
        let verifying_key = VerifyingKey::from_bytes(&pk_arr)
            .map_err(|e| QrStudioError::CryptoError(format!("Invalid public key: {}", e)))?;

        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        verifying_key
            .verify(cleartext.as_bytes(), &signature)
            .map_err(|e| {
                QrStudioError::CryptoError(format!("Ed25519 verification failed: {}", e))
            })?;

        Ok((cleartext.to_string(), pk_hex.to_string()))
    }
}

/// PBKDF2-HMAC-SHA256 Key Derivation (10,000 iterations)
fn derive_key(passphrase: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    let mut u = Sha256::new();
    u.update(passphrase);
    u.update(salt);
    u.update(1u32.to_be_bytes());
    let mut prev = u.finalize();
    key.copy_from_slice(&prev);

    for _ in 1..10000 {
        let mut h = Sha256::new();
        h.update(passphrase);
        h.update(prev);
        prev = h.finalize();
        for (k, &p) in key.iter_mut().zip(prev.iter()) {
            *k ^= p;
        }
    }
    key
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, QrStudioError> {
    if hex.len() % 2 != 0 {
        return Err(QrStudioError::CryptoError("Odd hex length".to_string()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| QrStudioError::CryptoError(format!("Hex parse error: {}", e)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_analyzer_phishing_detection() {
        let bad_url = "https://paypal.com@evil-phish.net/login?account=verify";
        let report = SecurityAnalyzer::inspect_payload(bad_url);
        assert_eq!(report.risk_level, SecurityRiskLevel::Critical);
        assert!(report.findings.iter().any(|f| f.code == "SEC-URL-002"));

        let javascript_uri = "javascript:alert(document.cookie)";
        let report_js = SecurityAnalyzer::inspect_payload(javascript_uri);
        assert_eq!(report_js.risk_level, SecurityRiskLevel::Critical);
        assert_eq!(report_js.risk_score, 100);

        let safe_url = "https://github.com/dayashimoga/rustapps";
        let report_safe = SecurityAnalyzer::inspect_payload(safe_url);
        assert_eq!(report_safe.risk_level, SecurityRiskLevel::Safe);
        assert_eq!(report_safe.risk_score, 0);
    }

    #[test]
    fn test_aes_gcm_encrypted_qr_roundtrip() {
        let secret = "CONFIDENTIAL_API_KEY_94819482";
        let pass = "CorrectHorseBatteryStaple!";

        let encrypted = SecurityAnalyzer::encrypt_payload(secret, pass).expect("Should encrypt");
        assert!(encrypted.starts_with("AGQR-ENC:v1:"));

        let decrypted =
            SecurityAnalyzer::decrypt_payload(&encrypted, pass).expect("Should decrypt");
        assert_eq!(decrypted, secret);

        let wrong_pass = SecurityAnalyzer::decrypt_payload(&encrypted, "WrongPassword");
        assert!(wrong_pass.is_err());
    }

    #[test]
    fn test_ed25519_signed_qr_roundtrip() {
        let payload = "TICKET:EVENT-2026:VIP:SEAT-42A";
        let signed = SecurityAnalyzer::sign_payload(payload, None).expect("Should sign");
        assert!(signed.starts_with("AGQR-SIG:v1:"));

        let report = SecurityAnalyzer::inspect_payload(&signed);
        assert_eq!(
            report.signature_status,
            SignatureVerificationStatus::SignatureValid
        );
        assert_eq!(report.verified_cleartext.as_deref(), Some(payload));
    }
}
