#![forbid(unsafe_code)]
//! DevOps Secret Vault - Domain Error Types

use thiserror::Error;

/// Domain error types for Vault operations
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Vault is not initialized. Please set a master password.")]
    NotInitialized,

    #[error("Vault is already initialized.")]
    AlreadyInitialized,

    #[error("Vault is locked. Unlock with master password or biometrics first.")]
    Locked,

    #[error("Invalid master password or corrupted verification token.")]
    InvalidMasterPassword,

    #[error("Secret with ID '{0}' not found.")]
    SecretNotFound(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Cryptographic operation failed: {0}")]
    Crypto(#[from] shared_encryption::EncryptionError),

    #[error("Storage failure: {0}")]
    Storage(#[from] shared_storage::StorageError),

    #[error("Search index failure: {0}")]
    Search(#[from] shared_search::SearchError),

    #[error("Tantivy search error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Biometric authentication error: {0}")]
    Biometric(String),

    #[error("Secure clipboard error: {0}")]
    Clipboard(String),

    #[error(
        "Too many failed unlock attempts. Rate limited. Retry after {retry_after_secs} seconds."
    )]
    RateLimited { retry_after_secs: u64 },

    #[error("Shamir secret sharing error: {0}")]
    Shamir(String),

    #[error("Licensing error: {0}")]
    Licensing(#[from] shared_licensing::LicensingError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
