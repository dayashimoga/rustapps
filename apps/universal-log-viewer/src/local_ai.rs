#![forbid(unsafe_code)]
//! Local AI and Statistical Root-Cause Analysis Engine
//!
//! Provides 100% offline log-template mining (Drain algorithm), statistical
//! anomaly spike detection (z-score / IQR), error clustering, and correlated failure tracing
//! with zero external cloud dependencies or heavy neural weights.

use crate::parsers::{LogSeverity, NormalizedLogRecord};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mined log template cluster
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogTemplateCluster {
    pub cluster_id: usize,
    pub template: String,
    pub token_count: usize,
    pub frequency: usize,
    pub sample_lines: Vec<usize>,
}

/// An identified anomaly spike in volume or error density
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalySpike {
    pub spike_id: usize,
    pub start_epoch_ms: i64,
    pub end_epoch_ms: i64,
    pub z_score: f64,
    pub observed_value: f64,
    pub expected_value: f64,
    pub metric_name: String,
    pub description: String,
    pub severity: LogSeverity,
}

/// Clustered error group for root cause investigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCluster {
    pub cluster_id: usize,
    pub template: String,
    pub error_count: usize,
    pub affected_sources: Vec<String>,
    pub sample_lines: Vec<usize>,
}

/// Node in the Drain prefix tree
#[derive(Debug)]
struct DrainNode {
    children: HashMap<String, DrainNode>,
    cluster_ids: Vec<usize>,
}

impl DrainNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            cluster_ids: Vec::new(),
        }
    }
}

/// Offline Drain log parser and template miner
pub struct DrainLogMiner {
    depth: usize,
    similarity_threshold: f64,
    root: DrainNode,
    clusters: Vec<LogTemplateCluster>,
    mask_regexes: Vec<(Regex, &'static str)>,
}

impl Default for DrainLogMiner {
    fn default() -> Self {
        Self::new()
    }
}

impl DrainLogMiner {
    /// Create a new Drain miner with default parameter depth=4 and similarity=0.5
    pub fn new() -> Self {
        let mask_regexes = vec![
            // IP Addresses
            (Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(), "<IP>"),
            // UUIDs
            (
                Regex::new(r"\b[0-9a-fA-F]{8}-(?:[0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}\b").unwrap(),
                "<UUID>",
            ),
            // Hexadecimal hashes / memory pointers
            (
                Regex::new(r"\b0x[0-9a-fA-F]+\b|\b[0-9a-fA-F]{16,64}\b").unwrap(),
                "<HEX>",
            ),
            // File paths
            (Regex::new(r"(/[a-zA-Z0-9_\.\-]+)+").unwrap(), "<PATH>"),
            // Numbers / durations
            (Regex::new(r"\b\d+\b").unwrap(), "<NUM>"),
        ];

        Self {
            depth: 4,
            similarity_threshold: 0.5,
            root: DrainNode::new(),
            clusters: Vec::new(),
            mask_regexes,
        }
    }

    /// Preprocess and mask variables from a raw log message
    pub fn mask_message(&self, msg: &str) -> String {
        let mut masked = msg.to_string();
        for (re, replacement) in &self.mask_regexes {
            masked = re.replace_all(&masked, *replacement).into_owned();
        }
        masked
    }

    /// Tokenize masked message into whitespace-delimited tokens
    pub fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Compute similarity between two token lists of the same length
    pub fn similarity(seq1: &[String], seq2: &[String]) -> (f64, usize) {
        if seq1.len() != seq2.len() || seq1.is_empty() {
            return (0.0, 0);
        }

        let mut sim_tokens = 0usize;
        for (t1, t2) in seq1.iter().zip(seq2.iter()) {
            if t1 == t2 {
                sim_tokens += 1;
            }
        }

        let ratio = (sim_tokens as f64) / (seq1.len() as f64);
        (ratio, sim_tokens)
    }

    /// Mine template from a single log message
    pub fn add_log(&mut self, line_number: usize, raw_msg: &str) -> usize {
        let masked = self.mask_message(raw_msg);
        let tokens = Self::tokenize(&masked);
        let seq_len = tokens.len();

        if seq_len == 0 {
            return 0;
        }

        // Search prefix tree: Root -> length layer -> token layers
        let len_key = seq_len.to_string();
        let len_node = self
            .root
            .children
            .entry(len_key)
            .or_insert_with(DrainNode::new);

        let mut current_node = len_node;
        let search_depth = self.depth.min(seq_len);

        for token in tokens.iter().take(search_depth) {
            let key = if token.starts_with('<') && token.ends_with('>') {
                "<*>".to_string()
            } else {
                token.clone()
            };

            current_node = current_node
                .children
                .entry(key)
                .or_insert_with(DrainNode::new);
        }

        // Find best matching cluster in candidate list
        let mut best_match: Option<(usize, f64)> = None;
        for &c_id in &current_node.cluster_ids {
            let cluster = &self.clusters[c_id];
            let cluster_tokens = Self::tokenize(&cluster.template);
            let (sim, _) = Self::similarity(&tokens, &cluster_tokens);

            if sim >= self.similarity_threshold {
                if let Some((_, best_sim)) = best_match {
                    if sim > best_sim {
                        best_match = Some((c_id, sim));
                    }
                } else {
                    best_match = Some((c_id, sim));
                }
            }
        }

        if let Some((match_id, _)) = best_match {
            // Update matched cluster template with wildcards
            let existing_template = self.clusters[match_id].template.clone();
            let cluster_tokens = Self::tokenize(&existing_template);
            let mut updated_tokens = Vec::with_capacity(seq_len);

            for (t1, t2) in cluster_tokens.iter().zip(tokens.iter()) {
                if t1 == t2 {
                    updated_tokens.push(t1.clone());
                } else {
                    updated_tokens.push("<*>".to_string());
                }
            }

            let cluster = &mut self.clusters[match_id];
            cluster.template = updated_tokens.join(" ");
            cluster.frequency += 1;
            if cluster.sample_lines.len() < 10 {
                cluster.sample_lines.push(line_number);
            }
            match_id
        } else {
            // Create new cluster
            let cluster_id = self.clusters.len();
            let new_cluster = LogTemplateCluster {
                cluster_id,
                template: tokens.join(" "),
                token_count: seq_len,
                frequency: 1,
                sample_lines: vec![line_number],
            };
            self.clusters.push(new_cluster);
            current_node.cluster_ids.push(cluster_id);
            cluster_id
        }
    }

    /// Return all mined clusters sorted by frequency descending
    pub fn get_clusters(&self) -> Vec<LogTemplateCluster> {
        let mut sorted = self.clusters.clone();
        sorted.sort_unstable_by_key(|b| std::cmp::Reverse(b.frequency));
        sorted
    }
}

/// Offline statistical anomaly spike detector
pub struct AnomalyDetector;

impl AnomalyDetector {
    /// Detect volume spikes and error density anomalies using rolling z-scores
    pub fn detect_spikes(
        bucket_counts: &[(i64, i64, usize, usize)], // (start_ms, end_ms, total_count, error_count)
        z_threshold: f64,
    ) -> Vec<AnomalySpike> {
        if bucket_counts.len() < 3 {
            return Vec::new();
        }

        let mut spikes = Vec::new();

        // Calculate mean and standard deviation of total volume
        let n = bucket_counts.len() as f64;
        let sum_total: f64 = bucket_counts.iter().map(|b| b.2 as f64).sum();
        let mean_total = sum_total / n;

        let variance_total: f64 = bucket_counts
            .iter()
            .map(|b| (b.2 as f64 - mean_total).powi(2))
            .sum::<f64>()
            / n;
        let std_dev_total = variance_total.sqrt().max(1.0);

        // Calculate mean and std dev of error rates
        let sum_errors: f64 = bucket_counts.iter().map(|b| b.3 as f64).sum();
        let mean_errors = sum_errors / n;

        let variance_errors: f64 = bucket_counts
            .iter()
            .map(|b| (b.3 as f64 - mean_errors).powi(2))
            .sum::<f64>()
            / n;
        let std_dev_errors = variance_errors.sqrt().max(1.0);

        let mut spike_counter = 0usize;

        for (i, &(start_ms, end_ms, total, errors)) in bucket_counts.iter().enumerate() {
            let total_f = total as f64;
            let error_f = errors as f64;

            // Check volume spike
            let z_volume = (total_f - mean_total) / std_dev_total;
            if z_volume >= z_threshold && total_f > mean_total * 1.5 {
                spike_counter += 1;
                spikes.push(AnomalySpike {
                    spike_id: spike_counter,
                    start_epoch_ms: start_ms,
                    end_epoch_ms: end_ms,
                    z_score: (z_volume * 100.0).round() / 100.0,
                    observed_value: total_f,
                    expected_value: (mean_total * 100.0).round() / 100.0,
                    metric_name: "log_volume".to_string(),
                    description: format!(
                        "Sudden volume surge in bucket {}: {} logs (expected {:.1}, z-score: {:.2})",
                        i, total, mean_total, z_volume
                    ),
                    severity: LogSeverity::Warn,
                });
            }

            // Check error surge
            let z_error = (error_f - mean_errors) / std_dev_errors;
            if z_error >= z_threshold && errors >= 3 && error_f > mean_errors * 1.5 {
                spike_counter += 1;
                spikes.push(AnomalySpike {
                    spike_id: spike_counter,
                    start_epoch_ms: start_ms,
                    end_epoch_ms: end_ms,
                    z_score: (z_error * 100.0).round() / 100.0,
                    observed_value: error_f,
                    expected_value: (mean_errors * 100.0).round() / 100.0,
                    metric_name: "error_density".to_string(),
                    description: format!(
                        "Critical error surge in bucket {}: {} errors (expected {:.1}, z-score: {:.2})",
                        i, errors, mean_errors, z_error
                    ),
                    severity: LogSeverity::Error,
                });
            }
        }

        spikes
    }

    /// Cluster error logs and group by template to identify primary root causes
    pub fn cluster_errors(records: &[NormalizedLogRecord]) -> Vec<ErrorCluster> {
        let mut miner = DrainLogMiner::new();
        let mut error_records = Vec::new();

        for r in records {
            if r.severity >= LogSeverity::Error {
                let cluster_id = miner.add_log(r.line_number, &r.message);
                error_records.push((cluster_id, r));
            }
        }

        let clusters = miner.get_clusters();
        let mut result = Vec::new();

        for c in clusters {
            let mut affected_sources = Vec::new();
            for &(cid, r) in &error_records {
                if cid == c.cluster_id {
                    if let Some(ref src) = r.source {
                        if !affected_sources.contains(src) {
                            affected_sources.push(src.clone());
                        }
                    }
                }
            }

            result.push(ErrorCluster {
                cluster_id: c.cluster_id,
                template: c.template,
                error_count: c.frequency,
                affected_sources,
                sample_lines: c.sample_lines,
            });
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drain_template_mining() {
        let mut miner = DrainLogMiner::new();

        miner.add_log(1, "Connection to 192.168.1.10 failed on port 8080: timeout");
        miner.add_log(2, "Connection to 192.168.1.15 failed on port 8080: timeout");
        miner.add_log(3, "Connection to 10.0.0.1 failed on port 9090: timeout");

        let clusters = miner.get_clusters();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].frequency, 3);
        assert!(clusters[0]
            .template
            .contains("Connection to <IP> failed on port"));
    }

    #[test]
    fn test_anomaly_spike_detection() {
        let mut buckets = Vec::new();
        for i in 0..20 {
            // normal volume = 10, normal error = 0
            buckets.push((i * 1000, (i + 1) * 1000, 10, 0));
        }
        // inject sudden massive spike at bucket 10
        buckets[10] = (10000, 11000, 150, 45);

        let spikes = AnomalyDetector::detect_spikes(&buckets, 2.5);
        assert!(!spikes.is_empty());
        assert!(spikes.iter().any(|s| s.metric_name == "log_volume"));
        assert!(spikes.iter().any(|s| s.metric_name == "error_density"));
    }

    #[test]
    fn test_error_clustering() {
        let mut records = Vec::new();
        for i in 1..=5 {
            let r = NormalizedLogRecord {
                line_number: i,
                raw: format!("HTTP 500 error user {}", i * 100),
                timestamp_raw: None,
                timestamp_epoch_ms: None,
                severity: LogSeverity::Error,
                message: format!("HTTP 500 error user {}", i * 100),
                source: Some("auth-svc".to_string()),
                attributes: std::collections::BTreeMap::new(),
            };
            records.push(r);
        }

        let clusters = AnomalyDetector::cluster_errors(&records);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].error_count, 5);
        assert!(clusters[0]
            .affected_sources
            .contains(&"auth-svc".to_string()));
    }
}
