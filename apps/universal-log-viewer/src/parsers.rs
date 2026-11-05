#![forbid(unsafe_code)]
//! High-Throughput Streaming Log Parsers
//!
//! Provides zero-copy and streaming deserializers for JSON Lines, CSV/TSV,
//! and raw log formats with automatic format detection and normalized representations.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Supported log format categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    Auto,
    JsonLines,
    Csv,
    Tsv,
    CloudWatch,
    Kubernetes,
    RawText,
}

/// Normalized log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogSeverity {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
    Unknown = 6,
}

impl LogSeverity {
    /// Parse severity from string representations (case-insensitive)
    pub fn parse_str(s: &str) -> Self {
        let trimmed = s.trim().to_ascii_uppercase();
        if let Some(sev) = Self::from_word(&trimmed) {
            return sev;
        }

        // If the entire input is a numeric syslog priority (0..7)
        if let Ok(code) = trimmed.parse::<u8>() {
            return match code {
                0..=2 => LogSeverity::Fatal,
                3 => LogSeverity::Error,
                4 => LogSeverity::Warn,
                5 | 6 => LogSeverity::Info,
                7 => LogSeverity::Debug,
                _ => LogSeverity::Unknown,
            };
        }

        // Search tokens inside brackets, colons, or separated by non-alphanumeric delimiters
        for token in trimmed.split(|c: char| !c.is_alphanumeric()) {
            if !token.is_empty() {
                if let Some(sev) = Self::from_word(token) {
                    return sev;
                }
            }
        }

        LogSeverity::Unknown
    }

    fn from_word(token: &str) -> Option<Self> {
        match token {
            "TRACE" | "TRC" | "FINER" | "FINEST" => Some(LogSeverity::Trace),
            "DEBUG" | "DBG" | "FINE" => Some(LogSeverity::Debug),
            "INFO" | "INF" | "NOTICE" | "INFORMATION" => Some(LogSeverity::Info),
            "WARN" | "WARNING" | "WRN" => Some(LogSeverity::Warn),
            "ERROR" | "ERR" | "ERRO" | "SEVERE" => Some(LogSeverity::Error),
            "FATAL" | "CRIT" | "CRITICAL" | "EMERG" | "EMERGENCY" | "ALERT" => {
                Some(LogSeverity::Fatal)
            }
            _ => None,
        }
    }

    /// Return canonical uppercase name
    pub fn as_str(&self) -> &'static str {
        match self {
            LogSeverity::Trace => "TRACE",
            LogSeverity::Debug => "DEBUG",
            LogSeverity::Info => "INFO",
            LogSeverity::Warn => "WARN",
            LogSeverity::Error => "ERROR",
            LogSeverity::Fatal => "FATAL",
            LogSeverity::Unknown => "UNKNOWN",
        }
    }
}

/// Normalized log record across all structured and unstructured formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLogRecord {
    pub line_number: usize,
    pub raw: String,
    pub timestamp_raw: Option<String>,
    pub timestamp_epoch_ms: Option<i64>,
    pub severity: LogSeverity,
    pub message: String,
    pub source: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

/// Fast ISO-8601 / RFC-3339 and epoch timestamp parser (offline, zero-alloc)
pub fn parse_timestamp(s: &str) -> Option<i64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try epoch milliseconds or seconds
    if let Ok(epoch) = trimmed.parse::<i64>() {
        // If 10 digits, assume seconds -> convert to ms
        if epoch > 1_000_000_000 && epoch < 100_000_000_000 {
            return Some(epoch * 1000);
        }
        // If 13 digits, assume milliseconds
        if (100_000_000_000..100_000_000_000_000).contains(&epoch) {
            return Some(epoch);
        }
        // If 16 digits (micro) or 19 digits (nano)
        if (100_000_000_000_000..100_000_000_000_000_000).contains(&epoch) {
            return Some(epoch / 1000);
        }
        if epoch >= 100_000_000_000_000_000 {
            return Some(epoch / 1_000_000);
        }
    }

    // Parse ISO-8601: YYYY-MM-DD[T ]HH:MM:SS[.fraction][Z|(+|-)HH:MM]
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 19
        && (bytes[4] == b'-' || bytes[4] == b'/')
        && (bytes[7] == b'-' || bytes[7] == b'/')
    {
        let year: i64 = std::str::from_utf8(&bytes[0..4]).ok()?.parse().ok()?;
        let month: u32 = std::str::from_utf8(&bytes[5..7]).ok()?.parse().ok()?;
        let day: u32 = std::str::from_utf8(&bytes[8..10]).ok()?.parse().ok()?;
        let hour: u32 = std::str::from_utf8(&bytes[11..13]).ok()?.parse().ok()?;
        let min: u32 = std::str::from_utf8(&bytes[14..16]).ok()?.parse().ok()?;
        let sec: u32 = std::str::from_utf8(&bytes[17..19]).ok()?.parse().ok()?;

        let mut millis: u32 = 0;
        let mut idx = 19;
        if idx < bytes.len() && (bytes[idx] == b'.' || bytes[idx] == b',') {
            idx += 1;
            let start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            let frac_str = std::str::from_utf8(&bytes[start..idx]).ok()?;
            let frac_val: u32 = frac_str.parse().ok()?;
            let len = frac_str.len();
            millis = if len >= 3 {
                frac_val / 10u32.pow((len - 3) as u32)
            } else {
                frac_val * 10u32.pow((3 - len) as u32)
            };
        }

        let days = days_from_civil(year, month, day);
        let total_seconds = days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64);
        let mut epoch_ms = total_seconds * 1000 + (millis as i64);

        // Parse timezone offset if present
        if idx < bytes.len() {
            let tz_bytes = &bytes[idx..];
            if tz_bytes.len() >= 5 && (tz_bytes[0] == b'+' || tz_bytes[0] == b'-') {
                let sign: i64 = if tz_bytes[0] == b'+' { -1 } else { 1 };
                let tz_hour: i64 = std::str::from_utf8(&tz_bytes[1..3]).ok()?.parse().ok()?;
                let tz_min: i64 = if tz_bytes.len() >= 5 && tz_bytes[3] == b':' {
                    std::str::from_utf8(&tz_bytes[4..6]).ok()?.parse().ok()?
                } else if tz_bytes.len() >= 5 {
                    std::str::from_utf8(&tz_bytes[3..5]).ok()?.parse().ok()?
                } else {
                    0
                };
                epoch_ms += sign * (tz_hour * 3600 + tz_min * 60) * 1000;
            }
        }

        return Some(epoch_ms);
    }

    None
}

/// Compute Unix epoch day count from Gregorian calendar (civil day calculation)
fn days_from_civil(mut y: i64, mut m: u32, d: u32) -> i64 {
    if m <= 2 {
        y -= 1;
        m += 12;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m as i64 - 3) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Format auto-detection by inspecting a sample of log lines
pub fn detect_format(sample_lines: &[&str]) -> LogFormat {
    if sample_lines.is_empty() {
        return LogFormat::RawText;
    }

    let mut json_valid = 0usize;
    let mut csv_valid = 0usize;
    let mut tsv_valid = 0usize;
    let mut k8s_valid = 0usize;
    let mut cw_valid = 0usize;

    for line in sample_lines.iter().take(20) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check JSON Lines
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            json_valid += 1;
            if trimmed.contains("\"logGroup\"") || trimmed.contains("\"logStream\"") {
                cw_valid += 1;
            }
        }

        // Check Kubernetes CRI format: <timestamp> (stdout|stderr) (F|P) <content>
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 4
            && (parts[1] == "stdout" || parts[1] == "stderr")
            && (parts[2] == "F" || parts[2] == "P")
            && parse_timestamp(parts[0]).is_some()
        {
            k8s_valid += 1;
        }

        // Check CSV/TSV
        let comma_count = trimmed.chars().filter(|&c| c == ',').count();
        if comma_count >= 2 {
            csv_valid += 1;
        }
        let tab_count = trimmed.chars().filter(|&c| c == '\t').count();
        if tab_count >= 2 {
            tsv_valid += 1;
        }
    }

    if k8s_valid >= sample_lines.len().min(3) {
        return LogFormat::Kubernetes;
    }
    if cw_valid >= sample_lines.len().min(3) {
        return LogFormat::CloudWatch;
    }
    if json_valid >= sample_lines.len().min(3) {
        return LogFormat::JsonLines;
    }
    if tsv_valid >= sample_lines.len().min(3) {
        return LogFormat::Tsv;
    }
    if csv_valid >= sample_lines.len().min(3) {
        return LogFormat::Csv;
    }

    LogFormat::RawText
}

/// Streaming parser for JSON Lines logs
pub struct StreamingJsonParser;

impl StreamingJsonParser {
    /// Parse a single line into a NormalizedLogRecord
    pub fn parse_line(line_number: usize, line: &str) -> NormalizedLogRecord {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return NormalizedLogRecord {
                line_number,
                raw: line.to_string(),
                timestamp_raw: None,
                timestamp_epoch_ms: None,
                severity: LogSeverity::Unknown,
                message: String::new(),
                source: None,
                attributes: BTreeMap::new(),
            };
        }

        if let Ok(serde_json::Value::Object(map)) =
            serde_json::from_str::<serde_json::Value>(trimmed)
        {
            let mut timestamp_raw = None;
            let mut timestamp_epoch_ms = None;
            let mut severity = LogSeverity::Unknown;
            let mut message = None;
            let mut source = None;
            let mut attributes = BTreeMap::new();

            for (k, v) in map {
                let k_lower = k.to_ascii_lowercase();

                // Match timestamp
                if timestamp_raw.is_none()
                    && (k_lower == "timestamp"
                        || k_lower == "@timestamp"
                        || k_lower == "time"
                        || k_lower == "ts"
                        || k_lower == "date")
                {
                    let ts_str = match &v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => v.to_string(),
                    };
                    timestamp_epoch_ms = parse_timestamp(&ts_str);
                    timestamp_raw = Some(ts_str);
                    continue;
                }

                // Match severity
                if severity == LogSeverity::Unknown
                    && (k_lower == "level"
                        || k_lower == "severity"
                        || k_lower == "lvl"
                        || k_lower == "log_level"
                        || k_lower == "status")
                {
                    let sev_str = match &v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => v.to_string(),
                    };
                    severity = LogSeverity::parse_str(&sev_str);
                    continue;
                }

                // Match message
                if message.is_none()
                    && (k_lower == "message"
                        || k_lower == "msg"
                        || k_lower == "log"
                        || k_lower == "payload"
                        || k_lower == "text")
                {
                    message = match v {
                        serde_json::Value::String(s) => Some(s),
                        _ => Some(v.to_string()),
                    };
                    continue;
                }

                // Match source / logger / service
                if source.is_none()
                    && (k_lower == "source"
                        || k_lower == "logger"
                        || k_lower == "service"
                        || k_lower == "component"
                        || k_lower == "app")
                {
                    source = match v {
                        serde_json::Value::String(s) => Some(s),
                        _ => Some(v.to_string()),
                    };
                    continue;
                }

                // Store remaining attributes
                let val_str = match v {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Null => "null".to_string(),
                    _ => v.to_string(),
                };
                attributes.insert(k, val_str);
            }

            return NormalizedLogRecord {
                line_number,
                raw: line.to_string(),
                timestamp_raw,
                timestamp_epoch_ms,
                severity,
                message: message.unwrap_or_else(|| trimmed.to_string()),
                source,
                attributes,
            };
        }

        // Fallback for non-object JSON or parse error
        NormalizedLogRecord {
            line_number,
            raw: line.to_string(),
            timestamp_raw: None,
            timestamp_epoch_ms: None,
            severity: LogSeverity::Unknown,
            message: line.to_string(),
            source: None,
            attributes: BTreeMap::new(),
        }
    }
}

/// Streaming parser for CSV / TSV logs
pub struct StreamingCsvParser {
    delimiter: u8,
    headers: Option<Vec<String>>,
    ts_col: Option<usize>,
    sev_col: Option<usize>,
    msg_col: Option<usize>,
}

impl StreamingCsvParser {
    /// Create a new CSV parser with custom delimiter
    pub fn new(delimiter: u8) -> Self {
        Self {
            delimiter,
            headers: None,
            ts_col: None,
            sev_col: None,
            msg_col: None,
        }
    }

    /// Initialize parser with a header line
    pub fn with_header_line(&mut self, header_line: &str) {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(self.delimiter)
            .from_reader(header_line.as_bytes());

        if let Some(Ok(record)) = rdr.records().next() {
            let headers: Vec<String> = record.iter().map(|s| s.trim().to_string()).collect();
            for (i, h) in headers.iter().enumerate() {
                let h_lower = h.to_ascii_lowercase();
                if self.ts_col.is_none()
                    && (h_lower.contains("time") || h_lower.contains("date") || h_lower == "ts")
                {
                    self.ts_col = Some(i);
                }
                if self.sev_col.is_none()
                    && (h_lower.contains("level") || h_lower.contains("sev") || h_lower == "status")
                {
                    self.sev_col = Some(i);
                }
                if self.msg_col.is_none()
                    && (h_lower.contains("msg") || h_lower.contains("message") || h_lower == "log")
                {
                    self.msg_col = Some(i);
                }
            }
            self.headers = Some(headers);
        }
    }

    /// Parse a single row into a NormalizedLogRecord
    pub fn parse_row(&self, line_number: usize, row_str: &str) -> NormalizedLogRecord {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(self.delimiter)
            .flexible(true)
            .from_reader(row_str.as_bytes());

        if let Some(Ok(record)) = rdr.records().next() {
            let mut timestamp_raw = None;
            let mut timestamp_epoch_ms = None;
            let mut severity = LogSeverity::Unknown;
            let mut message = None;
            let mut attributes = BTreeMap::new();

            for (i, field) in record.iter().enumerate() {
                let field_str = field.trim();
                let header_name = self
                    .headers
                    .as_ref()
                    .and_then(|h| h.get(i).cloned())
                    .unwrap_or_else(|| format!("col_{}", i));

                if Some(i) == self.ts_col {
                    timestamp_epoch_ms = parse_timestamp(field_str);
                    timestamp_raw = Some(field_str.to_string());
                } else if Some(i) == self.sev_col {
                    severity = LogSeverity::parse_str(field_str);
                } else if Some(i) == self.msg_col {
                    message = Some(field_str.to_string());
                } else {
                    attributes.insert(header_name, field_str.to_string());
                }
            }

            let final_msg = message.unwrap_or_else(|| {
                // If no specific message column, use the whole row or last column
                record.iter().next_back().unwrap_or(row_str).to_string()
            });

            return NormalizedLogRecord {
                line_number,
                raw: row_str.to_string(),
                timestamp_raw,
                timestamp_epoch_ms,
                severity,
                message: final_msg,
                source: None,
                attributes,
            };
        }

        NormalizedLogRecord {
            line_number,
            raw: row_str.to_string(),
            timestamp_raw: None,
            timestamp_epoch_ms: None,
            severity: LogSeverity::Unknown,
            message: row_str.to_string(),
            source: None,
            attributes: BTreeMap::new(),
        }
    }
}

/// Individual field frequency value count
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldValueCount {
    pub value: String,
    pub count: usize,
    pub percentage: f64,
}

/// Aggregated breakdown for a specific structured field key
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldBreakdown {
    pub field_name: String,
    pub total_occurrences: usize,
    pub distinct_values_count: usize,
    pub top_values: Vec<FieldValueCount>,
}

/// Comprehensive Top-N field frequency distribution report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAggregationReport {
    pub total_records_inspected: usize,
    pub fields: Vec<FieldBreakdown>,
}

/// Compute frequency distribution of structured fields across records
pub fn compute_field_aggregations(
    records: &[NormalizedLogRecord],
    top_n_per_field: usize,
) -> FieldAggregationReport {
    let mut field_tallies: HashMap<String, HashMap<String, usize>> = HashMap::new();

    for r in records {
        for (k, v) in &r.attributes {
            if !k.is_empty() && !v.is_empty() {
                *field_tallies
                    .entry(k.clone())
                    .or_default()
                    .entry(v.clone())
                    .or_insert(0) += 1;
            }
        }

        let sev_str = r.severity.as_str().to_string();
        if sev_str != "UNKNOWN" {
            *field_tallies
                .entry("severity".to_string())
                .or_default()
                .entry(sev_str)
                .or_insert(0) += 1;
        }

        if let Some(ref src) = r.source {
            if !src.is_empty() {
                *field_tallies
                    .entry("service".to_string())
                    .or_default()
                    .entry(src.clone())
                    .or_insert(0) += 1;
            }
        }

        if r.message.starts_with('{') && r.message.ends_with('}') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&r.message) {
                if let Some(obj) = val.as_object() {
                    for (k, v) in obj {
                        if k == "timestamp"
                            || k == "time"
                            || k == "raw"
                            || k == "message"
                            || k == "msg"
                        {
                            continue;
                        }
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => continue,
                        };
                        if !val_str.is_empty() && val_str.len() < 120 {
                            *field_tallies
                                .entry(k.clone())
                                .or_default()
                                .entry(val_str)
                                .or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    let mut breakdowns = Vec::new();

    for (field_name, val_map) in field_tallies {
        let total_occurrences: usize = val_map.values().sum();
        let distinct_values_count = val_map.len();

        let mut sorted_values: Vec<(String, usize)> = val_map.into_iter().collect();
        sorted_values.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let top_values: Vec<FieldValueCount> = sorted_values
            .into_iter()
            .take(top_n_per_field)
            .map(|(value, count)| {
                let percentage = if total_occurrences > 0 {
                    (count as f64 / total_occurrences as f64) * 100.0
                } else {
                    0.0
                };
                FieldValueCount {
                    value,
                    count,
                    percentage: (percentage * 10.0).round() / 10.0,
                }
            })
            .collect();

        breakdowns.push(FieldBreakdown {
            field_name,
            total_occurrences,
            distinct_values_count,
            top_values,
        });
    }

    breakdowns.sort_by(|a, b| {
        b.total_occurrences
            .cmp(&a.total_occurrences)
            .then_with(|| a.field_name.cmp(&b.field_name))
    });

    FieldAggregationReport {
        total_records_inspected: records.len(),
        fields: breakdowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp_iso8601() {
        let ts = "2026-10-19T10:15:30.500Z";
        let parsed = parse_timestamp(ts);
        assert!(parsed.is_some());
        let epoch = parsed.unwrap();
        assert!(epoch > 1_700_000_000_000);
    }

    #[test]
    fn test_parse_timestamp_epoch() {
        let ts_sec = "1700000000";
        let ts_ms = "1700000000000";
        assert_eq!(parse_timestamp(ts_sec), Some(1_700_000_000_000));
        assert_eq!(parse_timestamp(ts_ms), Some(1_700_000_000_000));
    }

    #[test]
    fn test_parse_severity_variants() {
        assert_eq!(LogSeverity::parse_str("info"), LogSeverity::Info);
        assert_eq!(LogSeverity::parse_str("WARN"), LogSeverity::Warn);
        assert_eq!(LogSeverity::parse_str("error"), LogSeverity::Error);
        assert_eq!(LogSeverity::parse_str("CRIT"), LogSeverity::Fatal);
        assert_eq!(LogSeverity::parse_str("unknown_val"), LogSeverity::Unknown);
    }

    #[test]
    fn test_detect_format_json() {
        let lines = vec![
            r#"{"timestamp":"2026-10-19T10:00:00Z","level":"info","message":"started"}"#,
            r#"{"timestamp":"2026-10-19T10:00:01Z","level":"error","message":"failed"}"#,
        ];
        assert_eq!(detect_format(&lines), LogFormat::JsonLines);
    }

    #[test]
    fn test_streaming_json_parser() {
        let json_line = r#"{"time":"2026-10-19T12:00:00Z","level":"WARN","msg":"disk 90% full","host":"srv-1","cluster":"prod"}"#;
        let record = StreamingJsonParser::parse_line(42, json_line);
        assert_eq!(record.line_number, 42);
        assert_eq!(record.severity, LogSeverity::Warn);
        assert_eq!(record.message, "disk 90% full");
        assert_eq!(
            record.attributes.get("host").map(String::as_str),
            Some("srv-1")
        );
        assert_eq!(
            record.attributes.get("cluster").map(String::as_str),
            Some("prod")
        );
        assert!(record.timestamp_epoch_ms.is_some());
    }

    #[test]
    fn test_streaming_csv_parser() {
        let mut csv_parser = StreamingCsvParser::new(b',');
        csv_parser.with_header_line("timestamp,level,message,ip");
        let row = "2026-10-19T14:30:00Z,ERROR,Connection timeout,192.168.1.100";
        let record = csv_parser.parse_row(10, row);
        assert_eq!(record.line_number, 10);
        assert_eq!(record.severity, LogSeverity::Error);
        assert_eq!(record.message, "Connection timeout");
        assert_eq!(
            record.attributes.get("ip").map(String::as_str),
            Some("192.168.1.100")
        );
    }

    #[test]
    fn test_compute_field_aggregations() {
        let mut rec1 = NormalizedLogRecord {
            line_number: 1,
            raw: String::new(),
            timestamp_raw: None,
            timestamp_epoch_ms: None,
            severity: LogSeverity::Error,
            message: r#"{"status": 500, "service": "auth", "client_ip": "10.0.0.1"}"#.to_string(),
            source: Some("auth".to_string()),
            attributes: BTreeMap::new(),
        };
        rec1.attributes
            .insert("cluster".to_string(), "prod-us-east".to_string());

        let mut rec2 = NormalizedLogRecord {
            line_number: 2,
            raw: String::new(),
            timestamp_raw: None,
            timestamp_epoch_ms: None,
            severity: LogSeverity::Info,
            message: r#"{"status": 200, "service": "auth", "client_ip": "10.0.0.2"}"#.to_string(),
            source: Some("auth".to_string()),
            attributes: BTreeMap::new(),
        };
        rec2.attributes
            .insert("cluster".to_string(), "prod-us-east".to_string());

        let report = compute_field_aggregations(&[rec1, rec2], 5);
        assert_eq!(report.total_records_inspected, 2);

        let cluster_field = report
            .fields
            .iter()
            .find(|f| f.field_name == "cluster")
            .unwrap();
        assert_eq!(cluster_field.total_occurrences, 2);
        assert_eq!(cluster_field.top_values[0].value, "prod-us-east");
        assert_eq!(cluster_field.top_values[0].percentage, 100.0);

        let sev_field = report
            .fields
            .iter()
            .find(|f| f.field_name == "severity")
            .unwrap();
        assert_eq!(sev_field.total_occurrences, 2);
        assert_eq!(sev_field.distinct_values_count, 2);
    }
}
