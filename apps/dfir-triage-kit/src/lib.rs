#![deny(unsafe_code)]
//! Offline Digital Forensics & Incident Response (DFIR) Triage Kit Library

pub mod collector;
pub mod commands;
pub mod ffi;
pub mod models;
pub mod service;

pub use collector::ForensicCollector;
pub use commands::dispatch_ipc_command;
pub use models::{
    ArtifactType, CertArtifact, DfirStats, FileIntegrityRecord, LogEventRecord, LogLevel,
    ProcessArtifact, Severity, SocketArtifact, SocketProtocol, SocketState, TriageSummary,
};
pub use service::{DfirError, DfirService, TriagePackage};
