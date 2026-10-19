#![forbid(unsafe_code)]
//! Shared Updater Subsystem
//!
//! Provides cryptographically verified standalone update package verification
//! and staged atomic replacement with automated rollback protection.

use sha2::{Digest, Sha256};
use shared_encryption::verify_ed25519;
use std::path::Path;
use thiserror::Error;

/// Updater error definitions
#[derive(Debug, Error)]
pub enum UpdaterError {
    #[error("Serialization / Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("I/O error during update: {0}")]
    Io(#[from] std::io::Error),
    #[error("SHA-256 hash mismatch: expected {expected}, computed {computed}")]
    HashMismatch { expected: String, computed: String },
    #[error("Cryptographic signature verification failed for update package")]
    SignatureInvalid,
    #[error("Hex decoding error: {0}")]
    HexDecode(String),
}

/// Metadata manifest accompanying an update package
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub target_os: String,
    pub target_arch: String,
    pub sha256: String,
    pub signature_ed25519: String,
    pub release_notes: String,
}

/// Standalone package verifier and installer
pub struct StandaloneUpdater;

impl StandaloneUpdater {
    /// Compute the SHA-256 hex digest of a byte slice.
    pub fn compute_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Verify the integrity and authenticity of an update payload against its manifest.
    pub fn verify_package(
        manifest: &UpdateManifest,
        payload_bytes: &[u8],
        verifying_key_bytes: &[u8; 32],
    ) -> Result<(), UpdaterError> {
        // 1. Verify SHA-256 checksum
        let computed_hash = Self::compute_sha256(payload_bytes);
        if computed_hash.to_lowercase() != manifest.sha256.to_lowercase() {
            return Err(UpdaterError::HashMismatch {
                expected: manifest.sha256.clone(),
                computed: computed_hash,
            });
        }

        // 2. Verify Ed25519 detached signature
        let sig_bytes = Self::decode_hex_64(&manifest.signature_ed25519)?;
        let is_valid = verify_ed25519(verifying_key_bytes, payload_bytes, &sig_bytes)
            .map_err(|_| UpdaterError::SignatureInvalid)?;

        if !is_valid {
            return Err(UpdaterError::SignatureInvalid);
        }

        Ok(())
    }

    /// Perform staged atomic replacement of the current binary with rollback backup.
    pub fn staged_atomic_replace<P: AsRef<Path>>(
        current_binary_path: P,
        new_binary_bytes: &[u8],
    ) -> Result<(), UpdaterError> {
        let current_path = current_binary_path.as_ref();
        let parent = current_path.parent().unwrap_or_else(|| Path::new("."));

        let staged_path = parent.join(format!(
            ".{}.staged.tmp",
            current_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("app")
        ));
        let backup_path = parent.join(format!(
            "{}.backup.old",
            current_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("app")
        ));

        // Write staged binary
        std::fs::write(&staged_path, new_binary_bytes)?;

        // Rename current binary to backup
        if current_path.exists() {
            if let Err(e) = std::fs::rename(current_path, &backup_path) {
                let _ = std::fs::remove_file(&staged_path);
                return Err(UpdaterError::Io(e));
            }
        }

        // Move staged binary into production place
        if let Err(e) = std::fs::rename(&staged_path, current_path) {
            // Rollback from backup if staging move fails
            if backup_path.exists() {
                let _ = std::fs::rename(&backup_path, current_path);
            }
            return Err(UpdaterError::Io(e));
        }

        // Clean up backup after success
        let _ = std::fs::remove_file(&backup_path);

        Ok(())
    }

    fn decode_hex_64(hex_str: &str) -> Result<[u8; 64], UpdaterError> {
        let clean = hex_str.trim();
        if clean.len() != 128 {
            return Err(UpdaterError::HexDecode(
                "Signature hex must be exactly 128 characters (64 bytes)".to_string(),
            ));
        }

        let mut output = [0u8; 64];
        for i in 0..64 {
            let byte_str = &clean[i * 2..i * 2 + 2];
            output[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|e| UpdaterError::HexDecode(e.to_string()))?;
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_encryption::{generate_ed25519_keypair, sign_ed25519};

    #[test]
    fn test_verify_valid_package() {
        let (signing_key, verifying_key) = generate_ed25519_keypair();
        let payload = b"Production Binary v1.0.1 Payload";

        let hash = StandaloneUpdater::compute_sha256(payload);
        let sig_bytes = sign_ed25519(&signing_key, payload);
        let sig_hex = sig_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let manifest = UpdateManifest {
            version: "1.0.1".to_string(),
            target_os: "windows".to_string(),
            target_arch: "x86_64".to_string(),
            sha256: hash,
            signature_ed25519: sig_hex,
            release_notes: "Performance optimizations and bugfixes".to_string(),
        };

        let result =
            StandaloneUpdater::verify_package(&manifest, payload, &verifying_key.to_bytes());
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_tampered_package_fails() {
        let (signing_key, verifying_key) = generate_ed25519_keypair();
        let payload = b"Authentic Payload";
        let tampered_payload = b"Malicious Payload";

        let hash = StandaloneUpdater::compute_sha256(payload);
        let sig_bytes = sign_ed25519(&signing_key, payload);
        let sig_hex = sig_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let manifest = UpdateManifest {
            version: "1.0.1".to_string(),
            target_os: "windows".to_string(),
            target_arch: "x86_64".to_string(),
            sha256: hash,
            signature_ed25519: sig_hex,
            release_notes: "".to_string(),
        };

        // Fails with hash mismatch
        let result = StandaloneUpdater::verify_package(
            &manifest,
            tampered_payload,
            &verifying_key.to_bytes(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_staged_atomic_replace() {
        let temp = tempfile::tempdir().unwrap();
        let bin_path = temp.path().join("app.exe");
        std::fs::write(&bin_path, "Old Binary v1.0.0").unwrap();

        let new_binary = b"New Binary v1.0.1";
        StandaloneUpdater::staged_atomic_replace(&bin_path, new_binary).unwrap();

        let updated = std::fs::read(&bin_path).unwrap();
        assert_eq!(updated, new_binary);
    }
}
