#![forbid(unsafe_code)]
//! Chronological Timeline Index and Volume/Severity Histograms
//!
//! Provides time-bucket aggregations, severity distributions, and sub-50ms
//! binary search time-window filtering across millions of log entries.

use crate::parsers::{LogSeverity, NormalizedLogRecord};
use serde::{Deserialize, Serialize};

/// A single time bucket in the histogram
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub bucket_index: usize,
    pub start_epoch_ms: i64,
    pub end_epoch_ms: i64,
    pub total_count: usize,
    pub trace_count: usize,
    pub debug_count: usize,
    pub info_count: usize,
    pub warn_count: usize,
    pub error_count: usize,
    pub fatal_count: usize,
    pub volume_per_second: f64,
}

/// Complete aggregated timeline histogram response for desktop UI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineHistogram {
    pub min_epoch_ms: i64,
    pub max_epoch_ms: i64,
    pub total_records: usize,
    pub bucket_width_ms: i64,
    pub buckets: Vec<TimelineBucket>,
}

/// Chronological index entry mapping timestamp to line number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeIndexEntry {
    pub epoch_ms: i64,
    pub line_number: usize,
    pub severity: LogSeverity,
}

/// High-performance chronological timeline index
#[derive(Debug, Default, Clone)]
pub struct TimelineIndex {
    entries: Vec<TimeIndexEntry>,
}

impl TimelineIndex {
    /// Create a new empty timeline index
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Build timeline index from normalized records
    pub fn from_records(records: &[NormalizedLogRecord]) -> Self {
        let mut entries = Vec::with_capacity(records.len());
        for r in records {
            if let Some(ts) = r.timestamp_epoch_ms {
                entries.push(TimeIndexEntry {
                    epoch_ms: ts,
                    line_number: r.line_number,
                    severity: r.severity,
                });
            }
        }
        // Sort chronologically
        entries.sort_unstable_by_key(|e| e.epoch_ms);
        Self { entries }
    }

    /// Push an entry incrementally
    pub fn push(&mut self, epoch_ms: i64, line_number: usize, severity: LogSeverity) {
        self.entries.push(TimeIndexEntry {
            epoch_ms,
            line_number,
            severity,
        });
    }

    /// Sort entries if pushed out-of-order
    pub fn sort(&mut self) {
        self.entries.sort_unstable_by_key(|e| e.epoch_ms);
    }

    /// Total indexed records with timestamps
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Filter line numbers by time range [start_ms, end_ms] in sub-50ms via binary search
    pub fn filter_by_timerange(&self, start_ms: i64, end_ms: i64) -> Vec<usize> {
        if self.entries.is_empty() || start_ms > end_ms {
            return Vec::new();
        }

        let start_idx = match self.entries.binary_search_by_key(&start_ms, |e| e.epoch_ms) {
            Ok(idx) => {
                // Rewind to first entry with identical timestamp
                let mut first = idx;
                while first > 0 && self.entries[first - 1].epoch_ms == start_ms {
                    first -= 1;
                }
                first
            }
            Err(idx) => idx,
        };

        let end_idx = match self.entries.binary_search_by_key(&end_ms, |e| e.epoch_ms) {
            Ok(idx) => {
                // Forward to last entry with identical timestamp
                let mut last = idx;
                while last + 1 < self.entries.len() && self.entries[last + 1].epoch_ms == end_ms {
                    last += 1;
                }
                last + 1
            }
            Err(idx) => idx,
        };

        if start_idx >= self.entries.len() || start_idx >= end_idx {
            return Vec::new();
        }

        self.entries[start_idx..end_idx]
            .iter()
            .map(|e| e.line_number)
            .collect()
    }

    /// Generate aggregated timeline histogram with target bucket count
    pub fn generate_histogram(&self, target_buckets: usize) -> Option<TimelineHistogram> {
        if self.entries.is_empty() {
            return None;
        }

        let min_epoch_ms = self.entries.first().unwrap().epoch_ms;
        let max_epoch_ms = self.entries.last().unwrap().epoch_ms;

        let span_ms = (max_epoch_ms - min_epoch_ms).max(1);
        let num_buckets = target_buckets.clamp(1, 200).min(self.entries.len());
        let bucket_width_ms = ((span_ms as f64) / (num_buckets as f64)).ceil() as i64;

        let mut buckets = Vec::with_capacity(num_buckets);
        for i in 0..num_buckets {
            let b_start = min_epoch_ms + (i as i64) * bucket_width_ms;
            let b_end = if i == num_buckets - 1 {
                max_epoch_ms + 1
            } else {
                b_start + bucket_width_ms
            };

            let duration_sec = ((b_end - b_start) as f64) / 1000.0;
            buckets.push(TimelineBucket {
                bucket_index: i,
                start_epoch_ms: b_start,
                end_epoch_ms: b_end,
                total_count: 0,
                trace_count: 0,
                debug_count: 0,
                info_count: 0,
                warn_count: 0,
                error_count: 0,
                fatal_count: 0,
                volume_per_second: duration_sec,
            });
        }

        for entry in &self.entries {
            let offset = entry.epoch_ms - min_epoch_ms;
            let mut b_idx = (offset / bucket_width_ms) as usize;
            if b_idx >= num_buckets {
                b_idx = num_buckets - 1;
            }

            let b = &mut buckets[b_idx];
            b.total_count += 1;
            match entry.severity {
                LogSeverity::Trace => b.trace_count += 1,
                LogSeverity::Debug => b.debug_count += 1,
                LogSeverity::Info => b.info_count += 1,
                LogSeverity::Warn => b.warn_count += 1,
                LogSeverity::Error => b.error_count += 1,
                LogSeverity::Fatal => b.fatal_count += 1,
                LogSeverity::Unknown => {}
            }
        }

        for b in &mut buckets {
            let duration_sec = b.volume_per_second;
            b.volume_per_second = if duration_sec > 0.0 {
                (b.total_count as f64) / duration_sec
            } else {
                0.0
            };
        }

        Some(TimelineHistogram {
            min_epoch_ms,
            max_epoch_ms,
            total_records: self.entries.len(),
            bucket_width_ms,
            buckets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_index_filter() {
        let mut index = TimelineIndex::new();
        index.push(1000, 1, LogSeverity::Info);
        index.push(2000, 2, LogSeverity::Warn);
        index.push(3000, 3, LogSeverity::Error);
        index.push(4000, 4, LogSeverity::Info);
        index.push(5000, 5, LogSeverity::Fatal);
        index.sort();

        let matched = index.filter_by_timerange(2000, 4000);
        assert_eq!(matched, vec![2, 3, 4]);

        let empty = index.filter_by_timerange(6000, 7000);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_timeline_histogram_generation() {
        let mut index = TimelineIndex::new();
        for i in 0..100 {
            let sev = if i % 10 == 0 {
                LogSeverity::Error
            } else if i % 5 == 0 {
                LogSeverity::Warn
            } else {
                LogSeverity::Info
            };
            index.push(1700000000000 + i * 1000, (i + 1) as usize, sev);
        }
        index.sort();

        let hist = index
            .generate_histogram(10)
            .expect("should generate histogram");
        assert_eq!(hist.buckets.len(), 10);
        assert_eq!(hist.total_records, 100);

        let total_counted: usize = hist.buckets.iter().map(|b| b.total_count).sum();
        assert_eq!(total_counted, 100);

        let error_counted: usize = hist.buckets.iter().map(|b| b.error_count).sum();
        assert_eq!(error_counted, 10);
    }
}
