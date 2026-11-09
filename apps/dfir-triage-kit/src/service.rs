#![forbid(unsafe_code)]
//! High-Level Digital Forensics & Incident Response Triage Service

use crate::collector::ForensicCollector;
use crate::evtx_parser::EvtxParser;
use crate::mitre::MitreEngine;
use crate::models::{
    CertArtifact, ChainOfCustodyManifest, DfirStats, FileIntegrityRecord, LogEventRecord,
    MitreAttackMapping, PackageVerificationReport, ProcessArtifact, Severity, SocketArtifact,
    TriageSummary,
};
use crate::sealer::EvidenceSealer;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DfirError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Triage run not found: {0}")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Full package of collected triage artifacts for an incident investigation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriagePackage {
    pub summary: TriageSummary,
    pub mitre_techniques: Vec<MitreAttackMapping>,
    pub processes: Vec<ProcessArtifact>,
    pub sockets: Vec<SocketArtifact>,
    pub suspicious_files: Vec<FileIntegrityRecord>,
    pub event_logs: Vec<LogEventRecord>,
    pub certificates: Vec<CertArtifact>,
}

/// Core DFIR operational service managing triage runs, persistent audit records, and heuristics
pub struct DfirService {
    conn: Arc<Mutex<Connection>>,
}

impl DfirService {
    /// Opens persistent SQLite WAL triage store
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, DfirError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let db_path = dir.join("dfir_triage.db");
        let conn = Connection::open(db_path)?;
        Self::init_db(conn)
    }

    /// Opens an in-memory triage store for unit tests and instant zero-footprint scans
    pub fn open_in_memory() -> Result<Self, DfirError> {
        let conn = Connection::open_in_memory()?;
        Self::init_db(conn)
    }

    fn init_db(conn: Connection) -> Result<Self, DfirError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS triage_runs (
                 triage_id TEXT PRIMARY KEY,
                 host_name TEXT NOT NULL,
                 os_type TEXT NOT NULL,
                 collected_at INTEGER NOT NULL,
                 total_processes INTEGER NOT NULL,
                 total_sockets INTEGER NOT NULL,
                 total_files_scanned INTEGER NOT NULL,
                 total_logs_analyzed INTEGER NOT NULL,
                 threats_detected INTEGER NOT NULL,
                 threat_score INTEGER NOT NULL,
                 severity TEXT NOT NULL,
                 package_json TEXT NOT NULL
             );",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Executes a full offline system triage sweep
    pub fn execute_triage(
        &self,
        target_scan_dir: Option<&Path>,
    ) -> Result<TriagePackage, DfirError> {
        let triage_id = format!("triage-{}", uuid::Uuid::new_v4().simple());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // 1. Collect Processes
        let processes = ForensicCollector::collect_processes();

        // 2. Collect Network Sockets
        let sockets = ForensicCollector::collect_network_sockets();

        // 3. Scan Filesystem (target directory or default temp dir)
        let default_temp = std::env::temp_dir();
        let scan_path = target_scan_dir.unwrap_or(&default_temp);
        let files = ForensicCollector::scan_directory_integrity(scan_path, 2, 50);

        // 4. Collect Event Logs
        let event_logs = ForensicCollector::inspect_event_logs();

        // 5. Collect Certificates
        let certificates = ForensicCollector::inspect_certificates();

        // 6. Correlate artifacts against MITRE ATT&CK matrix
        let mitre_techniques =
            MitreEngine::map_artifacts(&processes, &sockets, &files, &event_logs);

        // Calculate threat metrics & score (0 to 100)
        let susp_procs = processes.iter().filter(|p| p.is_suspicious).count();
        let ext_sockets = sockets.iter().filter(|s| s.is_external).count();
        let susp_files = files.iter().filter(|f| f.is_suspicious).count();
        let crit_logs = event_logs
            .iter()
            .filter(|l| {
                l.level == crate::models::LogLevel::Critical
                    || l.level == crate::models::LogLevel::Warning
            })
            .count();
        let bad_certs = certificates.iter().filter(|c| c.is_expired).count();
        let crit_mitre = mitre_techniques
            .iter()
            .filter(|m| m.severity == Severity::Critical || m.severity == Severity::High)
            .count();

        let threats_detected = susp_procs + ext_sockets + susp_files + bad_certs + crit_mitre;

        let mut threat_score = 0u32;
        threat_score += (susp_procs as u32) * 25;
        threat_score += (ext_sockets as u32) * 20;
        threat_score += (susp_files as u32) * 20;
        threat_score += (crit_logs as u32) * 10;
        threat_score += (bad_certs as u32) * 15;
        threat_score += (crit_mitre as u32) * 15;
        threat_score = threat_score.min(100);

        let severity = match threat_score {
            0..=20 => Severity::Low,
            21..=50 => Severity::Medium,
            51..=80 => Severity::High,
            _ => Severity::Critical,
        };

        let host_name = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "FORENSIC-WORKSTATION".to_string());

        let os_type = std::env::consts::OS.to_string();

        let summary = TriageSummary {
            triage_id: triage_id.clone(),
            host_name,
            os_type,
            collected_at: now,
            total_processes: processes.len(),
            total_sockets: sockets.len(),
            total_files_scanned: files.len(),
            total_logs_analyzed: event_logs.len(),
            threats_detected,
            threat_score,
            severity,
        };

        let package = TriagePackage {
            summary: summary.clone(),
            mitre_techniques,
            processes,
            sockets,
            suspicious_files: files,
            event_logs,
            certificates,
        };

        // Persist report in database
        let pkg_json = serde_json::to_string(&package).unwrap_or_default();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO triage_runs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                summary.triage_id,
                summary.host_name,
                summary.os_type,
                summary.collected_at,
                summary.total_processes as i64,
                summary.total_sockets as i64,
                summary.total_files_scanned as i64,
                summary.total_logs_analyzed as i64,
                summary.threats_detected as i64,
                summary.threat_score as i64,
                summary.severity.as_str(),
                pkg_json
            ],
        )?;

        Ok(package)
    }

    /// Lists past triage run summaries
    pub fn list_triage_runs(&self) -> Result<Vec<TriageSummary>, DfirError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT triage_id, host_name, os_type, collected_at, total_processes, total_sockets, total_files_scanned, total_logs_analyzed, threats_detected, threat_score, severity
             FROM triage_runs ORDER BY collected_at DESC LIMIT 50;",
        )?;

        let rows = stmt.query_map([], |row| {
            let sev_str: String = row.get(10)?;
            let severity = match sev_str.as_str() {
                "critical" => Severity::Critical,
                "high" => Severity::High,
                "medium" => Severity::Medium,
                "low" => Severity::Low,
                _ => Severity::Informational,
            };

            Ok(TriageSummary {
                triage_id: row.get(0)?,
                host_name: row.get(1)?,
                os_type: row.get(2)?,
                collected_at: row.get(3)?,
                total_processes: row.get::<_, i64>(4)? as usize,
                total_sockets: row.get::<_, i64>(5)? as usize,
                total_files_scanned: row.get::<_, i64>(6)? as usize,
                total_logs_analyzed: row.get::<_, i64>(7)? as usize,
                threats_detected: row.get::<_, i64>(8)? as usize,
                threat_score: row.get::<_, i64>(9)? as u32,
                severity,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Retrieves full triage artifact package by ID
    pub fn get_triage_package(&self, triage_id: &str) -> Result<TriagePackage, DfirError> {
        let conn = self.conn.lock().unwrap();
        let json_str: String = if triage_id == "latest" || triage_id.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT package_json FROM triage_runs ORDER BY collected_at DESC LIMIT 1;",
            )?;
            stmt.query_row([], |r| r.get(0))
                .map_err(|_| DfirError::NotFound("latest".to_string()))?
        } else {
            let mut stmt =
                conn.prepare("SELECT package_json FROM triage_runs WHERE triage_id = ?1;")?;
            stmt.query_row([triage_id], |r| r.get(0))
                .map_err(|_| DfirError::NotFound(triage_id.to_string()))?
        };

        let package: TriagePackage =
            serde_json::from_str(&json_str).unwrap_or_else(|_| TriagePackage {
                summary: TriageSummary {
                    triage_id: triage_id.to_string(),
                    host_name: "UNKNOWN".to_string(),
                    os_type: "unknown".to_string(),
                    collected_at: 0,
                    total_processes: 0,
                    total_sockets: 0,
                    total_files_scanned: 0,
                    total_logs_analyzed: 0,
                    threats_detected: 0,
                    threat_score: 0,
                    severity: Severity::Low,
                },
                mitre_techniques: Vec::new(),
                processes: Vec::new(),
                sockets: Vec::new(),
                suspicious_files: Vec::new(),
                event_logs: Vec::new(),
                certificates: Vec::new(),
            });

        Ok(package)
    }

    /// Retrieves global statistics
    pub fn get_stats(&self) -> Result<DfirStats, DfirError> {
        let conn = self.conn.lock().unwrap();
        let total_runs: usize =
            conn.query_row("SELECT COUNT(*) FROM triage_runs;", [], |r| r.get(0))?;
        let total_threats: usize = conn.query_row(
            "SELECT COALESCE(SUM(threats_detected), 0) FROM triage_runs;",
            [],
            |r| r.get(0),
        )?;
        let total_files: usize = conn.query_row(
            "SELECT COALESCE(SUM(total_files_scanned), 0) FROM triage_runs;",
            [],
            |r| r.get(0),
        )?;

        Ok(DfirStats {
            total_triage_runs: total_runs,
            total_threats_flagged: total_threats,
            total_files_hashed: total_files,
        })
    }

    /// Exports and cryptographically signs a triage package into a .dfir bundle
    pub fn export_signed_bundle(
        &self,
        triage_id: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<ChainOfCustodyManifest, DfirError> {
        let package = self.get_triage_package(triage_id)?;
        let mitre = if !package.mitre_techniques.is_empty() {
            package.mitre_techniques.clone()
        } else {
            MitreEngine::map_artifacts(
                &package.processes,
                &package.sockets,
                &package.suspicious_files,
                &package.event_logs,
            )
        };
        EvidenceSealer::seal_package(&package, &mitre, output_path)
    }

    /// Verifies an existing .dfir cryptographic evidence package
    pub fn verify_signed_bundle(
        &self,
        bundle_path: impl AsRef<Path>,
    ) -> Result<PackageVerificationReport, DfirError> {
        EvidenceSealer::verify_package(bundle_path)
    }

    /// Retrieves or computes MITRE ATT&CK technique mappings for a triage run
    pub fn get_mitre_mapping(&self, triage_id: &str) -> Result<Vec<MitreAttackMapping>, DfirError> {
        let package = self.get_triage_package(triage_id)?;
        if !package.mitre_techniques.is_empty() {
            Ok(package.mitre_techniques)
        } else {
            Ok(MitreEngine::map_artifacts(
                &package.processes,
                &package.sockets,
                &package.suspicious_files,
                &package.event_logs,
            ))
        }
    }

    /// Parses an offline Windows Event Log (.evtx) file
    pub fn parse_evtx_file(
        &self,
        path: impl AsRef<Path>,
        max_records: usize,
    ) -> Result<Vec<LogEventRecord>, DfirError> {
        EvtxParser::parse_file(path, max_records).map_err(DfirError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfir_service_triage_flow() {
        let service = DfirService::open_in_memory().unwrap();
        let pkg = service.execute_triage(None).unwrap();

        assert!(!pkg.summary.triage_id.is_empty());
        assert!(pkg.summary.total_processes > 0);
        assert!(pkg.summary.total_sockets > 0);
        assert!(!pkg.processes.is_empty());

        let list = service.list_triage_runs().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].triage_id, pkg.summary.triage_id);

        let retrieved = service.get_triage_package(&pkg.summary.triage_id).unwrap();
        assert_eq!(
            retrieved.summary.threats_detected,
            pkg.summary.threats_detected
        );

        let stats = service.get_stats().unwrap();
        assert_eq!(stats.total_triage_runs, 1);

        // Test MITRE mapping retrieval
        let mitre = service.get_mitre_mapping(&pkg.summary.triage_id).unwrap();
        // Even on clean systems, process list yields execution techniques or mappings
        assert_eq!(mitre.len(), pkg.mitre_techniques.len());

        // Test bundle export & verification
        let bundle_file = tempfile::NamedTempFile::new().unwrap();
        let manifest = service
            .export_signed_bundle(&pkg.summary.triage_id, bundle_file.path())
            .unwrap();
        assert_eq!(manifest.package_id, pkg.summary.triage_id);
        assert!(!manifest.ed25519_signature.is_empty());

        let verify_report = service.verify_signed_bundle(bundle_file.path()).unwrap();
        assert!(verify_report.is_valid);
        assert!(verify_report.signature_verified);
    }
}
