#![forbid(unsafe_code)]
//! Offline AWS CloudWatch and Kubernetes Log Importers & Normalizers
//!
//! Provides offline extraction, normalization, and multi-pod/container correlation
//! for logs exported from enterprise cloud environments with zero network connectivity.

use crate::parsers::{parse_timestamp, LogSeverity, NormalizedLogRecord};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// AWS CloudWatch log event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudWatchEvent {
    pub timestamp: Option<i64>,
    pub message: String,
    #[serde(rename = "ingestionTime")]
    pub ingestion_time: Option<i64>,
    #[serde(rename = "logStream")]
    pub log_stream: Option<String>,
    #[serde(rename = "logGroup")]
    pub log_group: Option<String>,
}

/// Kubernetes container log metadata extracted from path or header
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KubernetesMetadata {
    pub namespace: String,
    pub pod_name: String,
    pub container_name: String,
    pub stream: String, // stdout / stderr
}

/// Correlated multi-container failure event chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedFailureChain {
    pub root_error_line: usize,
    pub root_error_time_ms: i64,
    pub root_container: KubernetesMetadata,
    pub root_message: String,
    pub preceding_events: Vec<CorrelatedEvent>,
    pub succeeding_events: Vec<CorrelatedEvent>,
}

/// Individual correlated event in a failure chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedEvent {
    pub line_number: usize,
    pub time_delta_ms: i64,
    pub container: KubernetesMetadata,
    pub severity: LogSeverity,
    pub message: String,
}

/// Offline AWS CloudWatch log importer
pub struct CloudWatchImporter;

impl CloudWatchImporter {
    /// Parse a CloudWatch JSON-line or single JSON event
    pub fn parse_line(line_number: usize, line: &str) -> Option<NormalizedLogRecord> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Ok(event) = serde_json::from_str::<CloudWatchEvent>(trimmed) {
            let mut attributes = BTreeMap::new();
            if let Some(ref group) = event.log_group {
                attributes.insert("log_group".to_string(), group.clone());
            }
            if let Some(ref stream) = event.log_stream {
                attributes.insert("log_stream".to_string(), stream.clone());
            }
            if let Some(ingest) = event.ingestion_time {
                attributes.insert("ingestion_time".to_string(), ingest.to_string());
            }

            let severity = LogSeverity::parse_str(&event.message);

            return Some(NormalizedLogRecord {
                line_number,
                raw: line.to_string(),
                timestamp_raw: event.timestamp.map(|t| t.to_string()),
                timestamp_epoch_ms: event.timestamp,
                severity,
                message: event.message,
                source: event.log_stream,
                attributes,
            });
        }

        None
    }
}

/// Offline Kubernetes log importer and multi-container correlation engine
pub struct KubernetesImporter;

impl KubernetesImporter {
    /// Extract Kubernetes metadata from standard log file path
    /// Format: .../pods/<namespace>_<pod_name>_<uid>/<container_name>/...log
    pub fn extract_metadata_from_path(path: &str) -> Option<KubernetesMetadata> {
        let normalized = path.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').collect();

        // Search for pods directory pattern
        for (i, &segment) in segments.iter().enumerate() {
            if segment == "pods" && i + 2 < segments.len() {
                let pod_dir = segments[i + 1];
                let container_name = segments[i + 2].to_string();

                let pod_parts: Vec<&str> = pod_dir.split('_').collect();
                if pod_parts.len() >= 2 {
                    return Some(KubernetesMetadata {
                        namespace: pod_parts[0].to_string(),
                        pod_name: pod_parts[1].to_string(),
                        container_name,
                        stream: "stdout".to_string(),
                    });
                }
            }
        }

        None
    }

    /// Parse a CRI format line: <timestamp> (stdout|stderr) (F|P) <log_payload>
    pub fn parse_cri_line(
        line_number: usize,
        line: &str,
        default_meta: Option<&KubernetesMetadata>,
    ) -> NormalizedLogRecord {
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.splitn(4, ' ').collect();

        if parts.len() >= 4
            && (parts[1] == "stdout" || parts[1] == "stderr")
            && (parts[2] == "F" || parts[2] == "P")
        {
            let ts_str = parts[0];
            let stream = parts[1];
            let payload = parts[3];

            let epoch_ms = parse_timestamp(ts_str);
            let severity = if stream == "stderr" {
                let parsed = LogSeverity::parse_str(payload);
                if parsed == LogSeverity::Unknown {
                    LogSeverity::Warn
                } else {
                    parsed
                }
            } else {
                LogSeverity::parse_str(payload)
            };

            let mut attributes = BTreeMap::new();
            attributes.insert("stream".to_string(), stream.to_string());
            attributes.insert("tag".to_string(), parts[2].to_string());

            let mut source = None;
            if let Some(meta) = default_meta {
                attributes.insert("namespace".to_string(), meta.namespace.clone());
                attributes.insert("pod_name".to_string(), meta.pod_name.clone());
                attributes.insert("container_name".to_string(), meta.container_name.clone());
                source = Some(format!(
                    "{}/{}/{}",
                    meta.namespace, meta.pod_name, meta.container_name
                ));
            }

            NormalizedLogRecord {
                line_number,
                raw: line.to_string(),
                timestamp_raw: Some(ts_str.to_string()),
                timestamp_epoch_ms: epoch_ms,
                severity,
                message: payload.to_string(),
                source,
                attributes,
            }
        } else {
            // Raw fallback
            NormalizedLogRecord {
                line_number,
                raw: line.to_string(),
                timestamp_raw: None,
                timestamp_epoch_ms: None,
                severity: LogSeverity::Unknown,
                message: line.to_string(),
                source: default_meta.map(|m| format!("{}/{}", m.pod_name, m.container_name)),
                attributes: BTreeMap::new(),
            }
        }
    }

    /// Correlate failures across multiple containers and pods within a sliding time window (delta_ms)
    pub fn correlate_failures(
        records: &[NormalizedLogRecord],
        window_ms: i64,
    ) -> Vec<CorrelatedFailureChain> {
        let mut chains = Vec::new();
        let mut meta_cache: HashMap<usize, KubernetesMetadata> = HashMap::new();

        // Populate metadata for indexed records
        for (idx, r) in records.iter().enumerate() {
            let ns = r
                .attributes
                .get("namespace")
                .cloned()
                .unwrap_or_else(|| "default".to_string());
            let pod = r
                .attributes
                .get("pod_name")
                .cloned()
                .unwrap_or_else(|| "unknown-pod".to_string());
            let container = r
                .attributes
                .get("container_name")
                .cloned()
                .unwrap_or_else(|| "app".to_string());
            let stream = r
                .attributes
                .get("stream")
                .cloned()
                .unwrap_or_else(|| "stdout".to_string());

            meta_cache.insert(
                idx,
                KubernetesMetadata {
                    namespace: ns,
                    pod_name: pod,
                    container_name: container,
                    stream,
                },
            );
        }

        for (idx, r) in records.iter().enumerate() {
            if r.severity >= LogSeverity::Error {
                if let Some(err_time) = r.timestamp_epoch_ms {
                    let root_meta = meta_cache.get(&idx).cloned().unwrap();

                    let mut preceding = Vec::new();
                    let mut succeeding = Vec::new();

                    // Look behind for correlated warnings or errors in other containers
                    for (other_idx, other) in records.iter().enumerate() {
                        if other_idx == idx {
                            continue;
                        }
                        if let Some(other_time) = other.timestamp_epoch_ms {
                            let delta = other_time - err_time;
                            if delta.abs() <= window_ms {
                                let other_meta = meta_cache.get(&other_idx).cloned().unwrap();
                                // Include warnings or errors, or stdout right before crash
                                if other.severity >= LogSeverity::Warn
                                    || (-1000..0).contains(&delta)
                                {
                                    let ev = CorrelatedEvent {
                                        line_number: other.line_number,
                                        time_delta_ms: delta,
                                        container: other_meta,
                                        severity: other.severity,
                                        message: other.message.clone(),
                                    };
                                    if delta < 0 {
                                        preceding.push(ev);
                                    } else {
                                        succeeding.push(ev);
                                    }
                                }
                            }
                        }
                    }

                    if !preceding.is_empty() || !succeeding.is_empty() {
                        chains.push(CorrelatedFailureChain {
                            root_error_line: r.line_number,
                            root_error_time_ms: err_time,
                            root_container: root_meta,
                            root_message: r.message.clone(),
                            preceding_events: preceding,
                            succeeding_events: succeeding,
                        });
                    }
                }
            }
        }

        chains
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloudwatch_importer() {
        let cw_json = r#"{"timestamp": 1700000000000, "message": "ERROR Database connection pool exhausted", "logGroup": "/aws/lambda/checkout-svc", "logStream": "2026/10/19/[$LATEST]abc123"}"#;
        let record = CloudWatchImporter::parse_line(1, cw_json).expect("should parse");
        assert_eq!(record.severity, LogSeverity::Error);
        assert_eq!(record.timestamp_epoch_ms, Some(1700000000000));
        assert_eq!(
            record.attributes.get("log_group").map(String::as_str),
            Some("/aws/lambda/checkout-svc")
        );
    }

    #[test]
    fn test_kubernetes_metadata_extraction() {
        let path = "/var/log/pods/prod-payment_payment-service-789fbd76c-x9k2p_4b2c1d/payment-container/0.log";
        let meta = KubernetesImporter::extract_metadata_from_path(path).expect("should extract");
        assert_eq!(meta.namespace, "prod-payment");
        assert_eq!(meta.pod_name, "payment-service-789fbd76c-x9k2p");
        assert_eq!(meta.container_name, "payment-container");
    }

    #[test]
    fn test_kubernetes_cri_line_parsing() {
        let meta = KubernetesMetadata {
            namespace: "default".to_string(),
            pod_name: "auth-pod".to_string(),
            container_name: "auth-svc".to_string(),
            stream: "stderr".to_string(),
        };
        let line = "2026-10-19T10:15:30.123456789Z stderr F [ERROR] Token verification failed for user 9021";
        let record = KubernetesImporter::parse_cri_line(5, line, Some(&meta));

        assert_eq!(record.line_number, 5);
        assert_eq!(record.severity, LogSeverity::Error);
        assert_eq!(
            record.attributes.get("stream").map(String::as_str),
            Some("stderr")
        );
        assert_eq!(
            record.attributes.get("namespace").map(String::as_str),
            Some("default")
        );
        assert_eq!(
            record.message,
            "[ERROR] Token verification failed for user 9021"
        );
    }

    #[test]
    fn test_multi_container_correlation() {
        let meta_auth = KubernetesMetadata {
            namespace: "prod".to_string(),
            pod_name: "auth-pod".to_string(),
            container_name: "auth".to_string(),
            stream: "stdout".to_string(),
        };
        let meta_db = KubernetesMetadata {
            namespace: "prod".to_string(),
            pod_name: "db-pod".to_string(),
            container_name: "postgres".to_string(),
            stream: "stderr".to_string(),
        };

        let mut r1 = KubernetesImporter::parse_cri_line(
            1,
            "2026-10-19T10:00:00.000Z stderr F WARN Deadlock detected on lock table",
            Some(&meta_db),
        );
        r1.timestamp_epoch_ms = Some(1700000000000);

        let mut r2 = KubernetesImporter::parse_cri_line(
            2,
            "2026-10-19T10:00:01.000Z stdout F ERROR Authentication service query failed: connection closed",
            Some(&meta_auth),
        );
        r2.timestamp_epoch_ms = Some(1700000001000);

        let records = vec![r1, r2];
        let chains = KubernetesImporter::correlate_failures(&records, 5000);

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].root_error_line, 2);
        assert_eq!(chains[0].preceding_events.len(), 1);
        assert_eq!(chains[0].preceding_events[0].line_number, 1);
    }
}
