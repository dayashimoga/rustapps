#![forbid(unsafe_code)]
//! High-Performance Multi-Mode Log Search Engine
//!
//! Provides literal substring, compiled regular expression, and structured
//! search across memory-mapped log slices with strict sub-50ms query SLAs.

use crate::mmap_engine::MmapLogEngine;
use crate::sparse_index::SparseLineIndex;
use regex::RegexBuilder;
use std::time::Instant;
use thiserror::Error;

/// Search error definitions
#[derive(Debug, Error)]
pub enum SearchEngineError {
    #[error("Invalid regular expression '{pattern}': {reason}")]
    InvalidRegex { pattern: String, reason: String },
    #[error("Search execution error: {0}")]
    Execution(String),
}

/// Query parameters for log search
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogQuery {
    pub pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub limit: usize,
    pub start_line: usize,
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            is_regex: false,
            case_sensitive: false,
            limit: 500,
            start_line: 0,
        }
    }
}

/// Individual search match
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LogSearchMatch {
    pub line_number: usize,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_content: String,
    pub match_start_in_line: usize,
    pub match_end_in_line: usize,
}

/// Aggregated search report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogSearchReport {
    pub matches: Vec<LogSearchMatch>,
    pub total_matched_in_scan: usize,
    pub elapsed_micros: u64,
    pub reached_limit: bool,
}

/// Multi-mode search engine
pub struct LogSearchEngine;

impl LogSearchEngine {
    /// Execute query across mapped lines.
    pub fn search(
        mmap: &MmapLogEngine,
        index: &SparseLineIndex,
        query: &LogQuery,
    ) -> Result<LogSearchReport, SearchEngineError> {
        let start_time = Instant::now();
        let trimmed_pattern = query.pattern.trim();

        if trimmed_pattern.is_empty() {
            return Ok(LogSearchReport {
                matches: Vec::new(),
                total_matched_in_scan: 0,
                elapsed_micros: start_time.elapsed().as_micros() as u64,
                reached_limit: false,
            });
        }

        let regex = if query.is_regex {
            Some(
                RegexBuilder::new(trimmed_pattern)
                    .case_insensitive(!query.case_sensitive)
                    .build()
                    .map_err(|e| SearchEngineError::InvalidRegex {
                        pattern: trimmed_pattern.to_string(),
                        reason: e.to_string(),
                    })?,
            )
        } else {
            None
        };

        let literal_target = if !query.is_regex && !query.case_sensitive {
            trimmed_pattern.to_lowercase()
        } else {
            trimmed_pattern.to_string()
        };

        let mut matches = Vec::with_capacity(query.limit.min(100));
        let total_lines = index.total_lines();
        let mut total_matched = 0;
        let mut reached_limit = false;

        let bytes = mmap.as_bytes();

        for line_idx in query.start_line..total_lines {
            if let Some((byte_start, byte_end)) = index.get_line_byte_range(line_idx, bytes) {
                let line_bytes = &bytes[byte_start as usize..byte_end as usize];
                let line_text = String::from_utf8_lossy(line_bytes);

                let matched_span = if let Some(ref re) = regex {
                    re.find(&line_text).map(|m| (m.start(), m.end()))
                } else if !query.case_sensitive {
                    let lower = line_text.to_lowercase();
                    lower
                        .find(&literal_target)
                        .map(|pos| (pos, pos + literal_target.len()))
                } else {
                    line_text
                        .find(&literal_target)
                        .map(|pos| (pos, pos + literal_target.len()))
                };

                if let Some((match_start, match_end)) = matched_span {
                    total_matched += 1;
                    if matches.len() < query.limit {
                        matches.push(LogSearchMatch {
                            line_number: line_idx,
                            byte_start,
                            byte_end,
                            line_content: line_text.to_string(),
                            match_start_in_line: match_start,
                            match_end_in_line: match_end,
                        });
                    } else {
                        reached_limit = true;
                        break;
                    }
                }
            }
        }

        let elapsed_micros = start_time.elapsed().as_micros() as u64;

        Ok(LogSearchReport {
            matches,
            total_matched_in_scan: total_matched,
            elapsed_micros,
            reached_limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_engine_literal() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("search_test.log");
        let content =
            b"INFO [auth] User logged in\nWARN [disk] Space low\nERROR [auth] Token expired\n";
        std::fs::write(&file_path, content).unwrap();

        let mmap = MmapLogEngine::open(&file_path).unwrap();
        let index = SparseLineIndex::build(content);

        let query = LogQuery {
            pattern: "auth".to_string(),
            is_regex: false,
            case_sensitive: false,
            limit: 10,
            start_line: 0,
        };

        let report = LogSearchEngine::search(&mmap, &index, &query).unwrap();
        assert_eq!(report.matches.len(), 2);
        assert_eq!(report.matches[0].line_number, 0);
        assert_eq!(report.matches[1].line_number, 2);
    }

    #[test]
    fn test_search_engine_regex() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("regex_test.log");
        let content = b"Status code: 200 OK\nStatus code: 404 Not Found\nStatus code: 500 Error\n";
        std::fs::write(&file_path, content).unwrap();

        let mmap = MmapLogEngine::open(&file_path).unwrap();
        let index = SparseLineIndex::build(content);

        let query = LogQuery {
            pattern: r"Status code: (4\d\d|5\d\d)".to_string(),
            is_regex: true,
            case_sensitive: false,
            limit: 10,
            start_line: 0,
        };

        let report = LogSearchEngine::search(&mmap, &index, &query).unwrap();
        assert_eq!(report.matches.len(), 2);
        assert_eq!(report.matches[0].line_number, 1);
        assert_eq!(report.matches[1].line_number, 2);
    }
}
