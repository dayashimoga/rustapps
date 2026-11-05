#![forbid(unsafe_code)]
//! Multi-File Chronological Log Merger & OpenTelemetry Distributed Trace Waterfall
//!
//! Provides sub-50ms multi-stream log interleaving by normalized timestamps,
//! source-file provenance tracking, and OpenTelemetry trace correlation.

use crate::parsers::{parse_timestamp, NormalizedLogRecord};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Individual span representation in an OpenTelemetry-compatible trace waterfall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceWaterfallSpan {
    pub span_id: Option<String>,
    pub parent_id: Option<String>,
    pub service_name: String,
    pub line_number: usize,
    pub timestamp_epoch_ms: i64,
    pub offset_ms: i64,
    pub duration_ms: u64,
    pub severity: String,
    pub message: String,
    pub source_file: String,
}

/// Full trace correlation report visualizing timing waterfall across microservices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceWaterfallReport {
    pub trace_id: String,
    pub total_spans: usize,
    pub start_time_epoch_ms: i64,
    pub end_time_epoch_ms: i64,
    pub total_duration_ms: u64,
    pub root_service: String,
    pub spans: Vec<TraceWaterfallSpan>,
}

/// Result summary of multi-file chronological log merger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedLogSummary {
    pub total_files: usize,
    pub total_lines: usize,
    pub output_path: String,
    pub min_timestamp_epoch_ms: i64,
    pub max_timestamp_epoch_ms: i64,
}

/// Internal representation of an indexed line during multi-file merge
struct TimedLogLine {
    epoch_ms: i64,
    source_index: usize,
    text: String,
}

/// Merge multiple log files into a single unified chronologically sorted log file.
///
/// Preserves line ordering, normalizes timestamps, and tags each entry with its source file.
pub fn merge_log_files_chronologically<P: AsRef<Path>, Q: AsRef<Path>>(
    file_paths: &[P],
    dest_path: Q,
    max_lines_per_file: usize,
) -> anyhow::Result<MergedLogSummary> {
    if file_paths.is_empty() {
        return Err(anyhow::anyhow!("No log files provided for merger"));
    }

    let mut all_timed_lines: Vec<TimedLogLine> = Vec::new();
    let mut min_ts = i64::MAX;
    let mut max_ts = i64::MIN;

    for (file_idx, path_ref) in file_paths.iter().enumerate() {
        let path = path_ref.as_ref();
        if !path.exists() {
            continue;
        }

        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        let fallback_time = path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let mut last_seen_time = fallback_time;

        for (line_num, line_res) in reader.lines().enumerate() {
            if line_num >= max_lines_per_file {
                break;
            }
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Extract timestamp from line using ISO-8601 or syslog parser
            let epoch_ms = parse_timestamp(trimmed).unwrap_or(last_seen_time);
            last_seen_time = epoch_ms;

            if epoch_ms < min_ts {
                min_ts = epoch_ms;
            }
            if epoch_ms > max_ts {
                max_ts = epoch_ms;
            }

            all_timed_lines.push(TimedLogLine {
                epoch_ms,
                source_index: file_idx,
                text: trimmed.to_string(),
            });
        }
    }

    if all_timed_lines.is_empty() {
        return Err(anyhow::anyhow!("No log entries found across the specified files"));
    }

    // Stable sort chronologically by timestamp
    all_timed_lines.sort_by_key(|item| item.epoch_ms);

    let dest = dest_path.as_ref();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let out_file = std::fs::File::create(dest)?;
    let mut writer = BufWriter::new(out_file);

    let total_lines = all_timed_lines.len();

    for item in &all_timed_lines {
        let file_name = file_paths[item.source_index]
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.log");

        // Prefix line with source file tag
        writeln!(writer, "[{}] {}", file_name, item.text)?;
    }

    writer.flush()?;

    if min_ts == i64::MAX {
        min_ts = 0;
    }
    if max_ts == i64::MIN {
        max_ts = 0;
    }

    Ok(MergedLogSummary {
        total_files: file_paths.len(),
        total_lines,
        output_path: dest.to_string_lossy().to_string(),
        min_timestamp_epoch_ms: min_ts,
        max_timestamp_epoch_ms: max_ts,
    })
}

/// Extract trace_id and span metadata from raw text or JSON attributes
pub fn extract_trace_metadata(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut trace_id = None;
    let mut span_id = None;
    let mut service_name = None;

    // Fast check for JSON payload
    if (text.starts_with('{') && text.ends_with('}')) || text.contains("\"trace_id\"") || text.contains("\"traceId\"") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(tid) = val.get("trace_id").or_else(|| val.get("traceId")).or_else(|| val.get("traceid")).and_then(|v| v.as_str()) {
                trace_id = Some(tid.to_string());
            }
            if let Some(sid) = val.get("span_id").or_else(|| val.get("spanId")).or_else(|| val.get("spanid")).and_then(|v| v.as_str()) {
                span_id = Some(sid.to_string());
            }
            if let Some(svc) = val.get("service").or_else(|| val.get("service_name")).or_else(|| val.get("app")).and_then(|v| v.as_str()) {
                service_name = Some(svc.to_string());
            }
        }
    }

    // Key-value / Logfmt regex fallback if not found
    if trace_id.is_none() {
        for prefix in &["trace_id=", "traceId=", "trace_id: ", "traceId: ", "req_id=", "requestId=", "job_id="] {
            if let Some(idx) = text.find(prefix) {
                let slice = &text[idx + prefix.len()..];
                let token = slice.split(|c: char| c.is_whitespace() || c == ',' || c == '"' || c == ';' || c == ']' || c == '}').next().unwrap_or("");
                if !token.is_empty() {
                    trace_id = Some(token.trim().to_string());
                    break;
                }
            }
        }
    }

    if span_id.is_none() {
        for prefix in &["span_id=", "spanId=", "span_id: ", "spanId: "] {
            if let Some(idx) = text.find(prefix) {
                let slice = &text[idx + prefix.len()..];
                let token = slice.split(|c: char| c.is_whitespace() || c == ',' || c == '"' || c == ';' || c == ']' || c == '}').next().unwrap_or("");
                if !token.is_empty() {
                    span_id = Some(token.trim().to_string());
                    break;
                }
            }
        }
    }

    if service_name.is_none() {
        for prefix in &["service=", "service_name=", "app="] {
            if let Some(idx) = text.find(prefix) {
                let slice = &text[idx + prefix.len()..];
                let token = slice.split(|c: char| c.is_whitespace() || c == ',' || c == '"' || c == ']').next().unwrap_or("");
                if !token.is_empty() {
                    service_name = Some(token.trim().to_string());
                    break;
                }
            }
        }
    }

    (trace_id, span_id, service_name)
}

/// Correlate log records matching a given trace ID into an OpenTelemetry waterfall
pub fn correlate_trace_waterfall(
    records: &[NormalizedLogRecord],
    target_trace_id: &str,
    default_source_file: &str,
) -> Option<TraceWaterfallReport> {
    let clean_target = target_trace_id.trim();
    if clean_target.is_empty() {
        return None;
    }

    let mut matching_spans = Vec::new();
    let mut min_epoch = i64::MAX;
    let mut max_epoch = i64::MIN;

    for r in records {
        let (extracted_trace, span_id, service_opt) = extract_trace_metadata(&r.message);
        let matches = match extracted_trace {
            Some(ref tid) => tid.eq_ignore_ascii_case(clean_target),
            None => r.message.contains(clean_target),
        };

        if matches {
            let ts = r.timestamp_epoch_ms.unwrap_or(0);
            if ts > 0 && ts < min_epoch {
                min_epoch = ts;
            }
            if ts > max_epoch {
                max_epoch = ts;
            }

            let service = service_opt
                .or_else(|| r.source.clone())
                .unwrap_or_else(|| "default-service".to_string());

            matching_spans.push((
                ts,
                TraceWaterfallSpan {
                    span_id,
                    parent_id: None,
                    service_name: service,
                    line_number: r.line_number,
                    timestamp_epoch_ms: ts,
                    offset_ms: 0,
                    duration_ms: 5, // minimum 5ms span width for UI rendering
                    severity: format!("{:?}", r.severity).to_uppercase(),
                    message: r.message.clone(),
                    source_file: default_source_file.to_string(),
                },
            ));
        }
    }

    if matching_spans.is_empty() {
        return None;
    }

    // Sort spans chronologically
    matching_spans.sort_by_key(|(ts, _)| *ts);

    let t0 = if min_epoch != i64::MAX { min_epoch } else { matching_spans[0].0 };
    let tn = if max_epoch != i64::MIN { max_epoch } else { matching_spans.last().unwrap().0 };
    let total_duration_ms = (tn - t0).max(10) as u64;

    let root_service = matching_spans[0].1.service_name.clone();

    let mut spans = Vec::with_capacity(matching_spans.len());
    for i in 0..matching_spans.len() {
        let (ts, mut span) = matching_spans[i].clone();
        span.offset_ms = (ts - t0).max(0);

        // Approximate duration from delta to next span if available
        if i + 1 < matching_spans.len() {
            let next_ts = matching_spans[i + 1].0;
            let delta = (next_ts - ts).max(5) as u64;
            span.duration_ms = delta.min(total_duration_ms);
        } else {
            span.duration_ms = 10;
        }

        spans.push(span);
    }

    Some(TraceWaterfallReport {
        trace_id: clean_target.to_string(),
        total_spans: spans.len(),
        start_time_epoch_ms: t0,
        end_time_epoch_ms: tn,
        total_duration_ms,
        root_service,
        spans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[test]
    fn test_merge_log_files_chronologically() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("service_a.log");
        let file2 = dir.path().join("service_b.log");
        let out = dir.path().join("merged.log");

        std::fs::write(
            &file1,
            "2026-10-19T10:00:01Z [INFO] Service A initialized\n2026-10-19T10:00:05Z [ERROR] Service A timeout\n",
        )
        .unwrap();

        std::fs::write(
            &file2,
            "2026-10-19T10:00:02Z [INFO] Service B connection ok\n2026-10-19T10:00:03Z [WARN] Service B retrying\n",
        )
        .unwrap();

        let summary = merge_log_files_chronologically(&[&file1, &file2], &out, 1000).unwrap();
        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.total_lines, 4);

        let merged_content = std::fs::read_to_string(&out).unwrap();
        let lines: Vec<&str> = merged_content.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("Service A initialized"));
        assert!(lines[1].contains("Service B connection ok"));
        assert!(lines[2].contains("Service B retrying"));
        assert!(lines[3].contains("Service A timeout"));
    }

    #[test]
    fn test_extract_trace_metadata() {
        let json_line = r#"{"trace_id": "trace-9988", "span_id": "span-12", "service": "payment-api", "msg": "charge ok"}"#;
        let (t, s, svc) = extract_trace_metadata(json_line);
        assert_eq!(t.as_deref(), Some("trace-9988"));
        assert_eq!(s.as_deref(), Some("span-12"));
        assert_eq!(svc.as_deref(), Some("payment-api"));

        let logfmt_line = "2026-10-19T10:00:00Z [INFO] traceId=trace-abc456 spanId=span-001 service=auth-svc login";
        let (t2, s2, svc2) = extract_trace_metadata(logfmt_line);
        assert_eq!(t2.as_deref(), Some("trace-abc456"));
        assert_eq!(s2.as_deref(), Some("span-001"));
        assert_eq!(svc2.as_deref(), Some("auth-svc"));
    }

    #[test]
    fn test_correlate_trace_waterfall() {
        let records = vec![
            NormalizedLogRecord {
                line_number: 1,
                raw: "trace_id=req-xyz service=api Received request".to_string(),
                timestamp_raw: Some("2026-10-19T10:00:00Z".to_string()),
                timestamp_epoch_ms: Some(1700000000000),
                severity: crate::parsers::LogSeverity::Info,
                message: "trace_id=req-xyz service=api Received request".to_string(),
                source: Some("api".to_string()),
                attributes: BTreeMap::new(),
            },
            NormalizedLogRecord {
                line_number: 2,
                raw: "trace_id=req-xyz service=db Query executed in 45ms".to_string(),
                timestamp_raw: Some("2026-10-19T10:00:00.050Z".to_string()),
                timestamp_epoch_ms: Some(1700000000050),
                severity: crate::parsers::LogSeverity::Info,
                message: "trace_id=req-xyz service=db Query executed in 45ms".to_string(),
                source: Some("db".to_string()),
                attributes: BTreeMap::new(),
            },
        ];

        let report = correlate_trace_waterfall(&records, "req-xyz", "cluster.log").unwrap();
        assert_eq!(report.trace_id, "req-xyz");
        assert_eq!(report.total_spans, 2);
        assert_eq!(report.spans[0].offset_ms, 0);
        assert_eq!(report.spans[1].offset_ms, 50);
        assert_eq!(report.total_duration_ms, 50);
    }
}
