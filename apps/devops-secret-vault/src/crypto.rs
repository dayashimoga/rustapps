#![forbid(unsafe_code)]
//! DevOps Secret Vault - Cryptographic Operations & Key Lifecycle Management

use serde::{Deserialize, Serialize};
use shared_encryption::{
    decrypt_aes_gcm, derive_key_argon2id, encrypt_aes_gcm, generate_salt, Argon2Params,
    EncryptionError,
};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;
use crate::models::SecretPayload;

/// Magic token encrypted to verify master password correctness without storing plaintext
pub const VAULT_VERIFIER_TOKEN: &[u8] = b"VAULT_VERIFIER_TOKEN_V1";

/// Zeroizing container for the 256-bit symmetric master key
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultMasterKey(pub [u8; 32]);

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
}
