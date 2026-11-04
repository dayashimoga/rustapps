#![forbid(unsafe_code)]
//! High-Throughput Concurrent Filesystem Crawler

use crate::models::{DirectoryListing, FileItem};
use shared_indexing::{ConcurrentCrawler, FileMetadataRecord, WalkerOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Error)]
pub enum CrawlerError {
    #[error("I/O error during directory read: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path does not exist: {0}")]
    NotFound(PathBuf),
    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("Operation cancelled by user")]
    Cancelled,
}

/// Normalize a filesystem path by removing Windows verbatim prefixes (`\\?\` or `\\?\UNC\`).
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", rest))
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

/// Filesystem crawler and directory inspector
pub struct FileCrawler;

impl FileCrawler {
    /// Read a single directory level for instant file explorer navigation
    pub fn list_directory<P: AsRef<Path>>(dir_path: P) -> Result<DirectoryListing, CrawlerError> {
        let path = dir_path.as_ref();
        if !path.exists() {
            return Err(CrawlerError::NotFound(path.to_path_buf()));
        }
        if !path.is_dir() {
            return Err(CrawlerError::NotADirectory(path.to_path_buf()));
        }

        let canonical_raw = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let canonical = strip_verbatim_prefix(&canonical_raw);
        let parent_path = canonical.parent().map(strip_verbatim_prefix);

        let mut items = Vec::new();
        let mut total_size_bytes = 0u64;

        let read_dir = fs::read_dir(&canonical)?;
        for entry_res in read_dir {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue, // Skip unreadable entries gracefully
            };

            let entry_path = strip_verbatim_prefix(&entry.path());
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_name = entry.file_name().to_string_lossy().to_string();
            let extension = entry_path
                .extension()
                .map(|e| e.to_string_lossy().to_string());
            let size_bytes = if metadata.is_dir() { 0 } else { metadata.len() };
            let is_dir = metadata.is_dir();
            let is_readonly = metadata.permissions().readonly();

            let modified_timestamp_secs = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            total_size_bytes = total_size_bytes.saturating_add(size_bytes);

            items.push(FileItem {
                path: entry_path,
                file_name,
                extension,
                size_bytes,
                is_dir,
                modified_timestamp_secs,
                is_readonly,
            });
        }

        // Sort directories first, then files alphabetically (case-insensitive)
        items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()),
        });

        let total_count = items.len();

        Ok(DirectoryListing {
            current_path: canonical,
            parent_path,
            items,
            total_count,
            total_size_bytes,
        })
    }

    /// Stream all files recursively using shared-indexing worker threads
    pub fn crawl_recursive<P: AsRef<Path>>(
        root: P,
        options: WalkerOptions,
        cancel_rx: watch::Receiver<bool>,
    ) -> mpsc::Receiver<FileMetadataRecord> {
        ConcurrentCrawler::crawl_stream(root, options, cancel_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_list_directory_success() {
        let dir = tempdir().unwrap();
        let sub_dir = dir.path().join("sub_folder");
        fs::create_dir(&sub_dir).unwrap();

        let file_path = dir.path().join("alpha.txt");
        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"Hello world").unwrap();
            file.sync_all().unwrap();
        }

        let listing = FileCrawler::list_directory(dir.path()).unwrap();
        assert_eq!(listing.total_count, 2);
        // Directories first
        assert!(listing.items[0].is_dir);
        assert_eq!(listing.items[0].file_name, "sub_folder");
        assert!(!listing.items[1].is_dir);
        assert_eq!(listing.items[1].file_name, "alpha.txt");
        assert_eq!(listing.items[1].size_bytes, 11);
    }

    #[test]
    fn test_list_directory_not_found() {
        let missing = PathBuf::from("Z:/non_existent_path_99999");
        let result = FileCrawler::list_directory(&missing);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_directory_not_a_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("solo.txt");
        File::create(&file_path).unwrap();

        let result = FileCrawler::list_directory(&file_path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_crawl_recursive_streaming() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("file1.log");
        let f2 = dir.path().join("file2.txt");
        File::create(&f1).unwrap();
        File::create(&f2).unwrap();

        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let mut rx = FileCrawler::crawl_recursive(dir.path(), WalkerOptions::default(), cancel_rx);

        let mut count = 0;
        while let Some(_item) = rx.recv().await {
            count += 1;
        }
        // At least dir + 2 files = 3 entries
        assert!(count >= 2);
    }
}
