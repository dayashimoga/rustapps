#![forbid(unsafe_code)]
//! Core Domain Models for Offline Digital Forensics & Incident Response (DFIR) Triage Kit

use serde::{Deserialize, Serialize};

/// Forensic artifact category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ArtifactType {
    #[default]
    ProcessList,
    NetworkSockets,
    FileIntegrity,
    EventLogs,
    Certificates,
}

/// Incident triage threat severity levels
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord,
)]
pub enum Severity {
    #[default]
    Informational,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Informational => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

/// Active process artifact inspection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessArtifact {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub exe_path: String,
    pub command_line: String,
    pub sha256: String,
    pub user: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub is_suspicious: bool,
    pub threat_indicator: Option<String>,
}

/// Socket network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SocketProtocol {
    #[default]
    Tcp,
    Udp,
}

/// Socket connection lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SocketState {
    #[default]
    Listening,
    Established,
    TimeWait,
    CloseWait,
    Unknown,
}

/// Network socket connection artifact
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketArtifact {
    pub protocol: SocketProtocol,
    pub local_addr: String,
    pub remote_addr: Option<String>,
    pub state: SocketState,
    pub pid: u32,
    pub process_name: String,
    pub is_external: bool,
}

/// File integrity, cryptographic hash, and entropy record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileIntegrityRecord {
    pub path: String,
    pub file_size_bytes: u64,
    pub blake3_hash: String,
    pub sha256_hash: String,
    pub entropy: f64,
    pub is_suspicious: bool,
    pub anomaly_notes: Option<String>,
}

/// Log event severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

/// Normalized event log record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEventRecord {
    pub timestamp: i64,
    pub source: String,
    pub event_id: u32,
    pub level: LogLevel,
    pub message: String,
    pub user_id: Option<String>,
}

/// Certificate artifact inspection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertArtifact {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub sha256_fingerprint: String,
    pub not_before: i64,
    pub not_after: i64,
    pub is_expired: bool,
    pub is_self_signed: bool,
}

/// High-level triage collection report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageSummary {
    pub triage_id: String,
    pub host_name: String,
    pub os_type: String,
    pub collected_at: i64,
    pub total_processes: usize,
    pub total_sockets: usize,
    pub total_files_scanned: usize,
    pub total_logs_analyzed: usize,
    pub threats_detected: usize,
    pub threat_score: u32,
    pub severity: Severity,
}

/// Global forensic engine metrics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DfirStats {
    pub total_triage_runs: usize,
    pub total_threats_flagged: usize,
    pub total_files_hashed: usize,
}

/// MITRE ATT&CK technique mapping for flagged forensic anomalies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MitreAttackMapping {
    pub technique_id: String,
    pub technique_name: String,
    pub tactic: String,
    pub severity: Severity,
    pub evidence: String,
}

/// Cryptographic hash entry for a file in an evidence bundle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHashEntry {
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub blake3: String,
}

/// Court-admissible cryptographic chain-of-custody manifest
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainOfCustodyManifest {
    pub manifest_version: String,
    pub package_id: String,
    pub host_name: String,
    pub os_type: String,
    pub collected_at: i64,
    pub collector_version: String,
    pub sha256_package_hash: String,
    pub blake3_package_hash: String,
    pub ed25519_signature: String,
    pub public_key: String,
    pub mitre_summary: Vec<MitreAttackMapping>,
    pub file_hashes: Vec<FileHashEntry>,
}

/// Package verification report for tamper detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageVerificationReport {
    pub package_path: String,
    pub is_valid: bool,
    pub signature_verified: bool,
    pub files_verified: usize,
    pub mismatched_files: Vec<String>,
    pub signer_public_key: String,
    pub notes: String,
}
