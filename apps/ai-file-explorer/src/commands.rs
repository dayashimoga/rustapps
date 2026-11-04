#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for AI File Explorer

use crate::analytics::StorageAnalytics;
use crate::batch_rename::BatchRenamer;
use crate::crawler::FileCrawler;
use crate::dedup::DuplicateDetector;
use crate::doc_extract::DocumentExtractor;
use crate::models::{
    BatchRenameResult, BatchRenameRule, ContentSearchQuery, ContentSearchResult,
    DeduplicationSummary, DirectoryListing, ExtractedDocument, FileItem, FolderStats, OcrResult,
    RenamePreview, ScreenshotFingerprint, ScreenshotSearchResult, SearchMode, TreemapNode,
};
use crate::service::FileExplorerService;
use crate::treemap::TreemapGenerator;
use platform_common::{enumerate_drives_and_places, FsLocationEntry};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};
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

/// List directory contents (lock-free, never blocked by background indexing or scans)
pub fn list_directory_cmd(path: String) -> Result<DirectoryListing, String> {
    FileCrawler::list_directory(&path).map_err(|e| e.to_string())
}

/// Enumerate mounted drives (`C:\`, `H:\`, etc.) and user quick-access directories
pub fn list_drives_and_places_cmd() -> Result<Vec<FsLocationEntry>, String> {
    Ok(enumerate_drives_and_places())
}

/// Open a file or folder with the operating system's default handler
pub fn open_in_os_cmd(path: String) -> Result<bool, String> {
    let target = std::path::Path::new(&path);
    if !target.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(true)
}

/// Search indexed files using Tantivy (<50ms SLA)
pub fn search_files_cmd(query: String, limit: usize) -> Result<Vec<FileItem>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.search(&query, limit).map_err(|e| e.to_string())
}

/// Index a target directory into Tantivy
pub fn index_directory_cmd(path: String) -> Result<usize, String> {
    let listing = FileCrawler::list_directory(&path).map_err(|e| e.to_string())?;
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.index_files(&listing.items).map_err(|e| e.to_string())
}

/// Scan for duplicate files using 3-tier filtration (lock-free)
pub fn scan_duplicates_cmd(
    search_roots: Vec<String>,
    min_size_bytes: u64,
) -> Result<DeduplicationSummary, String> {
    let roots: Vec<PathBuf> = search_roots.into_iter().map(PathBuf::from).collect();
    DuplicateDetector::scan_duplicates(&roots, min_size_bytes).map_err(|e| e.to_string())
}

/// Compute storage analytics (lock-free)
pub fn analyze_storage_cmd(path: String, top_limit: usize) -> Result<FolderStats, String> {
    StorageAnalytics::analyze_directory(&path, top_limit).map_err(|e| e.to_string())
}

/// Generate squarified treemap bounding rectangles (lock-free)
pub fn get_treemap_cmd(
    path: String,
    width: f64,
    height: f64,
    max_depth: usize,
) -> Result<TreemapNode, String> {
    TreemapGenerator::generate_treemap(&path, width, height, max_depth).map_err(|e| e.to_string())
}

/// Preview proposed batch rename (lock-free)
pub fn preview_batch_rename_cmd(
    paths: Vec<String>,
    rule: BatchRenameRule,
) -> Result<Vec<RenamePreview>, String> {
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    BatchRenamer::preview_renames(&path_bufs, &rule).map_err(|e| e.to_string())
}

/// Execute atomic batch rename with rollback (lock-free)
pub fn execute_batch_rename_cmd(previews: Vec<RenamePreview>) -> Result<BatchRenameResult, String> {
    BatchRenamer::execute_batch_rename(&previews).map_err(|e| e.to_string())
}

/// Delete redundant or duplicate files permanently
pub fn delete_files_cmd(paths: Vec<String>) -> Result<crate::models::DeleteFilesResult, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    svc.delete_files(&path_bufs).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Phase 6 Command Endpoints
// ---------------------------------------------------------------------------

/// Extracts text and metadata from a document file (lock-free)
pub fn extract_document_cmd(path: String) -> Result<ExtractedDocument, String> {
    DocumentExtractor::extract_file(&path).map_err(|e| e.to_string())
}

/// Indexes a document file into Tantivy and vector embeddings
pub fn index_document_cmd(path: String) -> Result<ExtractedDocument, String> {
    let doc = DocumentExtractor::extract_file(&path).map_err(|e| e.to_string())?;
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
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

/// Discover and index all screenshots in a target directory
pub fn scan_screenshots_dir_cmd(path: String) -> Result<Vec<ScreenshotFingerprint>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.scan_screenshots_dir(&path).map_err(|e| e.to_string())
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilePreviewData {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub is_text: bool,
    pub is_image: bool,
    pub is_pdf: bool,
    pub is_media: bool,
    pub text_content: Option<String>,
    pub total_lines: Option<usize>,
    pub pdf_pages: Option<usize>,
    pub pdf_title: Option<String>,
    pub pdf_author: Option<String>,
    pub blake3_hash: Option<String>,
    pub modified_timestamp_secs: u64,
}

/// Instant file preview inspector (text, PDF, image, media, hash, metadata)
pub fn get_file_preview_cmd(path: String) -> Result<FilePreviewData, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() || !p.is_file() {
        return Err(format!("File does not exist: {}", path));
    }

    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let size_bytes = meta.len();
    let modified_timestamp_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let file_name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let image_exts = [
        "png", "jpg", "jpeg", "webp", "gif", "svg", "bmp", "ico", "tiff",
    ];
    let media_exts = [
        "mp4", "webm", "mov", "mkv", "avi", "mp3", "wav", "flac", "ogg", "m4a",
    ];
    let text_exts = [
        "rs",
        "ts",
        "js",
        "py",
        "json",
        "toml",
        "yaml",
        "yml",
        "md",
        "txt",
        "log",
        "csv",
        "tsv",
        "sh",
        "bat",
        "ps1",
        "html",
        "css",
        "svelte",
        "c",
        "cpp",
        "h",
        "go",
        "java",
        "sql",
        "xml",
        "lock",
        "ini",
        "env",
        "dockerfile",
        "gitignore",
    ];

    let is_image = image_exts.contains(&ext.as_str());
    let is_media = media_exts.contains(&ext.as_str());
    let is_pdf = ext == "pdf";
    let is_text = text_exts.contains(&ext.as_str())
        || (!is_image && !is_media && !is_pdf && size_bytes < 256 * 1024);

    let mut text_content = None;
    let mut total_lines = None;
    let mut pdf_pages = None;
    let mut pdf_title = None;
    let mut pdf_author = None;

    if is_pdf {
        if let Ok(extracted) = crate::doc_extract::DocumentExtractor::extract_file(p) {
            pdf_pages = extracted.metadata.page_count;
            pdf_title = extracted.metadata.title;
            pdf_author = extracted.metadata.author;
            let preview_chars: String = extracted.content_text.chars().take(8000).collect();
            text_content = Some(preview_chars);
            total_lines = Some(extracted.content_text.lines().count());
        }
    } else if is_text && size_bytes < 5 * 1024 * 1024 {
        if let Ok(bytes) = std::fs::read(p) {
            let sample_len = bytes.len().min(128 * 1024);
            let s = String::from_utf8_lossy(&bytes[..sample_len]).to_string();
            total_lines = Some(s.lines().count());
            text_content = Some(s);
        }
    }

    let blake3_hash = if size_bytes < 100 * 1024 * 1024 {
        std::fs::read(p)
            .ok()
            .map(|b| blake3::hash(&b).to_hex().to_string())
    } else {
        None
    };

    Ok(FilePreviewData {
        path,
        file_name,
        extension: ext,
        size_bytes,
        is_text,
        is_image,
        is_pdf,
        is_media,
        text_content,
        total_lines,
        pdf_pages,
        pdf_title,
        pdf_author,
        blake3_hash,
        modified_timestamp_secs,
    })
}

/// Export compliance findings to CSV or JSON file
pub fn export_compliance_report_cmd(
    target_dir: String,
    output_path: String,
    format: String,
) -> Result<usize, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let gate = shared_licensing::TierGate::enterprise();
    let summary = svc
        .scan_directory_compliance(&target_dir, &gate)
        .map_err(|e| e.to_string())?;
    svc.export_compliance_report(&summary, &output_path, &format)
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

fn parse_u64_val(val: Option<&Value>) -> Option<u64> {
    let v = val?;
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return u64::from_str_radix(hex, 16).ok();
        }
        return s.parse::<u64>().ok();
    }
    None
}

/// Dispatch desktop IPC commands from the Svelte 5 AI File Explorer frontend
pub fn dispatch_ipc_command(cmd: &str, args: Value) -> Result<Value, String> {
    match cmd {
        "list_directory_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let listing = list_directory_cmd(path)?;
            serde_json::to_value(listing).map_err(|e| e.to_string())
        }
        "get_file_preview_cmd" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing path parameter".to_string())?
                .to_string();
            let preview = get_file_preview_cmd(path)?;
            serde_json::to_value(preview).map_err(|e| e.to_string())
        }
        "list_drives_and_places_cmd" => {
            let entries = list_drives_and_places_cmd()?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "open_in_os_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let ok = open_in_os_cmd(path)?;
            Ok(json!({ "opened": ok }))
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
        "delete_files_cmd" => {
            let paths: Vec<String> = args
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let res = delete_files_cmd(paths)?;
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
        "scan_screenshots_dir_cmd" => {
            let path = resolve_valid_path(args.get("path").and_then(|v| v.as_str()));
            let res = scan_screenshots_dir_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "search_screenshots_by_hash_cmd" => {
            let target_hash =
                parse_u64_val(args.get("target_hash").or_else(|| args.get("targetHash")))
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
        "export_compliance_report_cmd" => {
            let target_dir = resolve_valid_path(
                args.get("target_dir")
                    .or_else(|| args.get("targetDir"))
                    .or_else(|| args.get("path"))
                    .and_then(|v| v.as_str()),
            );
            let output_path = args
                .get("output_path")
                .or_else(|| args.get("outputPath"))
                .and_then(|v| v.as_str())
                .unwrap_or("compliance_report.csv")
                .to_string();
            let format = args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("csv")
                .to_string();
            let count = export_compliance_report_cmd(target_dir, output_path, format)?;
            Ok(json!({ "exported_findings": count }))
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

        let treemap = get_treemap_cmd(path_str.clone(), 800.0, 600.0, 1).unwrap();
        assert_eq!(treemap.bounds.width, 800.0);

        // Test get_file_preview_cmd
        let test_file = dir.path().join("test_sample.rs");
        std::fs::write(&test_file, b"pub fn sample() { println!(\"Hello\"); }\n").unwrap();
        let preview = get_file_preview_cmd(test_file.to_string_lossy().to_string()).unwrap();
        assert!(preview.is_text);
        assert_eq!(preview.file_name, "test_sample.rs");
        assert_eq!(preview.extension, "rs");
        assert!(preview.text_content.unwrap().contains("pub fn sample"));
        assert!(preview.blake3_hash.is_some());
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

    #[test]
    fn test_commands_delete_files() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("dup_test.txt");
        std::fs::write(&file_path, b"duplicate content test").unwrap();
        let path_str = file_path.to_string_lossy().to_string();

        assert!(file_path.exists());
        let res = delete_files_cmd(vec![path_str.clone()]).unwrap();
        assert_eq!(res.deleted_count, 1);
        assert_eq!(res.reclaimed_bytes, 22);
        assert!(!file_path.exists());
    }

    #[test]
    fn test_commands_screenshot_indexing_and_search() {
        let dir = tempdir().unwrap();
        let gray = crate::ocr::render_text_to_gray_image("PAYLOAD", 80, 20);
        let bmp_bytes = crate::ocr::encode_gray_to_bmp(&gray);
        let img_path = dir.path().join("shot.bmp");
        std::fs::write(&img_path, bmp_bytes).unwrap();

        let dir_str = dir.path().to_string_lossy().to_string();
        let scanned = scan_screenshots_dir_cmd(dir_str).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].path, img_path);

        // Text search
        let text_res = search_screenshots_by_text_cmd("PAYLOAD".to_string(), 10).unwrap();
        assert_eq!(text_res.len(), 1);

        // Visual hash search
        let hash_res = search_screenshots_by_hash_cmd(scanned[0].dhash, 2, 10).unwrap();
        assert_eq!(hash_res.len(), 1);

        // Dispatch IPC verification
        let val = dispatch_ipc_command(
            "search_screenshots_by_text_cmd",
            serde_json::json!({ "query": "PAYLOAD", "limit": 5 }),
        )
        .unwrap();
        assert!(!val.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_commands_export_compliance_report() {
        let dir = tempdir().unwrap();
        let target_file = dir.path().join("secrets.txt");
        std::fs::write(
            &target_file,
            b"AKIAIOSFODNN7EXAMPLE\n-----BEGIN RSA PRIVATE KEY-----\n",
        )
        .unwrap();

        let out_csv = dir.path().join("report.csv");
        let dir_str = dir.path().to_string_lossy().to_string();
        let out_str = out_csv.to_string_lossy().to_string();

        let count = export_compliance_report_cmd(dir_str, out_str, "csv".to_string()).unwrap();
        assert!(count >= 2);
        assert!(out_csv.exists());

        let contents = std::fs::read_to_string(&out_csv).unwrap();
        assert!(contents.contains("AKIA"));
        assert!(contents.contains("PRIVATE KEY"));
    }

    #[test]
    fn test_commands_drives_and_no_verbatim_prefix() {
        let dir = tempdir().unwrap();
        let path_str = dir.path().to_string_lossy().to_string();
        let listing = list_directory_cmd(path_str).unwrap();
        assert!(!listing.current_path.to_string_lossy().starts_with(r"\\?\"));

        let places_val =
            dispatch_ipc_command("list_drives_and_places_cmd", serde_json::json!({})).unwrap();
        assert!(!places_val.as_array().unwrap().is_empty());
    }
}
