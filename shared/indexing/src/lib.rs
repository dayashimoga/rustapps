#![forbid(unsafe_code)]
//! Shared Indexing Subsystem
//!
//! Provides asynchronous, high-throughput filesystem traversal,
//! streaming chunking pipelines, and responsive cancellation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use tokio::sync::mpsc;
use walkdir::WalkDir;

/// Indexing error definitions
#[derive(Debug, Error)]
pub enum IndexingError {
    #[error("I/O error during filesystem crawl: {0}")]
    Io(#[from] std::io::Error),
    #[error("Indexing operation cancelled")]
    Cancelled,
    #[error("Channel send failed")]
    ChannelSendFailed,
}

/// Metadata extracted per discovered file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FileMetadataRecord {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub modified_timestamp_secs: u64,
}

/// Configuration options for filesystem traversal
#[derive(Debug, Clone)]
pub struct WalkerOptions {
    pub max_depth: Option<usize>,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub buffer_size: usize,
}

impl Default for WalkerOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            follow_symlinks: false,
            include_hidden: false,
            buffer_size: 1024,
        }
    }
}

/// High-throughput filesystem crawler
pub struct ConcurrentCrawler;

impl ConcurrentCrawler {
    /// Walk the target directory asynchronously, streaming records into an mpsc receiver.
    /// Supports graceful cancellation via the `cancel_rx` watch channel.
    pub fn crawl_stream<P: AsRef<Path>>(
        root: P,
        options: WalkerOptions,
        cancel_rx: tokio::sync::watch::Receiver<bool>,
    ) -> mpsc::Receiver<FileMetadataRecord> {
        let (tx, rx) = mpsc::channel(options.buffer_size);
        let root_path = root.as_ref().to_path_buf();

        tokio::task::spawn_blocking(move || {
            let mut walker = WalkDir::new(root_path).follow_links(options.follow_symlinks);

            if let Some(depth) = options.max_depth {
                walker = walker.max_depth(depth);
            }

            for entry_res in walker {
                // Check cancellation signal
                if *cancel_rx.borrow() {
                    break;
                }

                let entry = match entry_res {
                    Ok(e) => e,
                    Err(_) => continue, // Gracefully bypass unreadable files
                };

                // Check hidden file filter on child entries
                if entry.depth() > 0 && !options.include_hidden {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with('.') && name != "." && name != ".." {
                            continue;
                        }
                    }
                }

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let path = entry.path().to_path_buf();
                let file_name = entry.file_name().to_string_lossy().to_string();
                let extension = path.extension().map(|s| s.to_string_lossy().to_string());
                let size_bytes = metadata.len();
                let is_dir = metadata.is_dir();

                let modified_timestamp_secs = metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let record = FileMetadataRecord {
                    path,
                    file_name,
                    extension,
                    size_bytes,
                    is_dir,
                    modified_timestamp_secs,
                };

                if tx.blocking_send(record).is_err() {
                    // Receiver was dropped by consumer
                    break;
                }
            }
        });

        rx
    }
}

/// Chunking accumulator that groups streamed records into batches of `chunk_size`
pub struct ChunkingPipeline;

impl ChunkingPipeline {
    pub async fn collect_chunks(
        mut rx: mpsc::Receiver<FileMetadataRecord>,
        chunk_size: usize,
    ) -> Vec<Vec<FileMetadataRecord>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::with_capacity(chunk_size);

        while let Some(record) = rx.recv().await {
            current_chunk.push(record);
            if current_chunk.len() >= chunk_size {
                chunks.push(std::mem::replace(
                    &mut current_chunk,
                    Vec::with_capacity(chunk_size),
                ));
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }
}

/// Changes detected between two directory snapshots
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectoryDelta {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

impl DirectoryDelta {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}

/// Individual filesystem change event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsDeltaEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// Point-in-time recursive snapshot of directory entries
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectorySnapshot {
    pub root: PathBuf,
    pub files: HashMap<PathBuf, (u64, u64)>, // path -> (size_bytes, modified_timestamp_secs)
}

impl DirectorySnapshot {
    /// Capture a full recursive snapshot of the target directory
    pub fn capture<P: AsRef<Path>>(root: P, follow_symlinks: bool) -> Result<Self, IndexingError> {
        let root_path = root.as_ref().to_path_buf();
        let mut files = HashMap::new();

        let walker = WalkDir::new(&root_path).follow_links(follow_symlinks);
        for entry_res in walker {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path().to_path_buf();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = metadata.len();
            let modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            files.insert(path, (size, modified));
        }

        Ok(Self {
            root: root_path,
            files,
        })
    }

    /// Compare previous snapshot with current snapshot to detect additions, modifications, and removals
    pub fn diff(&self, current: &DirectorySnapshot) -> DirectoryDelta {
        let mut delta = DirectoryDelta::default();

        for (path, (curr_size, curr_mod)) in &current.files {
            match self.files.get(path) {
                None => {
                    delta.added.push(path.clone());
                }
                Some((prev_size, prev_mod)) => {
                    if prev_size != curr_size || prev_mod != curr_mod {
                        delta.modified.push(path.clone());
                    }
                }
            }
        }

        for path in self.files.keys() {
            if !current.files.contains_key(path) {
                delta.removed.push(path.clone());
            }
        }

        delta.added.sort();
        delta.modified.sort();
        delta.removed.sort();

        delta
    }
}

/// Recursive change detector maintaining rolling state across poll intervals
pub struct RecursiveChangeDetector {
    root: PathBuf,
    follow_symlinks: bool,
    last_snapshot: DirectorySnapshot,
}

impl RecursiveChangeDetector {
    pub fn new<P: AsRef<Path>>(root: P, follow_symlinks: bool) -> Result<Self, IndexingError> {
        let root_path = root.as_ref().to_path_buf();
        let snapshot = DirectorySnapshot::capture(&root_path, follow_symlinks)?;
        Ok(Self {
            root: root_path,
            follow_symlinks,
            last_snapshot: snapshot,
        })
    }

    pub fn poll_changes(&mut self) -> Result<DirectoryDelta, IndexingError> {
        let current = DirectorySnapshot::capture(&self.root, self.follow_symlinks)?;
        let delta = self.last_snapshot.diff(&current);
        self.last_snapshot = current;
        Ok(delta)
    }

    pub fn current_snapshot(&self) -> &DirectorySnapshot {
        &self.last_snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crawl_stream_and_chunking() {
        let temp = tempfile::tempdir().unwrap();
        let file1 = temp.path().join("alpha.txt");
        let file2 = temp.path().join("beta.md");
        let sub_dir = temp.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        let file3 = sub_dir.join("gamma.json");

        std::fs::write(&file1, "alpha content").unwrap();
        std::fs::write(&file2, "beta content").unwrap();
        std::fs::write(&file3, "gamma content").unwrap();

        let (_cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let rx = ConcurrentCrawler::crawl_stream(temp.path(), WalkerOptions::default(), cancel_rx);

        let chunks = ChunkingPipeline::collect_chunks(rx, 2).await;
        let total_items: usize = chunks.iter().map(|c| c.len()).sum();

        // temp dir root + 3 files + 1 subdir = 5 items
        assert_eq!(total_items, 5);
        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_crawl_stream_cancellation() {
        let temp = tempfile::tempdir().unwrap();
        for i in 0..100 {
            std::fs::write(temp.path().join(format!("file_{}.txt", i)), "data").unwrap();
        }

        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        // Trigger cancellation immediately
        cancel_tx.send(true).unwrap();

        let mut rx =
            ConcurrentCrawler::crawl_stream(temp.path(), WalkerOptions::default(), cancel_rx);

        let mut count = 0;
        while rx.recv().await.is_some() {
            count += 1;
        }

        // Cancelled early, count should be small (< 100)
        assert!(count < 100);
    }

    #[test]
    fn test_recursive_directory_snapshot_and_diff() {
        let temp = tempfile::tempdir().unwrap();
        let sub1 = temp.path().join("level1");
        let sub2 = sub1.join("level2");
        std::fs::create_dir_all(&sub2).unwrap();

        let file_a = temp.path().join("root.txt");
        let file_b = sub1.join("nested.md");
        let file_c = sub2.join("deep.rs");

        std::fs::write(&file_a, "root file").unwrap();
        std::fs::write(&file_b, "nested file").unwrap();
        std::fs::write(&file_c, "deep file").unwrap();

        let mut detector = RecursiveChangeDetector::new(temp.path(), false).unwrap();
        assert_eq!(detector.current_snapshot().files.len(), 6); // root, sub1, sub2, file_a, file_b, file_c

        // First poll with no changes should be empty
        let initial_delta = detector.poll_changes().unwrap();
        assert!(initial_delta.is_empty());

        // Now modify file_c, remove file_b, and add file_d deep inside level2
        std::fs::write(
            &file_c,
            "deep file modified content that is definitely longer",
        )
        .unwrap();
        std::fs::remove_file(&file_b).unwrap();
        let file_d = sub2.join("deep_new.json");
        std::fs::write(&file_d, "{\"hello\": \"world\"}").unwrap();

        let delta = detector.poll_changes().unwrap();
        assert_eq!(delta.added, vec![file_d]);
        assert_eq!(delta.removed, vec![file_b]);
        assert_eq!(delta.modified, vec![file_c]);
    }
}
