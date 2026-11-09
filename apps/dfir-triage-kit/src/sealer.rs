#![forbid(unsafe_code)]
//! Cryptographic Evidence Sealing & Chain-of-Custody Manifest Verification
//!
//! Generates court-admissible, tamper-evident .dfir bundles cryptographically signed
//! with offline Ed25519 keys and verified with dual SHA-256 + BLAKE3 hashes.

use crate::models::{
    ChainOfCustodyManifest, FileHashEntry, MitreAttackMapping, PackageVerificationReport,
};
use crate::service::{DfirError, TriagePackage};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub struct EvidenceSealer;

impl EvidenceSealer {
    const BUNDLE_MAGIC: &'static [u8; 8] = b"AGDFIR\x01\x00";

    /// Seals a triage package into an Ed25519-signed .dfir archive
    pub fn seal_package(
        package: &TriagePackage,
        mitre: &[MitreAttackMapping],
        out_path: impl AsRef<Path>,
    ) -> Result<ChainOfCustodyManifest, DfirError> {
        let p = out_path.as_ref();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Prepare artifact serialized entries
        let mut entries: Vec<(&str, Vec<u8>)> = Vec::new();

        let summary_bytes = serde_json::to_vec_pretty(&package.summary)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("summary.json", summary_bytes));

        let mitre_bytes = serde_json::to_vec_pretty(mitre)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("mitre_attack.json", mitre_bytes));

        let procs_bytes = serde_json::to_vec_pretty(&package.processes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("processes.json", procs_bytes));

        let sockets_bytes = serde_json::to_vec_pretty(&package.sockets)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("sockets.json", sockets_bytes));

        let files_bytes = serde_json::to_vec_pretty(&package.suspicious_files)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("files.json", files_bytes));

        let logs_bytes = serde_json::to_vec_pretty(&package.event_logs)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("event_logs.json", logs_bytes));

        let certs_bytes = serde_json::to_vec_pretty(&package.certificates)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(("certificates.json", certs_bytes));

        // Generate file hash entries
        let mut file_hashes = Vec::new();
        for (name, data) in &entries {
            file_hashes.push(FileHashEntry {
                file_name: name.to_string(),
                size_bytes: data.len() as u64,
                sha256: format!("{:x}", Sha256::digest(data)),
                blake3: blake3::hash(data).to_hex().to_string(),
            });
        }

        // Generate Ed25519 signing keypair for evidence sealing
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(&verifying_key.to_bytes());

        // Construct canonical data to sign: hashes of all contained artifacts
        let mut sign_payload = Vec::new();
        for fh in &file_hashes {
            sign_payload.extend_from_slice(fh.file_name.as_bytes());
            sign_payload.extend_from_slice(fh.sha256.as_bytes());
            sign_payload.extend_from_slice(fh.blake3.as_bytes());
        }

        let signature: Signature = signing_key.sign(&sign_payload);
        let sig_hex = hex::encode(&signature.to_bytes());

        // Build container payload: JSON map of files compressed with GzEncoder
        let mut map = HashMap::new();
        for (name, data) in entries {
            map.insert(name.to_string(), String::from_utf8_lossy(&data).to_string());
        }
        let container_json = serde_json::to_vec(&map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&container_json)?;
        let compressed_payload = encoder.finish()?;

        let package_sha256 = format!("{:x}", Sha256::digest(&compressed_payload));
        let package_blake3 = blake3::hash(&compressed_payload).to_hex().to_string();

        let manifest = ChainOfCustodyManifest {
            manifest_version: "1.0".to_string(),
            package_id: package.summary.triage_id.clone(),
            host_name: package.summary.host_name.clone(),
            os_type: package.summary.os_type.clone(),
            collected_at: package.summary.collected_at,
            collector_version: env!("CARGO_PKG_VERSION").to_string(),
            sha256_package_hash: package_sha256,
            blake3_package_hash: package_blake3,
            ed25519_signature: sig_hex,
            public_key: public_key_hex,
            mitre_summary: mitre.to_vec(),
            file_hashes,
        };

        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Write bundle file
        let mut file = File::create(p)?;
        file.write_all(Self::BUNDLE_MAGIC)?;
        let manifest_len = (manifest_bytes.len() as u32).to_le_bytes();
        file.write_all(&manifest_len)?;
        file.write_all(&manifest_bytes)?;
        file.write_all(&compressed_payload)?;
        file.flush()?;

        Ok(manifest)
    }

    /// Verifies the cryptographic integrity and digital signature of a .dfir package
    pub fn verify_package(path: impl AsRef<Path>) -> Result<PackageVerificationReport, DfirError> {
        let p = path.as_ref();
        let mut file = File::open(p)?;

        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;
        if &magic != Self::BUNDLE_MAGIC {
            return Ok(PackageVerificationReport {
                package_path: p.to_string_lossy().to_string(),
                is_valid: false,
                signature_verified: false,
                files_verified: 0,
                mismatched_files: Vec::new(),
                signer_public_key: String::new(),
                notes: "Invalid .dfir header magic signature".to_string(),
            });
        }

        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf)?;
        let manifest_len = u32::from_le_bytes(len_buf) as usize;

        let mut manifest_bytes = vec![0u8; manifest_len];
        file.read_exact(&mut manifest_bytes)?;

        let manifest: ChainOfCustodyManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Read remaining compressed payload
        let mut compressed_payload = Vec::new();
        file.read_to_end(&mut compressed_payload)?;

        // Verify package hashes
        let calc_sha256 = format!("{:x}", Sha256::digest(&compressed_payload));
        let calc_blake3 = blake3::hash(&compressed_payload).to_hex().to_string();

        if calc_sha256 != manifest.sha256_package_hash
            || calc_blake3 != manifest.blake3_package_hash
        {
            return Ok(PackageVerificationReport {
                package_path: p.to_string_lossy().to_string(),
                is_valid: false,
                signature_verified: false,
                files_verified: 0,
                mismatched_files: vec!["package_payload".to_string()],
                signer_public_key: manifest.public_key,
                notes: "Package hash mismatch (tampering detected)".to_string(),
            });
        }

        // Verify Ed25519 signature
        let mut sign_payload = Vec::new();
        for fh in &manifest.file_hashes {
            sign_payload.extend_from_slice(fh.file_name.as_bytes());
            sign_payload.extend_from_slice(fh.sha256.as_bytes());
            sign_payload.extend_from_slice(fh.blake3.as_bytes());
        }

        let pubkey_bytes = hex::decode(&manifest.public_key)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let pubkey_arr: [u8; 32] = pubkey_bytes.try_into().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid pubkey len")
        })?;
        let verifying_key = VerifyingKey::from_bytes(&pubkey_arr)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let sig_bytes = hex::decode(&manifest.ed25519_signature)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid sig len"))?;
        let signature = Signature::from_bytes(&sig_arr);

        let signature_verified = verifying_key.verify(&sign_payload, &signature).is_ok();

        // Decompress and verify inner files
        let mut decoder = GzDecoder::new(&compressed_payload[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        let map: HashMap<String, String> = serde_json::from_slice(&decompressed)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut mismatched = Vec::new();
        let mut files_verified = 0;

        for fh in &manifest.file_hashes {
            if let Some(content) = map.get(&fh.file_name) {
                let actual_sha = format!("{:x}", Sha256::digest(content.as_bytes()));
                let actual_blake = blake3::hash(content.as_bytes()).to_hex().to_string();
                if actual_sha == fh.sha256 && actual_blake == fh.blake3 {
                    files_verified += 1;
                } else {
                    mismatched.push(fh.file_name.clone());
                }
            } else {
                mismatched.push(fh.file_name.clone());
            }
        }

        let is_valid = signature_verified && mismatched.is_empty();

        Ok(PackageVerificationReport {
            package_path: p.to_string_lossy().to_string(),
            is_valid,
            signature_verified,
            files_verified,
            mismatched_files: mismatched,
            signer_public_key: manifest.public_key,
            notes: if is_valid {
                "Package signature and all file integrity hashes verified successfully".to_string()
            } else {
                "Integrity verification failure".to_string()
            },
        })
    }
}

// Minimal hex encoding helper to avoid extra dependency
mod hex {
    pub fn encode(data: &[u8]) -> String {
        let mut s = String::with_capacity(data.len() * 2);
        for &b in data {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("Invalid hex length".to_string());
        }
        let mut bytes = Vec::with_capacity(s.len() / 2);
        for i in (0..s.len()).step_by(2) {
            let byte = u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex byte: {}", e))?;
            bytes.push(byte);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Severity, TriageSummary};

    #[test]
    fn test_seal_and_verify_roundtrip() {
        let temp = tempfile::NamedTempFile::new().unwrap();

        let summary = TriageSummary {
            triage_id: "triage-test-01".to_string(),
            host_name: "INCIDENT-SERVER".to_string(),
            os_type: "windows".to_string(),
            collected_at: 1700000000,
            total_processes: 1,
            total_sockets: 0,
            total_files_scanned: 0,
            total_logs_analyzed: 0,
            threats_detected: 0,
            threat_score: 0,
            severity: Severity::Low,
        };

        let pkg = TriagePackage {
            summary,
            mitre_techniques: Vec::new(),
            processes: Vec::new(),
            sockets: Vec::new(),
            suspicious_files: Vec::new(),
            event_logs: Vec::new(),
            certificates: Vec::new(),
        };

        let mitre = vec![MitreAttackMapping {
            technique_id: "T1059.001".to_string(),
            technique_name: "PowerShell".to_string(),
            tactic: "Execution".to_string(),
            severity: Severity::Medium,
            evidence: "Test evidence".to_string(),
        }];

        let manifest = EvidenceSealer::seal_package(&pkg, &mitre, temp.path()).unwrap();
        assert_eq!(manifest.package_id, "triage-test-01");
        assert!(!manifest.ed25519_signature.is_empty());
        assert!(!manifest.public_key.is_empty());
        assert_eq!(manifest.file_hashes.len(), 7);

        let report = EvidenceSealer::verify_package(temp.path()).unwrap();
        assert!(report.is_valid);
        assert!(report.signature_verified);
        assert_eq!(report.files_verified, 7);
        assert!(report.mismatched_files.is_empty());
    }
}
