#![forbid(unsafe_code)]
//! Lock-Free Sparse Line-Offset Index
//!
//! Implements ADR-0009: sparse checkpoint indexing (checkpoint every K=128 lines)
//! with SIMD newline scanning via `memchr`, maintaining sub-microsecond line lookups
//! and minimal RAM footprint even on 500GB log files.

use memchr::memchr_iter;

/// Default checkpoint interval (every 128 lines)
pub const DEFAULT_CHECKPOINT_INTERVAL: usize = 128;

/// Sparse line index representation
#[derive(Debug, Clone)]
pub struct SparseLineIndex {
    /// Byte offsets of line 0, line K, line 2K, etc.
    checkpoints: Vec<u64>,
    /// Total count of lines discovered in file
    total_lines: usize,
    /// Number of lines between checkpoints
    interval: usize,
}

impl SparseLineIndex {
    /// Build a new sparse line index by scanning the entire byte slice using SIMD.
    pub fn build(bytes: &[u8]) -> Self {
        Self::build_with_interval(bytes, DEFAULT_CHECKPOINT_INTERVAL)
    }

    /// Build a sparse index with a custom checkpoint interval.
    pub fn build_with_interval(bytes: &[u8], interval: usize) -> Self {
        if bytes.is_empty() {
            return Self {
                checkpoints: vec![0],
                total_lines: 0,
                interval,
            };
        }

        let mut checkpoints = Vec::with_capacity((bytes.len() / (interval * 50)).max(16));
        // Line 0 starts at byte 0
        checkpoints.push(0);

        let mut current_line = 0;

        for newline_pos in memchr_iter(b'\n', bytes) {
            current_line += 1;
            let next_line_start = (newline_pos + 1) as u64;

            if current_line % interval == 0 {
                checkpoints.push(next_line_start);
            }
        }

        // If the file does not end with a trailing newline, count the final partial line
        let total_lines = if bytes.last() == Some(&b'\n') {
            current_line
        } else {
            current_line + 1
        };

        Self {
            checkpoints,
            total_lines,
            interval,
        }
    }

    /// Incrementally index appended bytes without re-indexing existing history.
    pub fn append_bytes(&mut self, new_bytes: &[u8], base_offset: u64) {
        if new_bytes.is_empty() {
            return;
        }

        // If existing lines align with checkpoint boundary, base_offset is a checkpoint
        if self.total_lines % self.interval == 0
            && self.checkpoints.len() == (self.total_lines / self.interval)
        {
            self.checkpoints.push(base_offset);
        }

        for newline_pos in memchr_iter(b'\n', new_bytes) {
            self.total_lines += 1;
            let next_line_start = base_offset + (newline_pos + 1) as u64;

            if self.total_lines % self.interval == 0 {
                self.checkpoints.push(next_line_start);
            }
        }
    }

    /// Lookup the exact `(start_byte_offset, end_byte_offset)` for a 0-indexed line.
    pub fn get_line_byte_range(&self, line_idx: usize, bytes: &[u8]) -> Option<(u64, u64)> {
        if line_idx >= self.total_lines || bytes.is_empty() {
            return None;
        }

        let checkpoint_idx = line_idx / self.interval;
        if checkpoint_idx >= self.checkpoints.len() {
            return None;
        }

        let checkpoint_byte_offset = self.checkpoints[checkpoint_idx] as usize;
        let lines_to_skip = line_idx % self.interval;

        // Slice starting at checkpoint offset
        let slice = &bytes[checkpoint_byte_offset..];
        let mut line_start_rel = 0;

        if lines_to_skip > 0 {
            let mut skipped = 0;
            for pos in memchr_iter(b'\n', slice) {
                skipped += 1;
                if skipped == lines_to_skip {
                    line_start_rel = pos + 1;
                    break;
                }
            }
        }

        let line_start_abs = (checkpoint_byte_offset + line_start_rel) as u64;
        let line_slice = &bytes[line_start_abs as usize..];

        // Find next newline or end of slice
        let line_len = memchr::memchr(b'\n', line_slice).unwrap_or(line_slice.len());
        let mut line_end_abs = line_start_abs + line_len as u64;

        // Trim trailing \r if present
        if line_end_abs > line_start_abs && bytes[(line_end_abs - 1) as usize] == b'\r' {
            line_end_abs -= 1;
        }

        Some((line_start_abs, line_end_abs))
    }

    /// Total number of lines indexed.
    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    /// Checkpoint interval.
    pub fn interval(&self) -> usize {
        self.interval
    }

    /// Number of checkpoints retained in memory.
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_index_small_file() {
        let text = b"First line\nSecond line\nThird line\n";
        let index = SparseLineIndex::build_with_interval(text, 2);

        assert_eq!(index.total_lines(), 3);
        // Line 0
        let (s0, e0) = index.get_line_byte_range(0, text).unwrap();
        assert_eq!(&text[s0 as usize..e0 as usize], b"First line");

        // Line 1
        let (s1, e1) = index.get_line_byte_range(1, text).unwrap();
        assert_eq!(&text[s1 as usize..e1 as usize], b"Second line");

        // Line 2
        let (s2, e2) = index.get_line_byte_range(2, text).unwrap();
        assert_eq!(&text[s2 as usize..e2 as usize], b"Third line");

        // Out of range line
        assert!(index.get_line_byte_range(3, text).is_none());
    }

    #[test]
    fn test_sparse_index_no_trailing_newline() {
        let text = b"Line A\nLine B\nLine C";
        let index = SparseLineIndex::build_with_interval(text, 2);

        assert_eq!(index.total_lines(), 3);
        let (s2, e2) = index.get_line_byte_range(2, text).unwrap();
        assert_eq!(&text[s2 as usize..e2 as usize], b"Line C");
    }

    #[test]
    fn test_sparse_index_windows_crlf() {
        let text = b"Windows Line 1\r\nWindows Line 2\r\n";
        let index = SparseLineIndex::build(text);

        let (s0, e0) = index.get_line_byte_range(0, text).unwrap();
        assert_eq!(&text[s0 as usize..e0 as usize], b"Windows Line 1");
    }

    #[test]
    fn test_sparse_index_incremental_append() {
        let initial_text = b"Line 1\nLine 2\n";
        let mut index = SparseLineIndex::build_with_interval(initial_text, 2);
        assert_eq!(index.total_lines(), 2);

        let appended = b"Line 3\nLine 4\n";
        let mut full_bytes = initial_text.to_vec();
        full_bytes.extend_from_slice(appended);

        index.append_bytes(appended, initial_text.len() as u64);
        assert_eq!(index.total_lines(), 4);

        let (s3, e3) = index.get_line_byte_range(3, &full_bytes).unwrap();
        assert_eq!(&full_bytes[s3 as usize..e3 as usize], b"Line 4");
    }
}
