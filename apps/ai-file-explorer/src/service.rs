#![forbid(unsafe_code)]
//! AI File Explorer Orchestrator Service
//!
//! Unified facade combining crawling, Tantivy indexing, 3-tier duplicate detection,
//! storage analytics, squarified treemaps, transactional batch rename,
//! multi-format document extraction, pure-Rust offline OCR, screenshot perceptual search,
//! and 8-bit quantized local vector embeddings.

use crate::analytics::StorageAnalytics;
use crate::batch_rename::BatchRenamer;
use crate::crawler::FileCrawler;
use crate::dedup::DuplicateDetector;
use crate::doc_extract::DocumentExtractor;
use crate::embeddings::VectorIndex;
use crate::indexer::FileSearchIndexer;
use crate::models::{
    BatchRenameResult, BatchRenameRule, ContentSearchQuery, ContentSearchResult,
    DeduplicationSummary, DirectoryListing, ExtractedDocument, FileItem, FolderStats, OcrResult,
    RenamePreview, ScreenshotFingerprint, ScreenshotSearchResult, SearchMode, TreemapNode,
};
use crate::ocr::OfflineOcrEngine;
use crate::screenshot::ScreenshotCatalog;
use crate::treemap::TreemapGenerator;
use crate::watcher::DirectoryWatcher;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
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
    #[error("Document extract error: {0}")]
    Extract(#[from] crate::doc_extract::ExtractError),
    #[error("OCR error: {0}")]
    Ocr(String),
    #[error("Service internal error: {0}")]
    Internal(String),
}

/// Core File Explorer application service
pub struct FileExplorerService {
    indexer: Arc<FileSearchIndexer>,
    watcher: Arc<DirectoryWatcher>,
    vector_index: Arc<VectorIndex>,
    screenshot_catalog: Arc<ScreenshotCatalog>,
}

impl FileExplorerService {
    /// Initialize application service with in-memory or on-disk indexer
    pub fn new() -> Result<Self, ServiceError> {
        let indexer = Arc::new(FileSearchIndexer::create_in_memory()?);
        let watcher = Arc::new(DirectoryWatcher::new());
        let vector_index = Arc::new(VectorIndex::new());
        let screenshot_catalog = Arc::new(ScreenshotCatalog::new());
        Ok(Self {
            indexer,
            watcher,
            vector_index,
            screenshot_catalog,
        })
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

    // -----------------------------------------------------------------------
    // Phase 6 Advanced Capabilities
    // -----------------------------------------------------------------------

    /// Extracts text and metadata from a document file
    pub fn extract_document<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<ExtractedDocument, ServiceError> {
        Ok(DocumentExtractor::extract_file(path)?)
    }

    /// Indexes an extracted document into both Tantivy full-text and vector index
    pub fn index_extracted_document(&self, doc: &ExtractedDocument) -> Result<(), ServiceError> {
        // 1. Tantivy keyword index
        self.indexer.index_document(doc)?;

        // 2. 8-bit quantized vector index
        self.vector_index
            .index_document(&doc.path, &doc.content_text);

        Ok(())
    }

    /// Performs keyword, semantic, or combined hybrid content search
    pub fn search_content(
        &self,
        query: &ContentSearchQuery,
    ) -> Result<ContentSearchResult, ServiceError> {
        let start = Instant::now();

        let mut items = match query.mode {
            SearchMode::Keyword => self.indexer.search_content(&query.query, query.limit)?,
            SearchMode::Semantic => self.vector_index.search(&query.query, query.limit),
            SearchMode::Combined => {
                let mut kw_hits = self
                    .indexer
                    .search_content(&query.query, query.limit)
                    .unwrap_or_default();
                let sem_hits = self.vector_index.search(&query.query, query.limit);

                // Combine and deduplicate hits by path
                let mut seen_paths = std::collections::HashSet::new();
                let mut combined = Vec::new();

                for item in kw_hits.drain(..) {
                    if seen_paths.insert(item.path.clone()) {
                        combined.push(item);
                    }
                }
                for item in sem_hits {
                    if seen_paths.insert(item.path.clone()) {
                        combined.push(item);
                    }
                }
                combined.truncate(query.limit);
                combined
            }
        };

        // Filter by extension if requested
        if let Some(ref ext_filter) = query.extension_filter {
            let filter_lower = ext_filter.to_lowercase();
            items.retain(|item| {
                item.path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase() == filter_lower)
                    .unwrap_or(false)
            });
        }

        let total_matches = items.len();
        let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(ContentSearchResult {
            items,
            total_matches,
            execution_time_ms,
        })
    }

    /// Performs offline OCR on an image file
    pub fn ocr_image<P: AsRef<Path>>(&self, path: P) -> Result<OcrResult, ServiceError> {
        OfflineOcrEngine::process_file(path).map_err(ServiceError::Ocr)
    }

    /// Indexes a screenshot file computing its 64-bit dHash and OCR text
    pub fn index_screenshot<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<ScreenshotFingerprint, ServiceError> {
        let fp = self
            .screenshot_catalog
            .index_screenshot(&path)
            .map_err(ServiceError::Ocr)?;

        // Also add OCR tokens to Tantivy index
        if !fp.ocr_text.is_empty() {
            let _ = self.indexer.index_ocr_tokens(path.as_ref(), &fp.ocr_text);
        }

        Ok(fp)
    }

    /// Searches for screenshots by visual dHash similarity
    pub fn search_screenshots_by_hash(
        &self,
        target_hash: u64,
        max_distance: u32,
        limit: usize,
    ) -> Vec<ScreenshotSearchResult> {
        self.screenshot_catalog
            .search_by_visual_hash(target_hash, max_distance, limit)
    }

    /// Searches for screenshots by OCR text keyword
    pub fn search_screenshots_by_text(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<ScreenshotSearchResult> {
        self.screenshot_catalog.search_by_ocr_text(query, limit)
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

    #[test]
    fn test_service_phase_6_content_and_semantic_search() {
        let dir = tempdir().unwrap();
        let doc_path = dir.path().join("system_design.md");
        let mut file = File::create(&doc_path).unwrap();
        file.write_all(b"# System Design\n\nHigh throughput streaming parsers with zero copy.")
            .unwrap();

        let service = FileExplorerService::new().unwrap();

        // 1. Document extraction
        let doc = service.extract_document(&doc_path).unwrap();
        assert_eq!(doc.file_name, "system_design.md");
        assert!(doc.content_text.contains("streaming parsers"));

        // 2. Document indexing
        service.index_extracted_document(&doc).unwrap();

        // 3. Keyword Content Search
        let kw_query = ContentSearchQuery {
            query: "streaming".to_string(),
            mode: SearchMode::Keyword,
            limit: 5,
            extension_filter: None,
        };
        let kw_res = service.search_content(&kw_query).unwrap();
        assert_eq!(kw_res.total_matches, 1);
        assert_eq!(kw_res.items[0].file_name, "system_design.md");

        // 4. Semantic Search
        let sem_query = ContentSearchQuery {
            query: "high throughput data parser".to_string(),
            mode: SearchMode::Semantic,
            limit: 5,
            extension_filter: None,
        };
        let sem_res = service.search_content(&sem_query).unwrap();
        assert_eq!(sem_res.total_matches, 1);
        assert!(sem_res.items[0].score > 0.0);
    }
}
