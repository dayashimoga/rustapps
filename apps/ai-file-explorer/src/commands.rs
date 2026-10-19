#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for AI File Explorer

use crate::models::{
    BatchRenameResult, BatchRenameRule, ContentSearchQuery, ContentSearchResult,
    DeduplicationSummary, DirectoryListing, ExtractedDocument, FileItem, FolderStats, OcrResult,
    RenamePreview, ScreenshotFingerprint, ScreenshotSearchResult, TreemapNode,
};
use crate::service::FileExplorerService;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

static SERVICE: OnceLock<Arc<Mutex<FileExplorerService>>> = OnceLock::new();

pub fn get_service() -> Arc<Mutex<FileExplorerService>> {
    SERVICE
        .get_or_init(|| {
            Arc::new(Mutex::new(
                FileExplorerService::new().expect("Failed to initialize FileExplorerService"),
            ))
        })
        .clone()
}

/// List directory contents
pub fn list_directory_cmd(path: String) -> Result<DirectoryListing, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.list_directory(&path).map_err(|e| e.to_string())
}

/// Search indexed files using Tantivy (<50ms SLA)
pub fn search_files_cmd(query: String, limit: usize) -> Result<Vec<FileItem>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.search(&query, limit).map_err(|e| e.to_string())
}

/// Index a target directory into Tantivy
pub fn index_directory_cmd(path: String) -> Result<usize, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let listing = svc.list_directory(&path).map_err(|e| e.to_string())?;
    svc.index_files(&listing.items).map_err(|e| e.to_string())
}

/// Scan for duplicate files using 3-tier filtration
pub fn scan_duplicates_cmd(
    search_roots: Vec<String>,
    min_size_bytes: u64,
) -> Result<DeduplicationSummary, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let roots: Vec<PathBuf> = search_roots.into_iter().map(PathBuf::from).collect();
    svc.scan_duplicates(&roots, min_size_bytes)
        .map_err(|e| e.to_string())
}

/// Compute storage analytics
pub fn analyze_storage_cmd(path: String, top_limit: usize) -> Result<FolderStats, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.analyze_storage(&path, top_limit)
        .map_err(|e| e.to_string())
}

/// Generate squarified treemap bounding rectangles
pub fn get_treemap_cmd(
    path: String,
    width: f64,
    height: f64,
    max_depth: usize,
) -> Result<TreemapNode, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.generate_treemap(&path, width, height, max_depth)
        .map_err(|e| e.to_string())
}

/// Preview proposed batch rename
pub fn preview_batch_rename_cmd(
    paths: Vec<String>,
    rule: BatchRenameRule,
) -> Result<Vec<RenamePreview>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    svc.preview_batch_rename(&path_bufs, &rule)
        .map_err(|e| e.to_string())
}

/// Execute atomic batch rename with rollback
pub fn execute_batch_rename_cmd(previews: Vec<RenamePreview>) -> Result<BatchRenameResult, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.execute_batch_rename(&previews)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Phase 6 Command Endpoints
// ---------------------------------------------------------------------------

/// Extracts text and metadata from a document file
pub fn extract_document_cmd(path: String) -> Result<ExtractedDocument, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.extract_document(&path).map_err(|e| e.to_string())
}

/// Indexes a document file into Tantivy and vector embeddings
pub fn index_document_cmd(path: String) -> Result<ExtractedDocument, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let doc = svc.extract_document(&path).map_err(|e| e.to_string())?;
    svc.index_extracted_document(&doc)
        .map_err(|e| e.to_string())?;
    Ok(doc)
}

/// Content and semantic search endpoint
pub fn search_content_cmd(query: ContentSearchQuery) -> Result<ContentSearchResult, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.search_content(&query).map_err(|e| e.to_string())
}

/// OCR processing on an image file
pub fn ocr_image_cmd(path: String) -> Result<OcrResult, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.ocr_image(&path).map_err(|e| e.to_string())
}

/// Screenshot visual and OCR indexing
pub fn index_screenshot_cmd(path: String) -> Result<ScreenshotFingerprint, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.index_screenshot(&path).map_err(|e| e.to_string())
}

/// Visual similarity screenshot search
pub fn search_screenshots_by_hash_cmd(
    target_hash: u64,
    max_distance: u32,
    limit: usize,
) -> Result<Vec<ScreenshotSearchResult>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    Ok(svc.search_screenshots_by_hash(target_hash, max_distance, limit))
}

/// Text keyword screenshot search
pub fn search_screenshots_by_text_cmd(
    query: String,
    limit: usize,
) -> Result<Vec<ScreenshotSearchResult>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    Ok(svc.search_screenshots_by_text(&query, limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SearchMode;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_commands_service_workflow() {
        let dir = tempdir().unwrap();
        let path_str = dir.path().to_string_lossy().to_string();

        let listing = list_directory_cmd(path_str.clone()).unwrap();
        assert_eq!(listing.total_count, 0);

        let stats = analyze_storage_cmd(path_str.clone(), 5).unwrap();
        assert_eq!(stats.total_files, 0);

        let treemap = get_treemap_cmd(path_str, 800.0, 600.0, 1).unwrap();
        assert_eq!(treemap.bounds.width, 800.0);
    }

    #[test]
    fn test_commands_phase_6_content_search() {
        let dir = tempdir().unwrap();
        let doc_file = dir.path().join("readme.md");
        let mut f = File::create(&doc_file).unwrap();
        f.write_all(b"# Welcome\nThis is an offline file search guide.")
            .unwrap();

        let doc_path = doc_file.to_string_lossy().to_string();
        let doc = index_document_cmd(doc_path).unwrap();
        assert_eq!(doc.file_name, "readme.md");

        let q = ContentSearchQuery {
            query: "offline file".to_string(),
            mode: SearchMode::Combined,
            limit: 5,
            extension_filter: None,
        };
        let res = search_content_cmd(q).unwrap();
        assert_eq!(res.total_matches, 1);
        assert_eq!(res.items[0].file_name, "readme.md");
    }
}
