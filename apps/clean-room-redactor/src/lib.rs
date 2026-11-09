//! Offline Clean-Room Document Redactor Core Library
//!
//! True clean-room binary text stream burnout, PII/PHI automated detection,
//! Bates numbering stamping, and Ed25519 cryptographic sanitization certificates.

pub mod certificate;
pub mod commands;
pub mod ffi;
pub mod models;
pub mod redact_engine;
pub mod service;

pub use certificate::CertificateManager;
pub use commands::dispatch_ipc_command;
pub use models::*;
pub use redact_engine::RedactionEngine;
pub use service::RedactionService;
