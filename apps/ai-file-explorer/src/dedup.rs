#![forbid(unsafe_code)]
//! Deterministic 3-Tier Multi-Stage Duplicate Detection Engine (ADR-0022)
//!
//! Minimizes disk I/O through a 3-tier filtration pipeline:
//! 1. Stage 1: Exact byte size grouping (zero file contents read)
//! 2. Stage 2: 4KB Header/Footer boundary hashing (discards >95% of non-duplicates)
//! 3. Stage 3: Full-file streaming BLAKE3 cryptographic verification (100% false-positive free)

use crate::models::{DeduplicationSummary, DuplicateGroup};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use walkdir::WalkDir;

const HEADER_FOOTER_SAMPLE_SIZE: usize = 4096;
const STREAM_BUFFER_SIZE: usize = 65536;

#[derive(Debug, Error)]
pub enum DedupError {
    #[error("I/O error during duplicate scan: {0}")]
    Io(#[from] std::io::Error),
    #[error("Target path not found: {0}")]
    NotFound(PathBuf),
}

/// Multi-stage duplicate file detector
pub struct DuplicateDetector;

impl DuplicateDetector {
    /// Scan target directories and identify all duplicate files using 3-tier filtration
    pub fn scan_duplicates<P: AsRef<Path>>(
        search_roots: &[P],
        min_size_bytes: u64,
    ) -> Result<DeduplicationSummary, DedupError> {
        let start_time = Instant::now();

        let mut total_scanned_files = 0usize;
        let mut total_scanned_bytes = 0u64;

        // Stage 1: Group files by exact byte size
        let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();

        for root in search_roots {
            let root_path = root.as_ref();
            if !root_path.exists() {
                continue;
            }

            for entry_res in WalkDir::new(root_path).follow_links(false) {
                let entry = match entry_res {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                if !entry.file_type().is_file() {
                    continue;
                }

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let size = metadata.len();
                total_scanned_files += 1;
                total_scanned_bytes = total_scanned_bytes.saturating_add(size);

                // Ignore files below minimum size threshold (e.g., 0-byte files)
                if size >= min_size_bytes && size > 0 {
                    size_map.entry(size).or_default().push(entry.into_path());
                }
            }
        }

        // Filter out unique file sizes (len < 2)
        let candidates_stage1: Vec<(u64, Vec<PathBuf>)> = size_map
            .into_iter()
            .filter(|(_, paths)| paths.len() >= 2)
            .collect();

        // Stage 2: 4KB Header/Footer boundary hashing
        let mut stage2_candidates: Vec<(u64, Vec<PathBuf>)> = Vec::new();

        for (size, paths) in candidates_stage1 {
            let mut partial_hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

            for path in paths {
                if let Ok(partial_hash) = Self::compute_partial_hash(&path, size) {
                    partial_hash_map.entry(partial_hash).or_default().push(path);
                }
            }

            for (_, matching_paths) in partial_hash_map {
                if matching_paths.len() >= 2 {
                    stage2_candidates.push((size, matching_paths));
                }
            }
        }

        // Stage 3: Full cryptographic BLAKE3 verification
        let mut final_duplicate_groups = Vec::new();
        let mut total_duplicate_files = 0usize;
        let mut potential_reclaimed_bytes = 0u64;

        for (size, paths) in stage2_candidates {
            let mut full_hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

            for path in paths {
                if let Ok(full_hash) = Self::compute_full_blake3(&path) {
                    full_hash_map.entry(full_hash).or_default().push(path);
                }
            }

            for (hash, matching_paths) in full_hash_map {
                let count = matching_paths.len();
                if count >= 2 {
                    let wasted = size.saturating_mul((count - 1) as u64);
                    total_duplicate_files += count;
                    potential_reclaimed_bytes = potential_reclaimed_bytes.saturating_add(wasted);

                    final_duplicate_groups.push(DuplicateGroup {
                        file_size: size,
                        blake3_hash: hash,
                        file_paths: matching_paths,
                        wasted_bytes: wasted,
                    });
                }
            }
        }

        // Sort groups by wasted bytes descending
        final_duplicate_groups.sort_by_key(|b| std::cmp::Reverse(b.wasted_bytes));

        let elapsed_millis = start_time.elapsed().as_millis() as u64;

        Ok(DeduplicationSummary {
            total_scanned_files,
            total_scanned_bytes,
            duplicate_groups: final_duplicate_groups,
            total_duplicate_files,
            potential_reclaimed_bytes,
            elapsed_millis,
        })
    }

    /// Read the 4KB header and footer to compute a fast partial boundary hash
    fn compute_partial_hash(path: &Path, file_size: u64) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = blake3::Hasher::new();

        // If file is smaller than 2 * 4KB, hash entire file
        let threshold = (HEADER_FOOTER_SAMPLE_SIZE * 2) as u64;
        if file_size <= threshold {
            let mut buf = Vec::with_capacity(file_size as usize);
            file.read_to_end(&mut buf)?;
            hasher.update(&buf);
        } else {
            // Read 4KB header
            let mut header = vec![0u8; HEADER_FOOTER_SAMPLE_SIZE];
            file.read_exact(&mut header)?;
            hasher.update(&header);

            // Seek to last 4KB footer
            file.seek(SeekFrom::End(-(HEADER_FOOTER_SAMPLE_SIZE as i64)))?;
            let mut footer = vec![0u8; HEADER_FOOTER_SAMPLE_SIZE];
            file.read_exact(&mut footer)?;
            hasher.update(&footer);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute full BLAKE3 cryptographic hash streaming in 64KB blocks
    pub fn compute_full_blake3(path: &Path) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0u8; STREAM_BUFFER_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_duplicate_detection_end_to_end() {
        let dir = tempdir().unwrap();

        // Create 2 identical files
        let f1 = dir.path().join("copy1.dat");
        let f2 = dir.path().join("copy2.dat");
        let payload = vec![42u8; 10000];
        {
            let mut file1 = File::create(&f1).unwrap();
            file1.write_all(&payload).unwrap();
            file1.sync_all().unwrap();
        }
        {
            let mut file2 = File::create(&f2).unwrap();
            file2.write_all(&payload).unwrap();
            file2.sync_all().unwrap();
        }

        // Create a different file of same size
        let f3 = dir.path().join("different_content.dat");
        let mut diff_payload = vec![42u8; 10000];
        diff_payload[5000] = 99; // Change a byte
        {
            let mut file3 = File::create(&f3).unwrap();
            file3.write_all(&diff_payload).unwrap();
            file3.sync_all().unwrap();
        }

        // Create a unique file of different size
        let f4 = dir.path().join("unique.txt");
        {
            let mut file4 = File::create(&f4).unwrap();
            file4.write_all(b"short").unwrap();
            file4.sync_all().unwrap();
        }

        let summary = DuplicateDetector::scan_duplicates(&[dir.path()], 1).unwrap();

        assert_eq!(summary.total_scanned_files, 4);
        assert_eq!(summary.duplicate_groups.len(), 1);
        assert_eq!(summary.duplicate_groups[0].file_paths.len(), 2);
        assert_eq!(summary.duplicate_groups[0].file_size, 10000);
        assert_eq!(summary.duplicate_groups[0].wasted_bytes, 10000);
        assert_eq!(summary.total_duplicate_files, 2);
        assert_eq!(summary.potential_reclaimed_bytes, 10000);
    }

    #[test]
    fn test_no_duplicates_returns_empty_groups() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("a.txt");
        let f2 = dir.path().join("b.txt");
        let mut file1 = File::create(&f1).unwrap();
        let mut file2 = File::create(&f2).unwrap();
        file1.write_all(b"one").unwrap();
        file2.write_all(b"two three").unwrap();

        let summary = DuplicateDetector::scan_duplicates(&[dir.path()], 1).unwrap();
        assert_eq!(summary.duplicate_groups.len(), 0);
        assert_eq!(summary.potential_reclaimed_bytes, 0);
    }
}
