#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for AI File Explorer

use crate::models::{
    BatchRenameResult, BatchRenameRule, ContentSearchQuery, ContentSearchResult,
    DeduplicationSummary, DirectoryListing, ExtractedDocument, FileItem, FolderStats, OcrResult,
    RenamePreview, ScreenshotFingerprint, ScreenshotSearchResult, SearchMode, TreemapNode,
};
use crate::service::FileExplorerService;
use serde_json::{json, Value};
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

/// Business-tier PII & sensitive data compliance scan command
pub fn scan_compliance_cmd(
    path: String,
) -> Result<crate::compliance::ComplianceScanSummary, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let gate = shared_licensing::TierGate::enterprise();
    svc.scan_directory_compliance(&path, &gate)
        .map_err(|e| e.to_string())
}

fn resolve_valid_path(raw: Option<&str>) -> String {
    if let Some(s) = raw {
        let trimmed = s.trim();
        if !trimmed.is_empty() && std::path::Path::new(trimmed).exists() {
            return trimmed.to_string();
        }
    }
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

/// Dispatch desktop IPC commands from the Svelte 5 AI File Explorer frontend
pub fn dispatch_ipc_command(cmd: &str, args: Value) -> Result<Value, String> {
    match cmd {
        "list_directory_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let listing = list_directory_cmd(path)?;
            serde_json::to_value(listing).map_err(|e| e.to_string())
        }
        "search_files_cmd" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
            let res = search_files_cmd(query, limit)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "index_directory_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let count = index_directory_cmd(path)?;
            Ok(json!(count))
        }
        "scan_duplicates_cmd" => {
            let roots: Vec<String> = args
                .get("search_roots")
                .or_else(|| args.get("searchRoots"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str())
                        .map(|s| resolve_valid_path(Some(s)))
                        .collect()
                })
                .unwrap_or_else(|| vec![resolve_valid_path(None)]);
            let min_size = args
                .get("min_size_bytes")
                .or_else(|| args.get("minSizeBytes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1024);
            let res = scan_duplicates_cmd(roots, min_size)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "analyze_storage_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let top_limit = args
                .get("top_limit")
                .or_else(|| args.get("topLimit"))
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let res = analyze_storage_cmd(path, top_limit)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "get_treemap_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let width = args.get("width").and_then(|v| v.as_f64()).unwrap_or(1024.0);
            let height = args.get("height").and_then(|v| v.as_f64()).unwrap_or(768.0);
            let max_depth = args
                .get("max_depth")
                .or_else(|| args.get("maxDepth"))
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as usize;
            let res = get_treemap_cmd(path, width, height, max_depth)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "preview_batch_rename_cmd" => {
            let paths: Vec<String> = args
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let rule_val = args
                .get("rule")
                .cloned()
                .ok_or_else(|| "Missing rule".to_string())?;
            let rule: BatchRenameRule =
                serde_json::from_value(rule_val).map_err(|e| e.to_string())?;
            let res = preview_batch_rename_cmd(paths, rule)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "execute_batch_rename_cmd" => {
            let previews_val = args
                .get("previews")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));
            let previews: Vec<RenamePreview> =
                serde_json::from_value(previews_val).map_err(|e| e.to_string())?;
            let res = execute_batch_rename_cmd(previews)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "extract_document_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let res = extract_document_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "index_document_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let res = index_document_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "search_content_cmd" => {
            let q_val = args.get("query").cloned().unwrap_or(args);
            let query: ContentSearchQuery =
                serde_json::from_value(q_val).unwrap_or(ContentSearchQuery {
                    query: String::new(),
                    mode: SearchMode::Combined,
                    limit: 25,
                    extension_filter: None,
                });
            let res = search_content_cmd(query)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "ocr_image_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let res = ocr_image_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "index_screenshot_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let res = index_screenshot_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "search_screenshots_by_hash_cmd" => {
            let target_hash = args
                .get("target_hash")
                .or_else(|| args.get("targetHash"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let max_distance = args
                .get("max_distance")
                .or_else(|| args.get("maxDistance"))
                .and_then(|v| v.as_u64())
                .unwrap_or(12) as u32;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let res = search_screenshots_by_hash_cmd(target_hash, max_distance, limit)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "search_screenshots_by_text_cmd" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let res = search_screenshots_by_text_cmd(query, limit)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "scan_compliance_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let res = scan_compliance_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        _ => Err(format!("Unknown AI File Explorer IPC command: {}", cmd)),
    }
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
