#![forbid(unsafe_code)]
//! Shared Telemetry Subsystem
//!
//! Provides strictly opt-in, local-only performance counters and diagnostic metrics.
//! Zero network communication, zero external exfiltration. Default is 100% disabled.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Telemetry error definitions
#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Telemetry lock poisoned")]
    LockPoisoned,
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
}
