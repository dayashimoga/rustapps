//! DevOps Secret Vault - Reusable UI-Agnostic Core Library
//!
//! Provides ultra-fast, lightweight, offline-first secret management:
//! - Argon2id master key derivation with memory-hard parameters ($m=64\,\text{MiB}, t=3, p=4$)
//! - AES-256-GCM authenticated payload encryption
//! - Zeroization of in-memory keys and decrypted buffers via `zeroize`
//! - Embedded SQLite database in WAL mode with automated schema migrations
//! - Ephemeral in-memory Tantivy search engine achieving $< 50\,\text{ms}$ query latency
//! - Secure auto-clearing clipboard manager
//! - Hardware & biometric authentication integration
//! - C-ABI FFI endpoints for Tauri desktop and Flutter mobile

pub mod backup;
pub mod biometrics;
pub mod cert_monitor;
pub mod clipboard;
pub mod cloud_creds;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod ffi;
pub mod generator;
pub mod leak_scanner;
pub mod models;
pub mod risk_scoring;
pub mod search;
pub mod service;
pub mod shamir;
pub mod ssh_agent;
pub mod storage;
pub mod totp;

pub use backup::{BackupEnvelope, BackupExportSummary, BackupImportSummary};
pub use biometrics::BiometricService;
pub use cert_monitor::{CertInspectionReport, CertStatus};
pub use clipboard::SecureClipboard;
pub use cloud_creds::{AwsCredential, AzureCredential, TerraformSecret};
pub use crypto::{VaultKeyDerivationMetadata, VaultMasterKey};
pub use error::VaultError;
pub use models::*;
pub use risk_scoring::{SecurityRecommendation, Severity, VaultHealthReport};
pub use search::VaultSearchIndex;
pub use service::VaultService;
pub use shamir::{
    reconstruct_master_key, reconstruct_secret, split_master_key, split_secret, ShamirShare,
};
pub use ssh_agent::{LoadedSshKey, SshAgentManager, SshAgentStatus};
pub use storage::VaultStorage;
pub use totp::{TotpAlgorithm, TotpCodeResponse, TotpConfig};
