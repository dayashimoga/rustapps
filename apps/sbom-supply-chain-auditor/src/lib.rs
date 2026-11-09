//! Air-Gapped SBOM & Supply-Chain Auditor Core Library
//!
//! 100% offline CycloneDX/SPDX ingestion, copyleft license risk classification,
//! offline CVE advisory vulnerability matching, and NTIA/EO 14028 compliance verification.

pub mod audit_engine;
pub mod commands;
pub mod ffi;
pub mod models;
pub mod service;

pub use audit_engine::AuditEngine;
pub use commands::dispatch_ipc_command;
pub use models::*;
pub use service::SbomService;
