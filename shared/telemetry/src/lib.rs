#![forbid(unsafe_code)]
//! Shared Telemetry Subsystem
//!
//! Provides strictly opt-in, local-only performance counters and diagnostic metrics.
//! Zero network communication, zero external exfiltration. Default is 100% disabled.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Telemetry error definitions
#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Telemetry lock poisoned")]
    LockPoisoned,
    #[error("Audit verification error: {0}")]
    AuditError(String),
}

/// Aggregated metrics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct MetricsSnapshot {
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub latency_avg_ms: HashMap<String, f64>,
    pub sample_counts: HashMap<String, u64>,
}

/// Internal metric accumulator
#[derive(Default)]
struct MetricsStore {
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    latencies: HashMap<String, Vec<f64>>,
}

/// Local performance telemetry monitor
#[derive(Clone)]
pub struct LocalTelemetry {
    enabled: Arc<AtomicBool>,
    store: Arc<RwLock<MetricsStore>>,
}

impl Default for LocalTelemetry {
    /// Nonnegotiable: telemetry disabled by default
    fn default() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            store: Arc::new(RwLock::new(MetricsStore::default())),
        }
    }
}

impl LocalTelemetry {
    /// Create a new monitor with explicit opt-in flag.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(enabled)),
            store: Arc::new(RwLock::new(MetricsStore::default())),
        }
    }

    /// Check if local telemetry is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable or disable telemetry at runtime.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Record a discrete event counter. No-op if telemetry is disabled.
    pub fn record_counter(&self, name: &str, increment: u64) {
        if !self.is_enabled() {
            return;
        }

        if let Ok(mut store) = self.store.write() {
            *store.counters.entry(name.to_string()).or_insert(0) += increment;
        }
    }

    /// Record an instantaneous gauge value. No-op if telemetry is disabled.
    pub fn record_gauge(&self, name: &str, value: f64) {
        if !self.is_enabled() {
            return;
        }

        if let Ok(mut store) = self.store.write() {
            store.gauges.insert(name.to_string(), value);
        }
    }

    /// Record an execution duration in milliseconds. No-op if telemetry is disabled.
    pub fn record_duration_ms(&self, name: &str, duration_ms: f64) {
        if !self.is_enabled() {
            return;
        }

        if let Ok(mut store) = self.store.write() {
            store
                .latencies
                .entry(name.to_string())
                .or_default()
                .push(duration_ms);
        }
    }

    /// Generate an aggregated snapshot of all recorded metrics.
    pub fn snapshot(&self) -> Result<MetricsSnapshot, TelemetryError> {
        let store = self
            .store
            .read()
            .map_err(|_| TelemetryError::LockPoisoned)?;

        let mut latency_avg_ms = HashMap::new();
        let mut sample_counts = HashMap::new();

        for (k, v) in &store.latencies {
            let count = v.len() as u64;
            sample_counts.insert(k.clone(), count);
            if count > 0 {
                let sum: f64 = v.iter().sum();
                latency_avg_ms.insert(k.clone(), sum / (count as f64));
            }
        }

        Ok(MetricsSnapshot {
            counters: store.counters.clone(),
            gauges: store.gauges.clone(),
            latency_avg_ms,
            sample_counts,
        })
    }

    /// Export metrics snapshot to local JSON string.
    pub fn export_json(&self) -> Result<String, TelemetryError> {
        let snap = self.snapshot()?;
        Ok(serde_json::to_string_pretty(&snap)?)
    }
}

/// Single tamper-evident audit log event secured with SHA-256 cryptographic hash chaining (RFC 6962-inspired)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub sequence_id: u64,
    pub timestamp_epoch_ms: u64,
    pub app_id: String,
    pub action: String,
    pub actor: String,
    pub resource: String,
    pub status: String,
    pub details: serde_json::Value,
    pub prev_hash: String,
    pub event_hash: String,
}

impl AuditEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn compute_hash(
        sequence_id: u64,
        timestamp_epoch_ms: u64,
        app_id: &str,
        action: &str,
        actor: &str,
        resource: &str,
        status: &str,
        details: &serde_json::Value,
        prev_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(sequence_id.to_le_bytes());
        hasher.update(timestamp_epoch_ms.to_le_bytes());
        hasher.update(app_id.as_bytes());
        hasher.update(action.as_bytes());
        hasher.update(actor.as_bytes());
        hasher.update(resource.as_bytes());
        hasher.update(status.as_bytes());
        let details_str = serde_json::to_string(details).unwrap_or_default();
        hasher.update(details_str.as_bytes());
        hasher.update(prev_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Audit chain verification report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AuditVerificationReport {
    pub total_events: usize,
    pub is_valid: bool,
    pub broken_index: Option<usize>,
    pub message: String,
}

/// Append-only local audit log sink with tamper-evident SHA-256 hash chaining
pub struct LocalAuditSink {
    app_id: String,
    log_file_path: PathBuf,
    state: RwLock<(u64, String)>, // (last_sequence_id, last_event_hash)
}

impl LocalAuditSink {
    pub fn open<P: AsRef<Path>>(log_dir: P, app_id: &str) -> Result<Self, TelemetryError> {
        let dir = log_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let log_file_path = dir.join(format!("{}-audit.jsonl", app_id));

        // Scan existing file to resume sequence_id and last_event_hash
        let mut last_seq = 0u64;
        let mut last_hash = "GENESIS_ROOT_HASH".to_string();

        if log_file_path.exists() {
            let file = File::open(&log_file_path)?;
            let reader = BufReader::new(file);
            for line_res in reader.lines() {
                let line = line_res?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                    last_seq = event.sequence_id;
                    last_hash = event.event_hash;
                }
            }
        }

        Ok(Self {
            app_id: app_id.to_string(),
            log_file_path,
            state: RwLock::new((last_seq, last_hash)),
        })
    }

    pub fn record(
        &self,
        action: &str,
        actor: &str,
        resource: &str,
        status: &str,
        details: serde_json::Value,
    ) -> Result<AuditEvent, TelemetryError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| TelemetryError::LockPoisoned)?;

        let seq = state.0 + 1;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let prev_hash = state.1.clone();
        let event_hash = AuditEvent::compute_hash(
            seq,
            timestamp,
            &self.app_id,
            action,
            actor,
            resource,
            status,
            &details,
            &prev_hash,
        );

        let event = AuditEvent {
            sequence_id: seq,
            timestamp_epoch_ms: timestamp,
            app_id: self.app_id.clone(),
            action: action.to_string(),
            actor: actor.to_string(),
            resource: resource.to_string(),
            status: status.to_string(),
            details,
            prev_hash,
            event_hash: event_hash.clone(),
        };

        let serialized = serde_json::to_string(&event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)?;

        writeln!(file, "{}", serialized)?;
        file.flush()?;

        state.0 = seq;
        state.1 = event_hash;

        Ok(event)
    }

    pub fn verify_log_file<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<AuditVerificationReport, TelemetryError> {
        let path = file_path.as_ref();
        if !path.exists() {
            return Ok(AuditVerificationReport {
                total_events: 0,
                is_valid: true,
                broken_index: None,
                message: "Audit file does not exist (empty log)".to_string(),
            });
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut expected_prev = "GENESIS_ROOT_HASH".to_string();
        let mut count = 0;

        for (idx, line_res) in reader.lines().enumerate() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }

            let event: AuditEvent = serde_json::from_str(&line).map_err(|e| {
                TelemetryError::AuditError(format!(
                    "Malformed audit JSON at line {}: {}",
                    idx + 1,
                    e
                ))
            })?;

            if event.prev_hash != expected_prev {
                return Ok(AuditVerificationReport {
                    total_events: count,
                    is_valid: false,
                    broken_index: Some(idx),
                    message: format!(
                        "Broken audit hash link at line {}: expected prev_hash='{}', found='{}'",
                        idx + 1,
                        expected_prev,
                        event.prev_hash
                    ),
                });
            }

            let computed = AuditEvent::compute_hash(
                event.sequence_id,
                event.timestamp_epoch_ms,
                &event.app_id,
                &event.action,
                &event.actor,
                &event.resource,
                &event.status,
                &event.details,
                &event.prev_hash,
            );

            if computed != event.event_hash {
                return Ok(AuditVerificationReport {
                    total_events: count,
                    is_valid: false,
                    broken_index: Some(idx),
                    message: format!(
                        "Tampered event payload at line {}: recomputed hash '{}' does not match '{}'",
                        idx + 1,
                        computed,
                        event.event_hash
                    ),
                });
            }

            expected_prev = event.event_hash;
            count += 1;
        }

        Ok(AuditVerificationReport {
            total_events: count,
            is_valid: true,
            broken_index: None,
            message: format!(
                "Audit chain intact: {} verified tamper-evident events",
                count
            ),
        })
    }
}

/// Offline OpenTelemetry-compatible span recording for local performance profiling
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SpanRecord {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time_epoch_ms: u64,
    pub duration_us: u64,
    pub tags: HashMap<String, String>,
}

/// Local append-only span tracer saving spans to offline `.jsonl` trace log
pub struct LocalSpanRecorder {
    trace_file_path: PathBuf,
}

impl LocalSpanRecorder {
    pub fn open<P: AsRef<Path>>(trace_file_path: P) -> Result<Self, TelemetryError> {
        let p = trace_file_path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { trace_file_path: p })
    }

    pub fn record_span(&self, span: &SpanRecord) -> Result<(), TelemetryError> {
        let line = serde_json::to_string(span)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.trace_file_path)?;
        writeln!(file, "{}", line)?;
        file.flush()?;
        Ok(())
    }

    pub fn read_spans(&self) -> Result<Vec<SpanRecord>, TelemetryError> {
        if !self.trace_file_path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.trace_file_path)?;
        let reader = BufReader::new(file);
        let mut spans = Vec::new();
        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(span) = serde_json::from_str::<SpanRecord>(&line) {
                spans.push(span);
            }
        }
        Ok(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_disabled_by_default() {
        let tel = LocalTelemetry::default();
        assert!(!tel.is_enabled());

        tel.record_counter("search_queries", 10);
        tel.record_gauge("memory_mb", 45.2);
        tel.record_duration_ms("open_file_latency", 12.5);

        let snap = tel.snapshot().unwrap();
        // Since it was disabled, nothing should be recorded
        assert!(snap.counters.is_empty());
        assert!(snap.gauges.is_empty());
        assert!(snap.latency_avg_ms.is_empty());
    }

    #[test]
    fn test_telemetry_opt_in_recording() {
        let tel = LocalTelemetry::new(true);
        assert!(tel.is_enabled());

        tel.record_counter("search_queries", 5);
        tel.record_counter("search_queries", 3);
        tel.record_gauge("resident_ram_mb", 180.0);
        tel.record_duration_ms("query_time", 20.0);
        tel.record_duration_ms("query_time", 40.0);

        let snap = tel.snapshot().unwrap();
        assert_eq!(*snap.counters.get("search_queries").unwrap(), 8);
        assert_eq!(*snap.gauges.get("resident_ram_mb").unwrap(), 180.0);
        assert_eq!(*snap.sample_counts.get("query_time").unwrap(), 2);
        assert_eq!(*snap.latency_avg_ms.get("query_time").unwrap(), 30.0);

        let json = tel.export_json().unwrap();
        assert!(json.contains("search_queries"));
        assert!(json.contains("resident_ram_mb"));
    }

    #[test]
    fn test_audit_sink_hash_chaining_and_tamper_detection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sink = LocalAuditSink::open(temp_dir.path(), "devops-secret-vault").unwrap();

        // 1. Record 3 valid audit events
        let ev1 = sink
            .record(
                "vault_unlock",
                "admin",
                "vault:primary",
                "SUCCESS",
                serde_json::json!({"auth": "biometric"}),
            )
            .unwrap();
        assert_eq!(ev1.sequence_id, 1);
        assert_eq!(ev1.prev_hash, "GENESIS_ROOT_HASH");

        let ev2 = sink
            .record(
                "secret_reveal",
                "admin",
                "secret:aws_prod",
                "SUCCESS",
                serde_json::json!({"fields": ["key", "secret"]}),
            )
            .unwrap();
        assert_eq!(ev2.sequence_id, 2);
        assert_eq!(ev2.prev_hash, ev1.event_hash);

        let ev3 = sink
            .record(
                "export_snapshot",
                "admin",
                "backup:vault.bak",
                "SUCCESS",
                serde_json::json!({"bytes": 4096}),
            )
            .unwrap();
        assert_eq!(ev3.sequence_id, 3);
        assert_eq!(ev3.prev_hash, ev2.event_hash);

        // 2. Verify intact chain
        let report = LocalAuditSink::verify_log_file(&sink.log_file_path).unwrap();
        assert!(report.is_valid);
        assert_eq!(report.total_events, 3);
        assert!(report.broken_index.is_none());

        // 3. Re-open sink on same path - should continue sequence
        let sink2 = LocalAuditSink::open(temp_dir.path(), "devops-secret-vault").unwrap();
        let ev4 = sink2
            .record(
                "vault_lock",
                "system",
                "vault:primary",
                "SUCCESS",
                serde_json::json!({}),
            )
            .unwrap();
        assert_eq!(ev4.sequence_id, 4);
        assert_eq!(ev4.prev_hash, ev3.event_hash);

        let report2 = LocalAuditSink::verify_log_file(&sink2.log_file_path).unwrap();
        assert!(report2.is_valid);
        assert_eq!(report2.total_events, 4);

        // 4. Test tamper detection: modify one byte in event 2
        let raw = std::fs::read_to_string(&sink.log_file_path).unwrap();
        let tampered = raw.replacen("aws_prod", "hacked_prod", 1);
        let tampered_path = temp_dir.path().join("tampered-audit.jsonl");
        std::fs::write(&tampered_path, tampered).unwrap();

        let tamper_report = LocalAuditSink::verify_log_file(&tampered_path).unwrap();
        assert!(!tamper_report.is_valid);
        assert_eq!(tamper_report.broken_index, Some(1)); // Line index 1 is ev2
    }

    #[test]
    fn test_span_recorder_offline_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let trace_path = temp_dir.path().join("traces.jsonl");
        let recorder = LocalSpanRecorder::open(&trace_path).unwrap();

        let mut tags = HashMap::new();
        tags.insert("component".to_string(), "dsp_engine".to_string());
        tags.insert("app".to_string(), "audio-mastering-studio".to_string());

        let span = SpanRecord {
            trace_id: "trace-abc-123".to_string(),
            span_id: "span-001".to_string(),
            parent_span_id: None,
            name: "fft_frequency_decomposition".to_string(),
            start_time_epoch_ms: 1730000000000,
            duration_us: 1420,
            tags,
        };

        recorder.record_span(&span).unwrap();
        let loaded = recorder.read_spans().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "fft_frequency_decomposition");
        assert_eq!(loaded[0].duration_us, 1420);
    }
}
