//! Data models for Crypto PKI & CA Studio
//!
//! Represents X.509 Certificates, Certificate Authorities, CSRs,
//! Certificate Revocation Lists (CRL), and Shamir M-of-N Key Ceremonies.

use serde::{Deserialize, Serialize};

/// Supported Cryptographic Key Algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyAlgorithm {
    Ed25519,
    Rsa2048,
    Rsa4096,
    EcdsaP256,
}

impl KeyAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyAlgorithm::Ed25519 => "Ed25519 (256-bit Edwards Curve)",
            KeyAlgorithm::Rsa2048 => "RSA 2048-bit",
            KeyAlgorithm::Rsa4096 => "RSA 4096-bit",
            KeyAlgorithm::EcdsaP256 => "ECDSA P-256 (secp256r1)",
        }
    }
}

/// Certificate Classification in the PKI Tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertType {
    RootCa,
    IntermediateCa,
    TlsServer,
    TlsClient,
    CodeSigning,
}

/// Distinguished Name (DN) Subject / Issuer
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectDn {
    pub common_name: String,
    pub organization: String,
    pub organizational_unit: String,
    pub country: String,
    pub state_or_province: String,
    pub locality: String,
}

impl SubjectDn {
    pub fn to_rfc2253(&self) -> String {
        let mut parts = Vec::new();
        if !self.common_name.is_empty() {
            parts.push(format!("CN={}", self.common_name));
        }
        if !self.organizational_unit.is_empty() {
            parts.push(format!("OU={}", self.organizational_unit));
        }
        if !self.organization.is_empty() {
            parts.push(format!("O={}", self.organization));
        }
        if !self.locality.is_empty() {
            parts.push(format!("L={}", self.locality));
        }
        if !self.state_or_province.is_empty() {
            parts.push(format!("ST={}", self.state_or_province));
        }
        if !self.country.is_empty() {
            parts.push(format!("C={}", self.country));
        }
        parts.join(", ")
    }
}

/// X.509 Key Usages
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyUsageFlags {
    pub digital_signature: bool,
    pub key_encipherment: bool,
    pub key_cert_sign: bool,
    pub crl_sign: bool,
}

/// X.509 Extended Key Usages
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedKeyUsageFlags {
    pub server_auth: bool,
    pub client_auth: bool,
    pub code_signing: bool,
    pub email_protection: bool,
}

/// X.509 Certificate Representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X509Certificate {
    pub certificate_id: String,
    pub serial_number_hex: String,
    pub cert_type: CertType,
    pub subject: SubjectDn,
    pub issuer_dn: String,
    pub issuer_ca_id: Option<String>,
    pub key_algorithm: KeyAlgorithm,
    pub validity_not_before_epoch_ms: i64,
    pub validity_not_after_epoch_ms: i64,
    pub subject_alt_names: Vec<String>,
    pub key_usage: KeyUsageFlags,
    pub ext_key_usage: ExtendedKeyUsageFlags,
    pub is_ca: bool,
    pub path_len_constraint: Option<u32>,
    pub fingerprint_sha256: String,
    pub certificate_pem: String,
    pub public_key_pem: String,
    pub is_revoked: bool,
    pub revocation_reason: Option<String>,
    pub revocation_epoch_ms: Option<i64>,
}

/// Certificate Signing Request (CSR)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSigningRequest {
    pub csr_id: String,
    pub subject: SubjectDn,
    pub key_algorithm: KeyAlgorithm,
    pub requested_sans: Vec<String>,
    pub requested_cert_type: CertType,
    pub public_key_pem: String,
    pub csr_pem: String,
    pub status: String, // "Pending", "Signed", "Rejected"
}

/// Revocation Entry in CRL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedCertificateEntry {
    pub serial_number_hex: String,
    pub revocation_epoch_ms: i64,
    pub reason: String,
}

/// Certificate Revocation List (CRL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRevocationList {
    pub crl_id: String,
    pub issuer_ca_id: String,
    pub issuer_dn: String,
    pub this_update_epoch_ms: i64,
    pub next_update_epoch_ms: i64,
    pub revoked_entries: Vec<RevokedCertificateEntry>,
    pub signature_hex: String,
    pub crl_pem: String,
}

/// Shamir Custodian Shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShamirShard {
    pub shard_index: u8,
    pub shard_hex: String,
    pub custodian_label: String,
    pub commitment_hash: String,
}

/// Shamir M-of-N Secret Sharing Ceremony
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShamirCeremony {
    pub ceremony_id: String,
    pub ca_id: String,
    pub ca_common_name: String,
    pub threshold_m: usize,
    pub total_n: usize,
    pub shards: Vec<ShamirShard>,
    pub created_at_epoch_ms: i64,
    pub status: String, // "Active", "Verified", "Reconstructed"
}

/// Overall PKI Studio Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkiStats {
    pub total_cas: usize,
    pub total_issued_certificates: usize,
    pub active_certificates: usize,
    pub revoked_certificates: usize,
    pub active_crls: usize,
    pub shamir_ceremonies_count: usize,
}
