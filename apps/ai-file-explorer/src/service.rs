#![forbid(unsafe_code)]
//! AI File Explorer Orchestrator Service

use crate::analytics::StorageAnalytics;
use crate::batch_rename::BatchRenamer;
use crate::crawler::FileCrawler;
use crate::dedup::DuplicateDetector;
use crate::indexer::FileSearchIndexer;
use crate::models::{
    BatchRenameResult, BatchRenameRule, DeduplicationSummary, DirectoryListing, FileItem,
    FolderStats, RenamePreview, TreemapNode,
};
use crate::treemap::TreemapGenerator;
use crate::watcher::DirectoryWatcher;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Crawler error: {0}")]
    Crawler(#[from] crate::crawler::CrawlerError),
    #[error("Indexer error: {0}")]
    Indexer(#[from] crate::indexer::IndexerError),
    #[error("Dedup error: {0}")]
    Dedup(#[from] crate::dedup::DedupError),
    #[error("Analytics error: {0}")]
    Analytics(#[from] crate::analytics::AnalyticsError),
    #[error("Treemap error: {0}")]
    Treemap(#[from] crate::treemap::TreemapError),
    #[error("Rename error: {0}")]
    Rename(#[from] crate::batch_rename::RenameError),
    #[error("Service internal error: {0}")]
    Internal(String),
}

/// Core File Explorer application service
pub struct FileExplorerService {
    indexer: Arc<FileSearchIndexer>,
    watcher: Arc<DirectoryWatcher>,
}

impl FileExplorerService {
    /// Initialize application service with in-memory or on-disk indexer
    pub fn new() -> Result<Self, ServiceError> {
        let indexer = Arc::new(FileSearchIndexer::create_in_memory()?);
        let watcher = Arc::new(DirectoryWatcher::new());
        Ok(Self { indexer, watcher })
    }

    /// List single directory level contents for instant browsing
    pub fn list_directory<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<DirectoryListing, ServiceError> {
        Ok(FileCrawler::list_directory(path)?)
    }

    /// Search indexed files using Tantivy (<50ms SLA)
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<FileItem>, ServiceError> {
        Ok(self.indexer.search(query, limit)?)
    }

    /// Index a list of discovered files into the Tantivy engine
    pub fn index_files(&self, items: &[FileItem]) -> Result<usize, ServiceError> {
        Ok(self.indexer.index_files(items)?)
    }

    /// Scan target directory for duplicate files using 3-tier filtration
    pub fn scan_duplicates<P: AsRef<Path>>(
        &self,
        search_roots: &[P],
        min_size_bytes: u64,
    ) -> Result<DeduplicationSummary, ServiceError> {
        Ok(DuplicateDetector::scan_duplicates(
            search_roots,
            min_size_bytes,
        )?)
    }

    /// Analyze directory for storage breakdown (file counts, size sums, extensions, largest files)
    pub fn analyze_storage<P: AsRef<Path>>(
        &self,
        root_path: P,
        top_largest_limit: usize,
    ) -> Result<FolderStats, ServiceError> {
        Ok(StorageAnalytics::analyze_directory(
            root_path,
            top_largest_limit,
        )?)
    }

    /// Generate squarified treemap bounding rectangles for given viewport dimensions
    pub fn generate_treemap<P: AsRef<Path>>(
        &self,
        root_path: P,
        width: f64,
        height: f64,
        max_depth: usize,
    ) -> Result<TreemapNode, ServiceError> {
        Ok(TreemapGenerator::generate_treemap(
            root_path, width, height, max_depth,
        )?)
    }

    /// Preview proposed batch rename and detect collisions
    pub fn preview_batch_rename(
        &self,
        file_paths: &[PathBuf],
        rule: &BatchRenameRule,
    ) -> Result<Vec<RenamePreview>, ServiceError> {
        Ok(BatchRenamer::preview_renames(file_paths, rule)?)
    }

    /// Execute atomic batch rename with automatic rollback on error
    pub fn execute_batch_rename(
        &self,
        previews: &[RenamePreview],
    ) -> Result<BatchRenameResult, ServiceError> {
        Ok(BatchRenamer::execute_batch_rename(previews)?)
    }

    /// Start real-time filesystem watching
    pub fn start_watching<P: AsRef<Path>>(&self, dir_path: P) {
        self.watcher.start_watching(dir_path, 100);
    }

    /// Stop filesystem watching
    pub fn stop_watching(&self) {
        self.watcher.stop();
    }
}

impl Default for FileExplorerService {
    fn default() -> Self {
        Self::new().expect("Failed to initialize FileExplorerService")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_service_lifecycle_and_operations() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("code.rs");
        let mut file1 = File::create(&f1).unwrap();
        file1.write_all(b"fn main() {}").unwrap();

        let service = FileExplorerService::new().unwrap();

        // 1. Directory listing
        let listing = service.list_directory(dir.path()).unwrap();
        assert_eq!(listing.total_count, 1);

        // 2. Indexing and search
        let indexed = service.index_files(&listing.items).unwrap();
        assert_eq!(indexed, 1);

        let hits = service.search("code", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_name, "code.rs");

        // 3. Storage analytics
        let stats = service.analyze_storage(dir.path(), 5).unwrap();
        assert_eq!(stats.total_files, 1);

        // 4. Treemap
        let treemap = service
            .generate_treemap(dir.path(), 800.0, 600.0, 2)
            .unwrap();
        assert_eq!(treemap.bounds.width, 800.0);
    }
}
