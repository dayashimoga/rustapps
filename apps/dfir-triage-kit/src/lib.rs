#![deny(unsafe_code)]
//! Offline Digital Forensics & Incident Response (DFIR) Triage Kit Library

pub mod collector;
pub mod commands;
pub mod evtx_parser;
pub mod ffi;
pub mod mitre;
pub mod models;
pub mod sealer;
pub mod service;

pub use collector::ForensicCollector;
pub use commands::dispatch_ipc_command;
pub use evtx_parser::EvtxParser;
pub use mitre::MitreEngine;
pub use models::{
    ArtifactType, CertArtifact, ChainOfCustodyManifest, DfirStats, FileHashEntry,
    FileIntegrityRecord, LogEventRecord, LogLevel, MitreAttackMapping, PackageVerificationReport,
    ProcessArtifact, Severity, SocketArtifact, SocketProtocol, SocketState, TriageSummary,
};
pub use sealer::EvidenceSealer;
pub use service::{DfirError, DfirService, TriagePackage};
