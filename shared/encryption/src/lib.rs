#![deny(unsafe_code)]
//! Shared Cryptographic Engine
//!
//! Implements AES-256-GCM authenticated payload encryption, Argon2id key
//! derivation, memory zeroization via `zeroize`, and Ed25519 digital signatures.

pub mod memlock;
pub use memlock::{LockedBytes, LockedKey};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Cryptographic error definitions
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("AES-256-GCM encryption failed")]
    EncryptionFailed,
    #[error("AES-256-GCM decryption failed: invalid ciphertext, wrong key, or tampered tag")]
    DecryptionFailed,
    #[error("Argon2id key derivation failed: {0}")]
    KeyDerivationFailed(String),
    #[error("Payload is too short to contain a valid 12-byte nonce and 16-byte tag")]
    PayloadTooShort,
    #[error("Invalid Ed25519 signature format")]
    InvalidSignature,
    #[error("Ed25519 signature verification failed")]
    SignatureVerificationFailed,
    #[error("Biometric authentication failed: {0}")]
    BiometricFailed(String),
}

/// Zeroizing wrapper for sensitive byte buffers
#[derive(Debug, Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct ZeroizingBytes(Vec<u8>);

impl ZeroizingBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}

/// Zeroizing wrapper for sensitive UTF-8 strings
#[derive(Debug, Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct ZeroizingString(String);

impl ZeroizingString {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Configurable Argon2id parameters
#[derive(Debug, Clone)]
pub struct Argon2Params {
    pub memory_cost_kib: u32,
    pub time_cost_iterations: u32,
    pub parallelism: u32,
}

impl Default for Argon2Params {
    /// OWASP standard recommended profile
    fn default() -> Self {
        Self {
            memory_cost_kib: 64 * 1024, // 64 MiB
            time_cost_iterations: 3,
            parallelism: 4,
        }
    }
}

impl Argon2Params {
    /// Fast profile used for automated unit tests
    pub fn fast_for_tests() -> Self {
        Self {
            memory_cost_kib: 1024, // 1 MiB
            time_cost_iterations: 1,
            parallelism: 1,
        }
    }
}

/// Symmetric payload encryption using AES-256-GCM.
///
/// Output format: `[12-byte random nonce || ciphertext || 16-byte authentication tag]`
pub fn encrypt_aes_gcm(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| EncryptionError::EncryptionFailed)?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| EncryptionError::EncryptionFailed)?;

    let mut output = Vec::with_capacity(12 + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

/// Symmetric payload decryption using AES-256-GCM.
///
/// Returns decrypted plaintext inside a secure `ZeroizingBytes` container.
pub fn decrypt_aes_gcm(key: &[u8; 32], payload: &[u8]) -> Result<ZeroizingBytes, EncryptionError> {
    if payload.len() < 12 + 16 {
        return Err(EncryptionError::PayloadTooShort);
    }

    let (nonce_slice, ciphertext_slice) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_slice);

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| EncryptionError::DecryptionFailed)?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext_slice)
        .map_err(|_| EncryptionError::DecryptionFailed)?;

    Ok(ZeroizingBytes::new(plaintext))
}

/// Derive a 256-bit (32-byte) key from a passphrase and salt using Argon2id.
pub fn derive_key_argon2id(
    passphrase: &[u8],
    salt: &[u8; 16],
    params: &Argon2Params,
) -> Result<[u8; 32], EncryptionError> {
    let argon_params = Params::new(
        params.memory_cost_kib,
        params.time_cost_iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|e| EncryptionError::KeyDerivationFailed(e.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(|e| EncryptionError::KeyDerivationFailed(e.to_string()))?;

    Ok(key)
}

/// Generate a fresh random 16-byte cryptographic salt.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Generate an Ed25519 signing and verifying key pair.
pub fn generate_ed25519_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Sign arbitrary payload bytes with an Ed25519 signing key.
pub fn sign_ed25519(signing_key: &SigningKey, data: &[u8]) -> [u8; 64] {
    let signature = signing_key.sign(data);
    signature.to_bytes()
}

/// Verify an Ed25519 signature against payload bytes.
pub fn verify_ed25519(
    verifying_key_bytes: &[u8; 32],
    data: &[u8],
    signature_bytes: &[u8; 64],
) -> Result<bool, EncryptionError> {
    let verifying_key = VerifyingKey::from_bytes(verifying_key_bytes)
        .map_err(|_| EncryptionError::InvalidSignature)?;

    let signature = Signature::from_bytes(signature_bytes);

    verifying_key
        .verify(data, &signature)
        .map(|_| true)
        .map_err(|_| EncryptionError::SignatureVerificationFailed)
}

/// Biometric provider abstraction
pub trait BiometricProvider: Send + Sync {
    /// Check whether native biometric hardware is available and enrolled.
    fn is_available(&self) -> bool;
    /// Prompt user for biometric authentication with a reason.
    fn authenticate(&self, prompt_message: &str) -> Result<bool, EncryptionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only Mock biometric authenticator
    struct MockBiometricProvider {
        available: bool,
        allow_authentication: bool,
    }

    impl Default for MockBiometricProvider {
        fn default() -> Self {
            Self {
                available: true,
                allow_authentication: true,
            }
        }
    }

    impl BiometricProvider for MockBiometricProvider {
        fn is_available(&self) -> bool {
            self.available
        }

        fn authenticate(&self, _prompt_message: &str) -> Result<bool, EncryptionError> {
            if !self.available {
                return Err(EncryptionError::BiometricFailed(
                    "Biometric hardware unavailable".to_string(),
                ));
            }
            if self.allow_authentication {
                Ok(true)
            } else {
                Err(EncryptionError::BiometricFailed(
                    "User cancelled or verification failed".to_string(),
                ))
            }
        }
    }

    #[test]
    fn test_aes_gcm_roundtrip() {
        let key = [0x42u8; 32];
        let message = b"Confidential AWS Secret Key 2026";

        let encrypted = encrypt_aes_gcm(&key, message).expect("Encryption must succeed");
        assert_ne!(encrypted.as_slice(), message);

        let decrypted = decrypt_aes_gcm(&key, &encrypted).expect("Decryption must succeed");
        assert_eq!(decrypted.as_slice(), message);
    }

    #[test]
    fn test_aes_gcm_tampered_fails() {
        let key = [0x42u8; 32];
        let message = b"Super sensitive secret payload";

        let mut encrypted = encrypt_aes_gcm(&key, message).unwrap();
        // Tamper with the ciphertext
        let len = encrypted.len();
        encrypted[len - 1] ^= 0xFF;

        let err = decrypt_aes_gcm(&key, &encrypted).unwrap_err();
        match err {
            EncryptionError::DecryptionFailed => (),
            other => panic!("Expected DecryptionFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_aes_gcm_payload_too_short() {
        let key = [0x42u8; 32];
        let short_payload = [0u8; 15]; // less than 12 + 16 = 28 bytes

        let err = decrypt_aes_gcm(&key, &short_payload).unwrap_err();
        match err {
            EncryptionError::PayloadTooShort => (),
            other => panic!("Expected PayloadTooShort, got {:?}", other),
        }
    }

    #[test]
    fn test_argon2id_derivation() {
        let passphrase = b"CorrectHorseBatteryStaple";
        let salt = generate_salt();
        let params = Argon2Params::fast_for_tests();

        let key1 = derive_key_argon2id(passphrase, &salt, &params).unwrap();
        let key2 = derive_key_argon2id(passphrase, &salt, &params).unwrap();
        assert_eq!(key1, key2);

        let different_salt = generate_salt();
        let key3 = derive_key_argon2id(passphrase, &different_salt, &params).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_ed25519_sign_and_verify() {
        let (signing_key, verifying_key) = generate_ed25519_keypair();
        let payload = b"Universal Log Viewer release v1.0.0 package binary payload";

        let signature = sign_ed25519(&signing_key, payload);
        let verified = verify_ed25519(&verifying_key.to_bytes(), payload, &signature).unwrap();
        assert!(verified);

        // Verification fails if payload altered
        let tampered_payload = b"Tampered binary payload";
        assert!(verify_ed25519(&verifying_key.to_bytes(), tampered_payload, &signature).is_err());
    }

    #[test]
    fn test_mock_biometrics() {
        let provider = MockBiometricProvider::default();
        assert!(provider.is_available());
        assert!(provider.authenticate("Unlock DevOps Vault").unwrap());

        let rejecting_provider = MockBiometricProvider {
            available: true,
            allow_authentication: false,
        };
        assert!(rejecting_provider.authenticate("Unlock").is_err());
    }
}
