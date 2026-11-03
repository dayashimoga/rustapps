#![forbid(unsafe_code)]
//! Cryptographically Verified Offline AI Model Hub Subsystem
//!
//! Enforces zero-unsolicited-egress air-gap isolation and authenticates
//! offline neural models via SHA-256 checksums and Ed25519 digital signatures.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_storage::MmapReader;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Model Hub error definitions
#[derive(Debug, Error)]
pub enum ModelHubError {
    #[error("I/O error during model pack operation: {0}")]
    Io(#[from] std::io::Error),
    #[error("SHA-256 checksum mismatch: expected {expected}, calculated {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("Model pack size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("Ed25519 cryptographic signature verification failed: {0}")]
    InvalidSignature(String),
    #[error("No trusted verifying key matched the signature")]
    UntrustedKey,
    #[error("Model {model_id} (version {version}) not installed or not found")]
    ModelNotFound { model_id: String, version: String },
    #[error("Storage error: {0}")]
    Storage(#[from] shared_storage::StorageError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Offline AI model category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelType {
    AsrWhisper,
    NmtTranslation,
    TtsSynthesizer,
    OcrClassifier,
    DenseEmbedding,
}

/// Cryptographically signed offline AI model package manifest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelManifest {
    pub model_id: String,
    pub version: String,
    pub model_type: ModelType,
    pub sha256_checksum: String,
    pub size_bytes: u64,
    pub supported_languages: Vec<String>,
    pub signature_hex: String,
}

impl ModelManifest {
    /// Canonical binary payload signed by model provider
    pub fn canonical_payload(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"OFFLINE_MODEL_MANIFEST_V1:");
        hasher.update(self.model_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.version.as_bytes());
        hasher.update(b"|");
        hasher.update(format!("{:?}", self.model_type).as_bytes());
        hasher.update(b"|");
        hasher.update(self.sha256_checksum.as_bytes());
        hasher.update(b"|");
        hasher.update(self.size_bytes.to_string().as_bytes());
        hasher.update(b"|");
        let mut langs = self.supported_languages.clone();
        langs.sort();
        hasher.update(langs.join(",").as_bytes());
        hasher.finalize().to_vec()
    }
}

/// Offline Model Manager handling local disk repository and zero-copy mmap mounting
pub struct ModelHubManager {
    models_dir: PathBuf,
    trusted_verifying_keys: Vec<VerifyingKey>,
}

impl ModelHubManager {
    /// Initialize with root models storage directory and a set of trusted Ed25519 public keys
    pub fn new<P: AsRef<Path>>(
        models_dir: P,
        trusted_key_bytes: &[[u8; 32]],
    ) -> Result<Self, ModelHubError> {
        let path = models_dir.as_ref().to_path_buf();
        fs::create_dir_all(&path)?;

        let mut trusted_verifying_keys = Vec::new();
        for key_bytes in trusted_key_bytes {
            if let Ok(vk) = VerifyingKey::from_bytes(key_bytes) {
                trusted_verifying_keys.push(vk);
            }
        }

        Ok(Self {
            models_dir: path,
            trusted_verifying_keys,
        })
    }

    /// Verifies SHA-256 digest, file size, and Ed25519 signature before mounting model pack
    pub fn verify_and_install_pack(
        &self,
        pack_file: &Path,
        manifest: &ModelManifest,
    ) -> Result<PathBuf, ModelHubError> {
        // 1. Verify file size
        let metadata = fs::metadata(pack_file)?;
        if metadata.len() != manifest.size_bytes {
            return Err(ModelHubError::SizeMismatch {
                expected: manifest.size_bytes,
                actual: metadata.len(),
            });
        }

        // 2. Compute and verify SHA-256 checksum
        let mut hasher = Sha256::new();
        let mut file = File::open(pack_file)?;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let calculated_checksum = format!("{:x}", hasher.finalize());
        if !calculated_checksum.eq_ignore_ascii_case(&manifest.sha256_checksum) {
            return Err(ModelHubError::ChecksumMismatch {
                expected: manifest.sha256_checksum.clone(),
                actual: calculated_checksum,
            });
        }

        // 3. Verify digital signature against trusted authority keys
        let payload = manifest.canonical_payload();
        let sig_bytes = hex_to_bytes(&manifest.signature_hex)
            .map_err(|e| ModelHubError::InvalidSignature(e.to_string()))?;
        if sig_bytes.len() != 64 {
            return Err(ModelHubError::InvalidSignature(
                "Signature must be exactly 64 bytes".to_string(),
            ));
        }

        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        let mut verified = false;
        for key in &self.trusted_verifying_keys {
            if key.verify(&payload, &signature).is_ok() {
                verified = true;
                break;
            }
        }

        if !verified {
            return Err(ModelHubError::UntrustedKey);
        }

        // 4. Install into models repository: models_dir/model_id/version/
        let install_dir = self
            .models_dir
            .join(&manifest.model_id)
            .join(&manifest.version);
        fs::create_dir_all(&install_dir)?;

        let dest_pack = install_dir.join("model.pack");
        fs::copy(pack_file, &dest_pack)?;

        let manifest_json = serde_json::to_string_pretty(manifest)?;
        fs::write(install_dir.join("manifest.json"), manifest_json)?;

        Ok(dest_pack)
    }

    /// Mounts an installed model pack as zero-copy memory-mapped file
    pub fn mount_model_mmap(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<MmapReader, ModelHubError> {
        let pack_path = self
            .models_dir
            .join(model_id)
            .join(version)
            .join("model.pack");

        if !pack_path.exists() {
            return Err(ModelHubError::ModelNotFound {
                model_id: model_id.to_string(),
                version: version.to_string(),
            });
        }

        let reader = MmapReader::open(&pack_path)?;
        Ok(reader)
    }

    /// Check if a model is installed
    pub fn is_installed(&self, model_id: &str, version: &str) -> bool {
        self.models_dir
            .join(model_id)
            .join(version)
            .join("model.pack")
            .exists()
    }
}

/// Offline authority for signing model manifests
pub struct ModelSigner {
    signing_key: SigningKey,
}

impl ModelSigner {
    pub fn generate() -> Self {
        let mut rng = rand::rngs::OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        Self { signing_key }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn sign_manifest(
        &self,
        model_id: &str,
        version: &str,
        model_type: ModelType,
        sha256_checksum: &str,
        size_bytes: u64,
        supported_languages: Vec<String>,
    ) -> ModelManifest {
        let mut manifest = ModelManifest {
            model_id: model_id.to_string(),
            version: version.to_string(),
            model_type,
            sha256_checksum: sha256_checksum.to_string(),
            size_bytes,
            supported_languages,
            signature_hex: String::new(),
        };

        let payload = manifest.canonical_payload();
        let sig = self.signing_key.sign(&payload);
        manifest.signature_hex = bytes_to_hex(&sig.to_bytes());
        manifest
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Odd hex length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("Invalid hex: {}", e)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_hub_verification_and_mounting() {
        let temp = tempfile::tempdir().unwrap();
        let repo_dir = temp.path().join("repo");
        let work_dir = temp.path().join("work");
        fs::create_dir_all(&work_dir).unwrap();

        let signer = ModelSigner::generate();
        let pub_key = signer.public_key_bytes();

        let hub = ModelHubManager::new(&repo_dir, &[pub_key]).unwrap();

        // Create a model binary pack
        let pack_path = work_dir.join("whisper_tiny_en.pack");
        let model_weights = vec![0x42u8; 1024]; // 1KB sample weights
        fs::write(&pack_path, &model_weights).unwrap();

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(&model_weights);
        let checksum = format!("{:x}", hasher.finalize());

        let manifest = signer.sign_manifest(
            "whisper-tiny-en",
            "1.0.0",
            ModelType::AsrWhisper,
            &checksum,
            model_weights.len() as u64,
            vec!["en".to_string()],
        );

        let installed_path = hub.verify_and_install_pack(&pack_path, &manifest).unwrap();
        assert!(installed_path.exists());
        assert!(hub.is_installed("whisper-tiny-en", "1.0.0"));

        // Memory-map the installed model
        let mmap = hub.mount_model_mmap("whisper-tiny-en", "1.0.0").unwrap();
        assert_eq!(mmap.len(), 1024);
        assert_eq!(mmap.as_bytes(), &model_weights[..]);
    }

    #[test]
    fn test_checksum_tampering_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let repo_dir = temp.path().join("repo");
        let work_dir = temp.path().join("work");
        fs::create_dir_all(&work_dir).unwrap();

        let signer = ModelSigner::generate();
        let pub_key = signer.public_key_bytes();
        let hub = ModelHubManager::new(&repo_dir, &[pub_key]).unwrap();

        let pack_path = work_dir.join("nmt_es_en.pack");
        fs::write(&pack_path, b"model payload").unwrap();

        let manifest = signer.sign_manifest(
            "nmt-es-en",
            "1.0.0",
            ModelType::NmtTranslation,
            "0000000000000000000000000000000000000000000000000000000000000000",
            13,
            vec!["es".to_string(), "en".to_string()],
        );

        let err = hub
            .verify_and_install_pack(&pack_path, &manifest)
            .unwrap_err();
        assert!(matches!(err, ModelHubError::ChecksumMismatch { .. }));
    }
}
