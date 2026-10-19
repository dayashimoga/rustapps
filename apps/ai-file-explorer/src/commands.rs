#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for AI File Explorer

use crate::models::{
    BatchRenameResult, BatchRenameRule, DeduplicationSummary, DirectoryListing, FileItem,
    FolderStats, RenamePreview, TreemapNode,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_commands_api_end_to_end() {
        let dir = tempdir().unwrap();
        let f = dir.path().join("cmd_test.txt");
        File::create(&f).unwrap();

        let listing = list_directory_cmd(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(listing.total_count, 1);

        let indexed = index_directory_cmd(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(indexed, 1);

        let search_res = search_files_cmd("cmd_test".to_string(), 10).unwrap();
        assert_eq!(search_res.len(), 1);

        let stats = analyze_storage_cmd(dir.path().to_string_lossy().to_string(), 5).unwrap();
        assert_eq!(stats.total_files, 1);
    }
}
