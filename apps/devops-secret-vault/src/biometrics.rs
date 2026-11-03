#![forbid(unsafe_code)]
//! DevOps Secret Vault - Hardware & Biometric Authentication Abstraction
//!
//! Provides OS keyring integration and biometric authentication verification
//! across Windows (Windows Hello / Credential Manager), macOS (Touch ID / Keychain),
//! and Linux (Polkit / Secret Service).

use platform_common::{PlatformKeyring, PlatformKeyringFactory};
use std::sync::Arc;

use crate::crypto::VaultMasterKey;
use crate::error::VaultError;

const BIOMETRIC_SERVICE_NAME: &str = "devops-secret-vault";
const BIOMETRIC_ACCOUNT_NAME: &str = "biometric_master_key";

/// Service managing biometric hardware authentication (Windows Hello, Touch ID, etc.)
/// and hardware-backed keyring integration for master key retention
#[derive(Clone)]
pub struct BiometricService {
    keyring: Arc<dyn PlatformKeyring>,
}

impl Default for BiometricService {
    fn default() -> Self {
        Self {
            keyring: Arc::from(PlatformKeyringFactory::create_keyring()),
        }
    }
}

impl BiometricService {
    /// Create with a specific platform keyring implementation (e.g. for testing)
    pub fn new(keyring: Arc<dyn PlatformKeyring>) -> Self {
        Self { keyring }
    }

    /// Check if biometric hardware is present and user is enrolled
    pub fn is_available(&self) -> bool {
        self.keyring.is_biometric_supported()
    }

    /// Prompt user for biometric authentication
    pub fn authenticate(&self, prompt_message: &str) -> Result<bool, VaultError> {
        self.keyring
            .prompt_biometric_auth(prompt_message)
            .map_err(|e| VaultError::Biometric(e.to_string()))
    }

    /// Store the master key securely in the platform keyring (enabling biometric unlock)
    pub fn store_biometric_key(&self, master_key: &VaultMasterKey) -> Result<(), VaultError> {
        let hex_key = bytes_to_hex(&master_key.0);
        self.keyring
            .set_secret(BIOMETRIC_SERVICE_NAME, BIOMETRIC_ACCOUNT_NAME, &hex_key)
            .map_err(|e| {
                VaultError::Biometric(format!("Failed to store biometric credential: {e}"))
            })
    }

    /// Retrieve master key stored in the platform keyring
    pub fn retrieve_biometric_key(&self) -> Result<Option<VaultMasterKey>, VaultError> {
        match self
            .keyring
            .get_secret(BIOMETRIC_SERVICE_NAME, BIOMETRIC_ACCOUNT_NAME)
        {
            Ok(Some(hex_str)) => {
                let bytes = hex_to_bytes(hex_str.trim()).ok_or_else(|| {
                    VaultError::Biometric("Invalid stored hex key format".to_string())
                })?;
                if bytes.len() != 32 {
                    return Err(VaultError::Biometric(
                        "Stored biometric key length mismatch: expected 32 bytes".to_string(),
                    ));
                }
                let mut key_arr = [0u8; 32];
                key_arr.copy_from_slice(&bytes);
                Ok(Some(VaultMasterKey(key_arr)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(VaultError::Biometric(format!(
                "Failed to retrieve biometric key from keyring: {e}"
            ))),
        }
    }

    /// Clear the stored biometric master key from the platform keyring
    pub fn clear_biometric_key(&self) -> Result<(), VaultError> {
        self.keyring
            .delete_secret(BIOMETRIC_SERVICE_NAME, BIOMETRIC_ACCOUNT_NAME)
            .map_err(|e| {
                VaultError::Biometric(format!("Failed to delete biometric credential: {e}"))
            })
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_common::InMemoryKeyring;

    #[test]
    fn test_biometric_service_success() {
        let test_keyring = Arc::new(InMemoryKeyring::for_test(true, true));
        let service = BiometricService::new(test_keyring);
        assert!(service.is_available());
        assert!(service.authenticate("Unlock Vault").unwrap());

        // Test key storage and retrieval
        let dummy_key = VaultMasterKey([42u8; 32]);
        service.store_biometric_key(&dummy_key).unwrap();

        let retrieved = service.retrieve_biometric_key().unwrap().unwrap();
        assert_eq!(retrieved.0, [42u8; 32]);

        service.clear_biometric_key().unwrap();
        assert!(service.retrieve_biometric_key().unwrap().is_none());
    }
}
