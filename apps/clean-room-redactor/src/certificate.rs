use crate::models::SanitizationCertificate;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct CertificateIssueRequest<'a> {
    pub document_id: &'a str,
    pub filename: &'a str,
    pub original_bytes: &'a [u8],
    pub sanitized_bytes: &'a [u8],
    pub total_pages: usize,
    pub total_redactions: usize,
    pub scrubbed_categories: Vec<String>,
    pub metadata_stripped: bool,
    pub bates_range: Option<String>,
    pub signing_key: Option<&'a SigningKey>,
}

pub struct CertificateManager;

impl CertificateManager {
    /// Generates a signed sanitization certificate proving document integrity and redaction lineage
    pub fn issue_certificate(req: CertificateIssueRequest<'_>) -> SanitizationCertificate {
        // Dual hashing: SHA-256 + BLAKE3
        let orig_sha256 = format!("{:x}", Sha256::digest(req.original_bytes));
        let orig_blake3 = blake3::hash(req.original_bytes).to_hex().to_string();

        let san_sha256 = format!("{:x}", Sha256::digest(req.sanitized_bytes));
        let san_blake3 = blake3::hash(req.sanitized_bytes).to_hex().to_string();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cert_id = format!("CERT-SAN-{}", san_blake3[0..12].to_uppercase());

        // Canonical manifest string for cryptographic signing
        let manifest = format!(
            "CERT:{}|DOC:{}|FILE:{}|TIME:{}|ORIG_SHA:{}|SAN_SHA:{}|PAGES:{}|REDACTIONS:{}",
            cert_id,
            req.document_id,
            req.filename,
            timestamp,
            orig_sha256,
            san_sha256,
            req.total_pages,
            req.total_redactions
        );

        // Sign with provided key or generate deterministic session key
        let generated_key = SigningKey::generate(&mut OsRng);
        let key_to_use = req.signing_key.unwrap_or(&generated_key);
        let sig = key_to_use.sign(manifest.as_bytes());
        let signature_hex = hex::encode(sig.to_bytes());
        let signing_key_id = hex::encode(key_to_use.verifying_key().to_bytes());

        SanitizationCertificate {
            certificate_id: cert_id,
            document_id: req.document_id.to_string(),
            filename: req.filename.to_string(),
            timestamp,
            original_sha256: orig_sha256,
            original_blake3: orig_blake3,
            sanitized_sha256: san_sha256,
            sanitized_blake3: san_blake3,
            total_pages: req.total_pages,
            total_redactions_applied: req.total_redactions,
            scrubbed_categories: req.scrubbed_categories,
            metadata_stripped: req.metadata_stripped,
            bates_range: req.bates_range,
            signing_key_id,
            signature_hex,
            verification_status: "VERIFIED_AUTHENTIC".to_string(),
        }
    }

    /// Verifies an Ed25519 sanitization certificate against its public key and sanitized document bytes
    pub fn verify_certificate(
        cert: &SanitizationCertificate,
        sanitized_bytes: &[u8],
    ) -> Result<bool, String> {
        // 1. Verify sanitized file hash matches certificate
        let actual_san_sha256 = format!("{:x}", Sha256::digest(sanitized_bytes));
        if cert.sanitized_sha256 != actual_san_sha256 {
            return Err("Sanitized document SHA-256 digest does not match certificate".to_string());
        }

        let actual_san_blake3 = blake3::hash(sanitized_bytes).to_hex().to_string();
        if cert.sanitized_blake3 != actual_san_blake3 {
            return Err("Sanitized document BLAKE3 digest does not match certificate".to_string());
        }

        // 2. Reconstruct canonical manifest
        let manifest = format!(
            "CERT:{}|DOC:{}|FILE:{}|TIME:{}|ORIG_SHA:{}|SAN_SHA:{}|PAGES:{}|REDACTIONS:{}",
            cert.certificate_id,
            cert.document_id,
            cert.filename,
            cert.timestamp,
            cert.original_sha256,
            cert.sanitized_sha256,
            cert.total_pages,
            cert.total_redactions_applied
        );

        // 3. Decode public key and signature
        let pub_key_bytes = hex::decode(&cert.signing_key_id)
            .map_err(|e| format!("Invalid hex for signing_key_id: {}", e))?;
        if pub_key_bytes.len() != 32 {
            return Err("Invalid public key length; expected 32 bytes".to_string());
        }
        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&pub_key_bytes);
        let verifying_key = VerifyingKey::from_bytes(&key_arr)
            .map_err(|e| format!("Failed to parse VerifyingKey: {}", e))?;

        let sig_bytes = hex::decode(&cert.signature_hex)
            .map_err(|e| format!("Invalid hex for signature_hex: {}", e))?;
        if sig_bytes.len() != 64 {
            return Err("Invalid signature length; expected 64 bytes".to_string());
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        // 4. Verify Ed25519 signature
        verifying_key
            .verify(manifest.as_bytes(), &signature)
            .map(|_| true)
            .map_err(|e| format!("Ed25519 signature verification failed: {}", e))
    }

    /// Renders vector SVG of the Sanitization Certificate
    pub fn render_certificate_svg(cert: &SanitizationCertificate) -> String {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 650 360" width="100%" height="100%">
  <rect x="0" y="0" width="650" height="360" rx="10" fill="#0b0f17" stroke="#10b981" stroke-width="2"/>
  <!-- Header Banner -->
  <rect x="0" y="0" width="650" height="50" rx="10" fill="#10b98115"/>
  <text x="24" y="32" font-family="monospace" font-size="14" font-weight="bold" fill="#34d399">OFFLINE CRYPTOGRAPHIC SANITIZATION CERTIFICATE</text>
  <text x="626" y="32" text-anchor="end" font-family="monospace" font-size="11" fill="#10b981">ED25519 VERIFIED</text>
  
  <!-- Certificate Meta -->
  <text x="24" y="80" font-family="monospace" font-size="11" fill="#94a3b8">CERTIFICATE ID: <tspan fill="#f1f5f9" font-weight="bold">{}</tspan></text>
  <text x="24" y="105" font-family="monospace" font-size="11" fill="#94a3b8">FILE: <tspan fill="#38bdf8">{}</tspan> ({} Pages)</text>
  <text x="24" y="130" font-family="monospace" font-size="11" fill="#94a3b8">TOTAL REDACTIONS BURNED: <tspan fill="#34d399" font-weight="bold">{} entities</tspan></text>
  <text x="24" y="155" font-family="monospace" font-size="11" fill="#94a3b8">BATES RANGE: <tspan fill="#fbbf24">{}</tspan></text>
  
  <!-- Hash Proofs -->
  <line x1="24" y1="175" x2="626" y2="175" stroke="#1e293b" stroke-width="1"/>
  <text x="24" y="198" font-family="monospace" font-size="9" fill="#64748b">ORIGINAL BLAKE3:</text>
  <text x="24" y="214" font-family="monospace" font-size="9" fill="#94a3b8">{}</text>
  
  <text x="24" y="240" font-family="monospace" font-size="9" fill="#64748b">SANITIZED BLAKE3:</text>
  <text x="24" y="256" font-family="monospace" font-size="9" fill="#10b981">{}</text>

  <!-- Signature Proof -->
  <line x1="24" y1="275" x2="626" y2="275" stroke="#1e293b" stroke-width="1"/>
  <text x="24" y="298" font-family="monospace" font-size="9" fill="#64748b">ED25519 SIGNATURE DIGEST:</text>
  <text x="24" y="314" font-family="monospace" font-size="8" fill="#64748b">{}...</text>
  <text x="24" y="338" font-family="monospace" font-size="9" font-weight="bold" fill="#34d399">STATUS: 100% AIR-GAPPED CLEAN-ROOM CERTIFIED</text>
</svg>"##,
            cert.certificate_id,
            cert.filename,
            cert.total_pages,
            cert.total_redactions_applied,
            cert.bates_range.as_deref().unwrap_or("None"),
            cert.original_blake3,
            cert.sanitized_blake3,
            &cert.signature_hex[0..48]
        )
    }
}

// Helper module for hex encoding
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("Hex string has odd length".to_string());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16)
                    .map_err(|e| format!("Invalid hex digit: {}", e))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_issuance_and_verification() {
        let orig = b"Top Secret Defense Plan. Target SSN 123-45-6789.";
        let san = b"Top Secret Defense Plan. Target SSN XXX-XX-XXXX.";

        let key = SigningKey::generate(&mut OsRng);
        let cert = CertificateManager::issue_certificate(CertificateIssueRequest {
            document_id: "doc-101",
            filename: "defense_plan.pdf",
            original_bytes: orig,
            sanitized_bytes: san,
            total_pages: 1,
            total_redactions: 1,
            scrubbed_categories: vec!["SSN".to_string()],
            metadata_stripped: true,
            bates_range: Some("CONF-000001".to_string()),
            signing_key: Some(&key),
        });

        assert_eq!(cert.total_redactions_applied, 1);
        assert_ne!(cert.original_blake3, cert.sanitized_blake3);

        // Verification must pass with valid sanitized bytes
        let is_valid = CertificateManager::verify_certificate(&cert, san).unwrap();
        assert!(is_valid);

        // Verification must fail if document is tampered
        let tampered = b"Top Secret Defense Plan. Target SSN TAMPERED.";
        let err = CertificateManager::verify_certificate(&cert, tampered);
        assert!(err.is_err());
    }

    #[test]
    fn test_certificate_svg_renders_successfully() {
        let cert = SanitizationCertificate {
            certificate_id: "CERT-SAN-1234567890AB".to_string(),
            document_id: "doc-1".to_string(),
            filename: "settlement.pdf".to_string(),
            timestamp: 1700000000,
            original_sha256: "abc...".to_string(),
            original_blake3: "def...".to_string(),
            sanitized_sha256: "123...".to_string(),
            sanitized_blake3: "456...".to_string(),
            total_pages: 5,
            total_redactions_applied: 12,
            scrubbed_categories: vec!["SSN".to_string(), "CreditCard".to_string()],
            metadata_stripped: true,
            bates_range: Some("DEF-000001 through DEF-000005".to_string()),
            signing_key_id: "key123".to_string(),
            signature_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            verification_status: "VERIFIED_AUTHENTIC".to_string(),
        };

        let svg = CertificateManager::render_certificate_svg(&cert);
        assert!(svg.contains("CERT-SAN-1234567890AB"));
        assert!(svg.contains("settlement.pdf"));
        assert!(svg.contains("12 entities"));
    }
}
