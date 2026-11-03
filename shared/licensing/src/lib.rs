#![forbid(unsafe_code)]
//! Offline Cryptographic Licensing Subsystem
//!
//! Provides zero-network-egress, Ed25519-signed offline license verification,
//! granular per-app and suite licensing, feature gating, and license tier evaluation.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Licensing error definitions
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LicensingError {
    #[error("Invalid cryptographic signature on license certificate: {0}")]
    InvalidSignature(String),
    #[error("License expired at timestamp {expiry_secs} (current: {current_secs})")]
    Expired { expiry_secs: u64, current_secs: u64 },
    #[error("License application mismatch: expected {expected:?}, got {actual:?}")]
    AppMismatch { expected: AppId, actual: AppId },
    #[error("Insufficient license tier: requires {required:?}, currently {current:?}")]
    InsufficientTier {
        required: LicenseTier,
        current: LicenseTier,
    },
    #[error("Feature not licensed for current tier: {0}")]
    FeatureNotLicensed(String),
    #[error("Failed to parse license key or certificate: {0}")]
    ParseError(String),
}

/// Target application identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppId {
    AiFileExplorer,
    DevopsSecretVault,
    UniversalLogViewer,
    UniversalLiveTranslator,
    ProAvEditor,
    AllAppsSuite,
}

impl AppId {
    /// Check if this license covers a specific application
    pub fn covers(&self, target: AppId) -> bool {
        *self == AppId::AllAppsSuite || *self == target
    }
}

/// Commercial license tiers in ascending capability order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LicenseTier {
    /// Community Free tier (\$0 Forever)
    Free = 0,
    /// Professional Individual tier (perpetual offline license)
    Premium = 1,
    /// Enterprise & Air-Gapped Business tier
    Business = 2,
}

/// Raw offline license certificate loaded from `.offlic` JSON or file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseCertificate {
    pub license_id: String,
    pub licensee_name: String,
    pub licensee_email: String,
    pub tier: LicenseTier,
    pub app_id: AppId,
    pub issued_timestamp_secs: u64,
    pub expiry_timestamp_secs: Option<u64>,
    pub max_seats: u32,
    pub features: Vec<String>,
    pub signature_hex: String,
}

impl LicenseCertificate {
    /// Generates canonical bytes representation used for signing and verification
    pub fn canonical_payload(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"OFFLINE_LICENSE_CERT_V1:");
        hasher.update(self.license_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.licensee_name.as_bytes());
        hasher.update(b"|");
        hasher.update(self.licensee_email.as_bytes());
        hasher.update(b"|");
        hasher.update((self.tier as u8).to_string().as_bytes());
        hasher.update(b"|");
        hasher.update(format!("{:?}", self.app_id).as_bytes());
        hasher.update(b"|");
        hasher.update(self.issued_timestamp_secs.to_string().as_bytes());
        hasher.update(b"|");
        hasher.update(
            self.expiry_timestamp_secs
                .map(|e| e.to_string())
                .unwrap_or_else(|| "NONE".to_string())
                .as_bytes(),
        );
        hasher.update(b"|");
        hasher.update(self.max_seats.to_string().as_bytes());
        hasher.update(b"|");
        let mut sorted_features = self.features.clone();
        sorted_features.sort();
        hasher.update(sorted_features.join(",").as_bytes());

        hasher.finalize().to_vec()
    }
}

/// Verified license details returned after cryptographic check
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedLicense {
    pub license_id: String,
    pub licensee_name: String,
    pub tier: LicenseTier,
    pub app_id: AppId,
    pub is_perpetual: bool,
    pub features: Vec<String>,
}

/// In-process Ed25519 license verifier
#[derive(Debug, Clone)]
pub struct LicenseVerifier {
    verifying_key: VerifyingKey,
}

impl LicenseVerifier {
    /// Initialize with a 32-byte Ed25519 verifying public key
    pub fn new(public_key_bytes: &[u8; 32]) -> Result<Self, LicensingError> {
        let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(|e| LicensingError::ParseError(format!("Invalid public key: {}", e)))?;
        Ok(Self { verifying_key })
    }

    /// Verifies signature, expiry, and app matching against current runtime timestamp
    pub fn verify(
        &self,
        cert: &LicenseCertificate,
        target_app: AppId,
        current_timestamp_secs: u64,
    ) -> Result<VerifiedLicense, LicensingError> {
        // 1. Verify app compatibility
        if !cert.app_id.covers(target_app) {
            return Err(LicensingError::AppMismatch {
                expected: target_app,
                actual: cert.app_id,
            });
        }

        // 2. Check expiration (if not perpetual)
        if let Some(expiry) = cert.expiry_timestamp_secs {
            if current_timestamp_secs > expiry {
                return Err(LicensingError::Expired {
                    expiry_secs: expiry,
                    current_secs: current_timestamp_secs,
                });
            }
        }

        // 3. Verify cryptographic Ed25519 signature
        let payload = cert.canonical_payload();
        let sig_bytes = hex_to_bytes(&cert.signature_hex)
            .map_err(|e| LicensingError::InvalidSignature(e.to_string()))?;
        if sig_bytes.len() != 64 {
            return Err(LicensingError::InvalidSignature(
                "Signature must be exactly 64 bytes".to_string(),
            ));
        }

        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        self.verifying_key
            .verify(&payload, &signature)
            .map_err(|e| LicensingError::InvalidSignature(e.to_string()))?;

        Ok(VerifiedLicense {
            license_id: cert.license_id.clone(),
            licensee_name: cert.licensee_name.clone(),
            tier: cert.tier,
            app_id: cert.app_id,
            is_perpetual: cert.expiry_timestamp_secs.is_none(),
            features: cert.features.clone(),
        })
    }
}

/// Parameters for minting a new license certificate
#[derive(Debug, Clone)]
pub struct LicenseParameters {
    pub license_id: String,
    pub licensee_name: String,
    pub licensee_email: String,
    pub tier: LicenseTier,
    pub app_id: AppId,
    pub issued_timestamp_secs: u64,
    pub expiry_timestamp_secs: Option<u64>,
    pub max_seats: u32,
    pub features: Vec<String>,
}

/// Offline license minting authority for signing test and production licenses
pub struct LicenseAuthority {
    signing_key: SigningKey,
}

impl LicenseAuthority {
    /// Initialize with a 32-byte Ed25519 private seed
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        Self { signing_key }
    }

    /// Generates a new random authority keypair
    pub fn generate() -> Self {
        let mut rng = rand::rngs::OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        Self { signing_key }
    }

    /// Export the 32-byte public verifying key
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Sign and construct a completed LicenseCertificate
    pub fn sign_license(&self, params: LicenseParameters) -> LicenseCertificate {
        let mut cert = LicenseCertificate {
            license_id: params.license_id,
            licensee_name: params.licensee_name,
            licensee_email: params.licensee_email,
            tier: params.tier,
            app_id: params.app_id,
            issued_timestamp_secs: params.issued_timestamp_secs,
            expiry_timestamp_secs: params.expiry_timestamp_secs,
            max_seats: params.max_seats,
            features: params.features,
            signature_hex: String::new(),
        };

        let payload = cert.canonical_payload();
        let sig = self.signing_key.sign(&payload);
        cert.signature_hex = bytes_to_hex(&sig.to_bytes());
        cert
    }
}

/// Feature gate guard checking runtime requirements
#[derive(Debug, Clone)]
pub struct TierGate {
    active_license: Option<VerifiedLicense>,
}

impl TierGate {
    /// Create with an optional verified license (or default to Free tier if None)
    pub fn new(active_license: Option<VerifiedLicense>) -> Self {
        Self { active_license }
    }

    /// Create a TierGate configured for a specific tier
    pub fn for_tier(tier: LicenseTier) -> Self {
        Self {
            active_license: Some(VerifiedLicense {
                license_id: "system-tier".to_string(),
                licensee_name: "local".to_string(),
                tier,
                app_id: AppId::DevopsSecretVault,
                is_perpetual: true,
                features: vec![],
            }),
        }
    }

    /// Convenience constructor for enterprise / business tier
    pub fn enterprise() -> Self {
        Self::for_tier(LicenseTier::Business)
    }

    /// Current operational tier
    pub fn current_tier(&self) -> LicenseTier {
        self.active_license
            .as_ref()
            .map(|l| l.tier)
            .unwrap_or(LicenseTier::Free)
    }

    /// Require minimum tier, returning an error if current tier is insufficient
    pub fn require_tier(&self, required: LicenseTier) -> Result<(), LicensingError> {
        let current = self.current_tier();
        if current >= required {
            Ok(())
        } else {
            Err(LicensingError::InsufficientTier { required, current })
        }
    }

    /// Check if a specific named feature is enabled by tier or custom feature flag
    pub fn has_feature(&self, feature_name: &str) -> bool {
        if let Some(ref lic) = self.active_license {
            if lic.tier == LicenseTier::Business {
                return true;
            }
            if lic
                .features
                .iter()
                .any(|f| f.eq_ignore_ascii_case(feature_name))
            {
                return true;
            }
        }
        false
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, LicensingError> {
    if hex.len() % 2 != 0 {
        return Err(LicensingError::ParseError("Odd hex length".to_string()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| LicensingError::ParseError(format!("Invalid hex byte: {}", e)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_minting_and_verification() {
        let authority = LicenseAuthority::generate();
        let verifier = LicenseVerifier::new(&authority.public_key_bytes()).unwrap();

        let cert = authority.sign_license(LicenseParameters {
            license_id: "LIC-2026-001".to_string(),
            licensee_name: "Jane Doe".to_string(),
            licensee_email: "jane@example.com".to_string(),
            tier: LicenseTier::Premium,
            app_id: AppId::UniversalLogViewer,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: Some(1800000000),
            max_seats: 1,
            features: vec!["drain_mining".to_string(), "anomaly_detect".to_string()],
        });

        let verified = verifier
            .verify(&cert, AppId::UniversalLogViewer, 1750000000)
            .expect("Verification must succeed");

        assert_eq!(verified.license_id, "LIC-2026-001");
        assert_eq!(verified.tier, LicenseTier::Premium);
        assert!(!verified.is_perpetual);
    }

    #[test]
    fn test_suite_license_covers_all_apps() {
        let authority = LicenseAuthority::generate();
        let verifier = LicenseVerifier::new(&authority.public_key_bytes()).unwrap();

        let suite_cert = authority.sign_license(LicenseParameters {
            license_id: "SUITE-999".to_string(),
            licensee_name: "Acme Corp".to_string(),
            licensee_email: "licensing@acme.corp".to_string(),
            tier: LicenseTier::Business,
            app_id: AppId::AllAppsSuite,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None, // Perpetual
            max_seats: 100,
            features: vec!["unlimited".to_string()],
        });

        // Suite license covers all applications
        assert!(verifier
            .verify(&suite_cert, AppId::AiFileExplorer, 1750000000)
            .is_ok());
        assert!(verifier
            .verify(&suite_cert, AppId::DevopsSecretVault, 1750000000)
            .is_ok());
        assert!(verifier
            .verify(&suite_cert, AppId::UniversalLiveTranslator, 1750000000)
            .is_ok());
        assert!(verifier
            .verify(&suite_cert, AppId::ProAvEditor, 1750000000)
            .is_ok());
    }

    #[test]
    fn test_expired_license_rejection() {
        let authority = LicenseAuthority::generate();
        let verifier = LicenseVerifier::new(&authority.public_key_bytes()).unwrap();

        let expired_cert = authority.sign_license(LicenseParameters {
            license_id: "EXPIRED-01".to_string(),
            licensee_name: "Bob".to_string(),
            licensee_email: "bob@test.com".to_string(),
            tier: LicenseTier::Premium,
            app_id: AppId::UniversalLogViewer,
            issued_timestamp_secs: 1600000000,
            expiry_timestamp_secs: Some(1650000000),
            max_seats: 1,
            features: vec![],
        });

        let err = verifier
            .verify(&expired_cert, AppId::UniversalLogViewer, 1700000000)
            .unwrap_err();
        assert!(matches!(err, LicensingError::Expired { .. }));
    }

    #[test]
    fn test_tier_gate_enforcement() {
        let free_gate = TierGate::new(None);
        assert_eq!(free_gate.current_tier(), LicenseTier::Free);
        assert!(free_gate.require_tier(LicenseTier::Free).is_ok());
        assert_eq!(
            free_gate.require_tier(LicenseTier::Premium).unwrap_err(),
            LicensingError::InsufficientTier {
                required: LicenseTier::Premium,
                current: LicenseTier::Free,
            }
        );

        let business_license = VerifiedLicense {
            license_id: "BIZ-1".to_string(),
            licensee_name: "MegaCorp".to_string(),
            tier: LicenseTier::Business,
            app_id: AppId::AllAppsSuite,
            is_perpetual: true,
            features: vec!["custom_branding".to_string()],
        };

        let biz_gate = TierGate::new(Some(business_license));
        assert!(biz_gate.require_tier(LicenseTier::Business).is_ok());
        assert!(biz_gate.has_feature("any_feature"));
    }
}
