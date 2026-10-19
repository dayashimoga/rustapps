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

pub mod biometrics;
pub mod clipboard;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod ffi;
pub mod models;
pub mod search;
pub mod service;
pub mod storage;

pub use biometrics::BiometricService;
pub use clipboard::SecureClipboard;
pub use crypto::{VaultKeyDerivationMetadata, VaultMasterKey};
pub use error::VaultError;
pub use models::*;
pub use search::VaultSearchIndex;
pub use service::VaultService;
pub use storage::VaultStorage;
