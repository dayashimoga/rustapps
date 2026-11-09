#![forbid(unsafe_code)]
//! DevOps Secret Vault - Cryptographic Operations & Key Lifecycle Management

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_encryption::{
    decrypt_aes_gcm, derive_key_argon2id, encrypt_aes_gcm, generate_salt, Argon2Params,
    EncryptionError,
};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;
use crate::models::{EncryptedMetadataFields, SecretPayload};

/// Magic token encrypted to verify master password correctness without storing plaintext
pub const VAULT_VERIFIER_TOKEN: &[u8] = b"VAULT_VERIFIER_TOKEN_V1";

/// Zeroizing container for the 256-bit symmetric master key
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultMasterKey(pub [u8; 32]);

impl VaultMasterKey {
    /// Locks the master key bytes in RAM using OS memory page locking (VirtualLock / mlock)
    pub fn lock_in_memory(&self) -> shared_encryption::LockedKey<32> {
        shared_encryption::LockedKey::new(self.0)
    }

    /// Derive an isolated 256-bit key used exclusively for blind-indexing metadata at rest
    pub fn derive_blind_index_key(&self) -> [u8; 32] {
        hmac_sha256(&self.0, b"VAULT_BLIND_INDEX_DERIVATION_KEY_V1")
    }
}

/// Constant-time RFC 2104 compliant HMAC-SHA256 implementation
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k_padded = [0u8; 64];
    if key.len() > 64 {
        let hash = Sha256::digest(key);
        k_padded[..32].copy_from_slice(&hash);
    } else {
        k_padded[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k_padded[i];
        opad[i] ^= k_padded[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let result = outer.finalize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Compute a deterministic HMAC-SHA256 blind index token for searching metadata at rest
pub fn compute_blind_index(blind_key: &[u8; 32], term: &str) -> String {
    let normalized = term.trim().to_lowercase();
    let hash = hmac_sha256(blind_key, normalized.as_bytes());
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Encrypt secret title using AES-256-GCM and the master key
pub fn encrypt_title(master_key: &VaultMasterKey, title: &str) -> Result<Vec<u8>, VaultError> {
    let ciphertext =
        encrypt_aes_gcm(&master_key.0, title.as_bytes()).map_err(VaultError::Crypto)?;
    Ok(ciphertext)
}

/// Decrypt secret title using AES-256-GCM and the master key
pub fn decrypt_title(master_key: &VaultMasterKey, encrypted: &[u8]) -> Result<String, VaultError> {
    let decrypted = decrypt_aes_gcm(&master_key.0, encrypted).map_err(|e| match e {
        EncryptionError::DecryptionFailed => VaultError::Crypto(EncryptionError::DecryptionFailed),
        other => VaultError::Crypto(other),
    })?;
    let title = String::from_utf8(decrypted.as_slice().to_vec())
        .map_err(|e| VaultError::ValidationError(format!("Invalid UTF-8 in title: {}", e)))?;
    Ok(title)
}

impl std::fmt::Debug for VaultMasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VaultMasterKey([REDACTED])")
    }
}

/// Metadata stored in database for key derivation and verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyDerivationMetadata {
    pub salt: [u8; 16],
    pub verifier_ciphertext: Vec<u8>,
    pub memory_cost_kib: u32,
    pub time_cost_iterations: u32,
    pub parallelism: u32,
}

impl VaultKeyDerivationMetadata {
    /// Create new derivation metadata for a master password
    pub fn create(
        master_password: &str,
        params: &Argon2Params,
    ) -> Result<(Self, VaultMasterKey), VaultError> {
        let salt = generate_salt();
        let key_bytes = derive_key_argon2id(master_password.as_bytes(), &salt, params)
            .map_err(VaultError::Crypto)?;

        let master_key = VaultMasterKey(key_bytes);

        // Encrypt the known verifier token
        let verifier_ciphertext =
            encrypt_aes_gcm(&master_key.0, VAULT_VERIFIER_TOKEN).map_err(VaultError::Crypto)?;

        let meta = Self {
            salt,
            verifier_ciphertext,
            memory_cost_kib: params.memory_cost_kib,
            time_cost_iterations: params.time_cost_iterations,
            parallelism: params.parallelism,
        };

        Ok((meta, master_key))
    }

    /// Verify a master password against this derivation metadata and return the derived key
    pub fn unlock(&self, master_password: &str) -> Result<VaultMasterKey, VaultError> {
        let params = Argon2Params {
            memory_cost_kib: self.memory_cost_kib,
            time_cost_iterations: self.time_cost_iterations,
            parallelism: self.parallelism,
        };

        let key_bytes = derive_key_argon2id(master_password.as_bytes(), &self.salt, &params)
            .map_err(VaultError::Crypto)?;

        let candidate_key = VaultMasterKey(key_bytes);

        // Verify key by attempting to decrypt the verifier token
        self.verify_master_key(candidate_key)
    }

    /// Verify an already derived or reconstructed candidate master key against the verifier token
    pub fn verify_master_key(
        &self,
        candidate_key: VaultMasterKey,
    ) -> Result<VaultMasterKey, VaultError> {
        match decrypt_aes_gcm(&candidate_key.0, &self.verifier_ciphertext) {
            Ok(decrypted) if decrypted.as_slice() == VAULT_VERIFIER_TOKEN => Ok(candidate_key),
            _ => Err(VaultError::InvalidMasterPassword),
        }
    }
}

/// Encrypt a sensitive secret payload using AES-256-GCM and the master key
pub fn encrypt_secret_payload(
    master_key: &VaultMasterKey,
    payload: &SecretPayload,
) -> Result<Vec<u8>, VaultError> {
    payload.validate()?;
    let json_bytes = serde_json::to_vec(payload)?;
    let ciphertext = encrypt_aes_gcm(&master_key.0, &json_bytes).map_err(VaultError::Crypto)?;
    Ok(ciphertext)
}

/// Decrypt an encrypted secret payload using AES-256-GCM and the master key
pub fn decrypt_secret_payload(
    master_key: &VaultMasterKey,
    encrypted: &[u8],
) -> Result<SecretPayload, VaultError> {
    let decrypted = decrypt_aes_gcm(&master_key.0, encrypted).map_err(|e| match e {
        EncryptionError::DecryptionFailed => VaultError::Crypto(EncryptionError::DecryptionFailed),
        other => VaultError::Crypto(other),
    })?;

    let payload: SecretPayload = serde_json::from_slice(decrypted.as_slice())?;
    payload.validate()?;
    Ok(payload)
}

/// Encrypt sensitive metadata fields (username, url, notes, tags) using AES-256-GCM and the master key
pub fn encrypt_metadata_fields(
    master_key: &VaultMasterKey,
    fields: &EncryptedMetadataFields,
) -> Result<Vec<u8>, VaultError> {
    let json_bytes = serde_json::to_vec(fields)?;
    let ciphertext = encrypt_aes_gcm(&master_key.0, &json_bytes).map_err(VaultError::Crypto)?;
    Ok(ciphertext)
}

/// Decrypt sensitive metadata fields using AES-256-GCM and the master key
pub fn decrypt_metadata_fields(
    master_key: &VaultMasterKey,
    encrypted: &[u8],
) -> Result<EncryptedMetadataFields, VaultError> {
    let decrypted = decrypt_aes_gcm(&master_key.0, encrypted).map_err(|e| match e {
        EncryptionError::DecryptionFailed => VaultError::Crypto(EncryptionError::DecryptionFailed),
        other => VaultError::Crypto(other),
    })?;

    let fields: EncryptedMetadataFields = serde_json::from_slice(decrypted.as_slice())?;
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_init_and_unlock_success() {
        let params = Argon2Params::fast_for_tests();
        let password = "SuperSecretMasterPassword2026!";
        let (meta, _key) = VaultKeyDerivationMetadata::create(password, &params).unwrap();

        let unlocked_key = meta.unlock(password).expect("Unlock must succeed");
        assert_ne!(unlocked_key.0, [0u8; 32]);
    }

    #[test]
    fn test_vault_unlock_wrong_password_fails() {
        let params = Argon2Params::fast_for_tests();
        let (meta, _key) =
            VaultKeyDerivationMetadata::create("CorrectPassword123", &params).unwrap();

        let err = meta.unlock("WrongPassword999").unwrap_err();
        assert!(matches!(err, VaultError::InvalidMasterPassword));
    }

    #[test]
    fn test_secret_payload_encryption_roundtrip() {
        let master_key = VaultMasterKey([0x55u8; 32]);
        let original_payload = SecretPayload::Password {
            password: "MyDatabasePassword!@#".to_string(),
        };

        let encrypted = encrypt_secret_payload(&master_key, &original_payload).unwrap();
        assert_ne!(encrypted.as_slice(), b"MyDatabasePassword!@#");

        let decrypted = decrypt_secret_payload(&master_key, &encrypted).unwrap();
        assert_eq!(original_payload, decrypted);
    }

    #[test]
    fn test_metadata_fields_encryption_roundtrip() {
        let master_key = VaultMasterKey([0x77u8; 32]);
        let original_fields = EncryptedMetadataFields {
            tags: vec!["k8s".to_string(), "prod".to_string()],
            username: Some("admin_user".to_string()),
            url: Some("https://cluster.internal".to_string()),
            notes: Some("High privilege token".to_string()),
        };

        let encrypted = encrypt_metadata_fields(&master_key, &original_fields).unwrap();
        let decrypted = decrypt_metadata_fields(&master_key, &encrypted).unwrap();
        assert_eq!(original_fields, decrypted);
    }

    #[test]
    fn test_master_key_memory_locking() {
        let master_key = VaultMasterKey([0x99u8; 32]);
        let locked = master_key.lock_in_memory();
        assert_eq!(locked.as_bytes(), &master_key.0);
    }
    #[test]
    fn test_rfc2202_hmac_sha256_known_vector_1() {
        let key = [0x0bu8; 20];
        let data = b"Hi There";
        let hmac = hmac_sha256(&key, data);
        let hex: String = hmac.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(
            hex,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn test_blind_index_consistency_and_case_insensitivity() {
        let master_key = VaultMasterKey([0x42u8; 32]);
        let blind_key = master_key.derive_blind_index_key();

        let index1 = compute_blind_index(&blind_key, "AWS_PROD_ROOT_KEY");
        let index2 = compute_blind_index(&blind_key, "aws_prod_root_key ");
        assert_eq!(
            index1, index2,
            "Blind indices must be case and whitespace normalized"
        );

        let index_diff = compute_blind_index(&blind_key, "GCP_PROD_ROOT_KEY");
        assert_ne!(
            index1, index_diff,
            "Different terms must have different blind indices"
        );
    }

    #[test]
    fn test_title_encryption_roundtrip() {
        let master_key = VaultMasterKey([0x33u8; 32]);
        let original_title = "Production Kubernetes Cluster CA";
        let encrypted = encrypt_title(&master_key, original_title).unwrap();
        assert_ne!(encrypted.as_slice(), original_title.as_bytes());

        let decrypted = decrypt_title(&master_key, &encrypted).unwrap();
        assert_eq!(original_title, decrypted);
    }
}
