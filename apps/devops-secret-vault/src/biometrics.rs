#![forbid(unsafe_code)]
//! DevOps Secret Vault - Hardware & Biometric Authentication Abstraction

use shared_encryption::{BiometricProvider, MockBiometricProvider};
use std::sync::Arc;

use crate::error::VaultError;

/// Service managing biometric hardware authentication (Windows Hello, Touch ID, etc.)
#[derive(Clone)]
pub struct BiometricService {
    provider: Arc<dyn BiometricProvider>,
}

impl Default for BiometricService {
    fn default() -> Self {
        Self {
            provider: Arc::new(MockBiometricProvider::default()),
        }
    }
}

impl BiometricService {
    /// Create with a specific biometric provider
    pub fn new(provider: Arc<dyn BiometricProvider>) -> Self {
        Self { provider }
    }

    /// Check if biometric hardware is present and user is enrolled
    pub fn is_available(&self) -> bool {
        self.provider.is_available()
    }

    /// Prompt user for biometric authentication
    pub fn authenticate(&self, prompt_message: &str) -> Result<bool, VaultError> {
        self.provider
            .authenticate(prompt_message)
            .map_err(|e| VaultError::Biometric(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biometric_service_success() {
        let service = BiometricService::default();
        assert!(service.is_available());
        assert!(service.authenticate("Unlock Vault").unwrap());
    }
}
