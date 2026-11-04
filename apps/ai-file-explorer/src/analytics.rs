#![forbid(unsafe_code)]
//! Storage Analytics and Directory Statistics Engine

use crate::crawler::strip_verbatim_prefix;
use crate::models::{FileItem, FolderStats};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("I/O error during folder analysis: {0}")]
    Io(#[from] std::io::Error),
    #[error("Directory not found: {0}")]
    NotFound(PathBuf),
    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),
}

/// Storage analytics calculator
pub struct StorageAnalytics;

impl StorageAnalytics {
    /// Compute comprehensive storage analytics for the given directory tree
    pub fn analyze_directory<P: AsRef<Path>>(
        root_path: P,
        top_largest_limit: usize,
    ) -> Result<FolderStats, AnalyticsError> {
        let root_raw = root_path.as_ref();
        if !root_raw.exists() {
            return Err(AnalyticsError::NotFound(root_raw.to_path_buf()));
        }
        if !root_raw.is_dir() {
            return Err(AnalyticsError::NotADirectory(root_raw.to_path_buf()));
        }
        let root = strip_verbatim_prefix(root_raw);

        let mut total_files = 0usize;
        let mut total_dirs = 0usize;
        let mut total_size_bytes = 0u64;
        let mut extension_counts: HashMap<String, usize> = HashMap::new();
        let mut extension_sizes: HashMap<String, u64> = HashMap::new();
        let mut all_files: Vec<FileItem> = Vec::new();
        let started_at = Instant::now();
        let max_entries = 12_000usize;
        let mut visited_entries = 0usize;

        for entry_res in WalkDir::new(&root).max_depth(4).follow_links(false) {
            visited_entries += 1;
            if visited_entries > max_entries || started_at.elapsed() > Duration::from_millis(900) {
                break;
            }

            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let path = strip_verbatim_prefix(entry.path());
            let is_dir = metadata.is_dir();

            if is_dir {
                if entry.depth() > 0 {
                    total_dirs += 1;
                }
            } else {
                total_files += 1;
                let size = metadata.len();
                total_size_bytes = total_size_bytes.saturating_add(size);

                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_else(|| "none".to_string());

                *extension_counts.entry(ext.clone()).or_insert(0) += 1;
                let ext_size = extension_sizes.entry(ext.clone()).or_insert(0);
                *ext_size = ext_size.saturating_add(size);

                let modified_timestamp_secs = metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let file_name = entry.file_name().to_string_lossy().to_string();

                all_files.push(FileItem {
                    path,
                    file_name,
                    extension: if ext == "none" { None } else { Some(ext) },
                    size_bytes: size,
                    is_dir: false,
                    modified_timestamp_secs,
                    is_readonly: metadata.permissions().readonly(),
                });
            }
        }

        // Sort files by size descending to extract top largest files
        all_files.sort_by_key(|b| std::cmp::Reverse(b.size_bytes));
        let largest_files = all_files.into_iter().take(top_largest_limit).collect();

        Ok(FolderStats {
            root_path: root.to_path_buf(),
            total_files,
            total_dirs,
            total_size_bytes,
            extension_counts,
            extension_sizes,
            largest_files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_storage_analytics_calculation() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();

        let f1 = dir.path().join("big.bin");
        {
            let mut file1 = File::create(&f1).unwrap();
            file1.write_all(&vec![0u8; 100_000]).unwrap();
            file1.sync_all().unwrap();
        }

        let f2 = sub.join("small.rs");
        {
            let mut file2 = File::create(&f2).unwrap();
            file2.write_all(b"fn main() {}").unwrap();
            file2.sync_all().unwrap();
        }

        let stats = StorageAnalytics::analyze_directory(dir.path(), 5).unwrap();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.total_dirs, 1);
        assert!(stats.total_size_bytes >= 100_012);
        assert_eq!(stats.extension_counts.get("bin"), Some(&1));
        assert_eq!(stats.extension_counts.get("rs"), Some(&1));
        assert_eq!(stats.largest_files.len(), 2);
        assert_eq!(stats.largest_files[0].file_name, "big.bin");
    }
}
