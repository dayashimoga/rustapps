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
    PersonalKnowledgeGraph,
    ApiProtocolStudio,
    ParquetAnalyticsWorkbench,
    DfirTriageKit,
    ScreenRecorderStudio,
    SignalTelemetryOscilloscope,
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

/// Official 32-byte Ed25519 root verifying key embedded in production binaries
/// Allows verification 100% offline without remote servers or egress.
pub const OFFICIAL_VERIFYING_KEY_BYTES: [u8; 32] = [
    133, 157, 43, 71, 72, 245, 83, 122, 36, 95, 122, 228, 109, 101, 15, 108, 3, 133, 102, 9, 42,
    219, 151, 76, 247, 226, 93, 226, 123, 144, 45, 46,
];

/// Official 32-byte Ed25519 signing seed used by release tooling and testing
pub const OFFICIAL_SIGNING_SEED: [u8; 32] = [
    0x72, 0x6e, 0x74, 0x67, 0x72, 0x61, 0x76, 0x69, 0x74, 0x79, 0x5f, 0x6f, 0x66, 0x66, 0x6c, 0x69,
    0x6e, 0x65, 0x5f, 0x6c, 0x69, 0x63, 0x65, 0x6e, 0x73, 0x65, 0x5f, 0x73, 0x65, 0x65, 0x64, 0x31,
];

/// In-process Ed25519 license verifier
#[derive(Debug, Clone)]
pub struct LicenseVerifier {
    verifying_key: VerifyingKey,
}

impl LicenseVerifier {
    /// Create an official verifier with the embedded root public key
    pub fn official() -> Self {
        Self::new(&OFFICIAL_VERIFYING_KEY_BYTES).expect("Official verifying key must be valid")
    }

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
    /// Initialize with the official monorepo private seed
    pub fn official() -> Self {
        Self::from_seed(&OFFICIAL_SIGNING_SEED)
    }

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

    /// Generate a status report for UI badges and diagnostics
    pub fn status_report(&self) -> LicenseStatusReport {
        if let Some(ref lic) = self.active_license {
            LicenseStatusReport {
                tier: match lic.tier {
                    LicenseTier::Free => "Free".to_string(),
                    LicenseTier::Premium => "Premium".to_string(),
                    LicenseTier::Business => "Business".to_string(),
                },
                licensee_name: lic.licensee_name.clone(),
                is_perpetual: lic.is_perpetual,
                is_active: true,
                features: lic.features.clone(),
            }
        } else {
            LicenseStatusReport {
                tier: "Free".to_string(),
                licensee_name: "Community Edition".to_string(),
                is_perpetual: true,
                is_active: false,
                features: vec![],
            }
        }
    }
}

/// Complete status report for IPC commands and frontend UI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseStatusReport {
    pub tier: String,
    pub licensee_name: String,
    pub is_perpetual: bool,
    pub is_active: bool,
    pub features: Vec<String>,
}

/// Verify offline license JSON certificate for a specific app
pub fn verify_license_json(
    json_str: &str,
    app_id: AppId,
) -> Result<VerifiedLicense, LicensingError> {
    let cert: LicenseCertificate = serde_json::from_str(json_str)
        .map_err(|e| LicensingError::ParseError(format!("Failed to parse JSON: {}", e)))?;
    let verifier = LicenseVerifier::official();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    verifier.verify(&cert, app_id, now)
}

/// Thread-safe license manager for applications
#[derive(Debug, Clone)]
pub struct LicensingManager {
    app_id: AppId,
    license_file_path: std::path::PathBuf,
    gate: std::sync::Arc<std::sync::RwLock<TierGate>>,
}

impl LicensingManager {
    /// Initialize manager for target app and license storage path
    pub fn new(app_id: AppId, storage_dir: &std::path::Path) -> Self {
        let license_file_path = storage_dir.join("license.offlic");
        let initial_license = if license_file_path.exists() {
            match std::fs::read_to_string(&license_file_path) {
                Ok(content) => verify_license_json(&content, app_id).ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        Self {
            app_id,
            license_file_path,
            gate: std::sync::Arc::new(std::sync::RwLock::new(TierGate::new(initial_license))),
        }
    }

    /// Current operational tier gate
    pub fn gate(&self) -> TierGate {
        self.gate.read().unwrap().clone()
    }

    /// Current license status report
    pub fn status_report(&self) -> LicenseStatusReport {
        self.gate.read().unwrap().status_report()
    }

    /// Activate a signed offline license JSON certificate
    pub fn activate(&self, license_json: &str) -> Result<LicenseStatusReport, LicensingError> {
        let verified = verify_license_json(license_json, self.app_id)?;

        // Save to offline persistent storage
        if let Some(parent) = self.license_file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.license_file_path, license_json);

        // Update in-memory gate
        let mut g = self.gate.write().unwrap();
        *g = TierGate::new(Some(verified));
        Ok(g.status_report())
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

    #[test]
    fn test_official_keypair() {
        let auth = LicenseAuthority::from_seed(&OFFICIAL_SIGNING_SEED);
        let pk = auth.public_key_bytes();
        println!("OFFICIAL_VERIFYING_KEY_BYTES = {:?}", pk);
        let verifier = LicenseVerifier::new(&pk).unwrap();
        let cert = auth.sign_license(LicenseParameters {
            license_id: "OFFICIAL-1".to_string(),
            licensee_name: "Official User".to_string(),
            licensee_email: "user@example.com".to_string(),
            tier: LicenseTier::Business,
            app_id: AppId::AllAppsSuite,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None,
            max_seats: 10,
            features: vec!["all".to_string()],
        });
        assert!(verifier
            .verify(&cert, AppId::AiFileExplorer, 1750000000)
            .is_ok());
    }

    #[test]
    fn test_tamper_resistance_signature_rejection() {
        let auth = LicenseAuthority::official();
        let mut cert = auth.sign_license(LicenseParameters {
            license_id: "TAMPER-TEST-01".to_string(),
            licensee_name: "Genuine Buyer".to_string(),
            licensee_email: "buyer@example.com".to_string(),
            tier: LicenseTier::Premium,
            app_id: AppId::DevopsSecretVault,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None,
            max_seats: 1,
            features: vec!["cloud_secrets".to_string()],
        });

        // 1. Verify genuine license succeeds
        let verifier = LicenseVerifier::official();
        assert!(verifier
            .verify(&cert, AppId::DevopsSecretVault, 1750000000)
            .is_ok());

        // 2. Tampering with tier from Premium to Business must fail
        cert.tier = LicenseTier::Business;
        let err = verifier
            .verify(&cert, AppId::DevopsSecretVault, 1750000000)
            .unwrap_err();
        assert!(matches!(err, LicensingError::InvalidSignature(_)));

        // 3. Tampering with licensee name must fail
        cert.tier = LicenseTier::Premium;
        cert.licensee_name = "Attacker".to_string();
        let err = verifier
            .verify(&cert, AppId::DevopsSecretVault, 1750000000)
            .unwrap_err();
        assert!(matches!(err, LicensingError::InvalidSignature(_)));

        // 4. Corrupting signature hex must fail
        cert.licensee_name = "Genuine Buyer".to_string();
        cert.signature_hex = "00".repeat(64);
        let err = verifier
            .verify(&cert, AppId::DevopsSecretVault, 1750000000)
            .unwrap_err();
        assert!(matches!(err, LicensingError::InvalidSignature(_)));
    }

    #[test]
    fn test_verify_license_json_helper() {
        let auth = LicenseAuthority::official();
        let cert = auth.sign_license(LicenseParameters {
            license_id: "JSON-01".to_string(),
            licensee_name: "Alice Developer".to_string(),
            licensee_email: "alice@code.com".to_string(),
            tier: LicenseTier::Business,
            app_id: AppId::AllAppsSuite,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None,
            max_seats: 5,
            features: vec!["offline_ai".to_string()],
        });

        let json = serde_json::to_string(&cert).unwrap();
        let verified =
            verify_license_json(&json, AppId::ProAvEditor).expect("JSON verification must succeed");
        assert_eq!(verified.tier, LicenseTier::Business);
        assert_eq!(verified.licensee_name, "Alice Developer");

        let gate = TierGate::new(Some(verified));
        let report = gate.status_report();
        assert_eq!(report.tier, "Business");
        assert!(report.is_active);
    }

    #[test]
    fn test_licensing_manager_workflow() {
        let temp = tempfile::tempdir().unwrap();
        let mgr = LicensingManager::new(AppId::AiFileExplorer, temp.path());

        // Default should be Free tier
        let initial_status = mgr.status_report();
        assert_eq!(initial_status.tier, "Free");
        assert!(!initial_status.is_active);

        // Mint and activate a valid license
        let auth = LicenseAuthority::official();
        let cert = auth.sign_license(LicenseParameters {
            license_id: "ACT-01".to_string(),
            licensee_name: "Pro Engineer".to_string(),
            licensee_email: "pro@offline.org".to_string(),
            tier: LicenseTier::Premium,
            app_id: AppId::AiFileExplorer,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None,
            max_seats: 1,
            features: vec!["deep_ocr".to_string()],
        });
        let cert_json = serde_json::to_string(&cert).unwrap();

        let updated_status = mgr.activate(&cert_json).expect("Activation should succeed");
        assert_eq!(updated_status.tier, "Premium");
        assert!(updated_status.is_active);
        assert_eq!(updated_status.licensee_name, "Pro Engineer");

        // Reload manager from same directory - license should persist
        let reloaded_mgr = LicensingManager::new(AppId::AiFileExplorer, temp.path());
        let persisted_status = reloaded_mgr.status_report();
        assert_eq!(persisted_status.tier, "Premium");
        assert!(persisted_status.is_active);
    }
}
