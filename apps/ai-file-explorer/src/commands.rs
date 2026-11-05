#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for AI File Explorer

use crate::analytics::StorageAnalytics;
use crate::batch_rename::BatchRenamer;
use crate::crawler::FileCrawler;
use crate::dedup::DuplicateDetector;
use crate::doc_extract::DocumentExtractor;
use crate::models::{
    BatchRenameResult, BatchRenameRule, ContentSearchQuery, ContentSearchResult, CopyFilesResult,
    DeduplicationSummary, DirectoryListing, ExtractedDocument, FileItem, FolderStats,
    MoveFilesResult, OcrResult, RenamePreview, ScreenshotFingerprint, ScreenshotSearchResult,
    SearchMode, SmartFolder, TreemapNode,
};
use crate::service::FileExplorerService;
use crate::treemap::TreemapGenerator;
use platform_common::{enumerate_drives_and_places, FsLocationEntry};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};

static SERVICE: OnceLock<Arc<Mutex<FileExplorerService>>> = OnceLock::new();
static BUILTIN_SMART_FOLDERS: OnceLock<Vec<SmartFolder>> = OnceLock::new();
static CUSTOM_SMART_FOLDERS: OnceLock<Mutex<Vec<SmartFolder>>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_licensing_manager() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("ai-file-explorer")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::AiFileExplorer, &data_dir)
    })
}

pub fn get_license_status_cmd() -> Result<LicenseStatusReport, String> {
    Ok(get_licensing_manager().status_report())
}

pub fn activate_license_cmd(license_json: &str) -> Result<LicenseStatusReport, String> {
    get_licensing_manager()
        .activate(license_json)
        .map_err(|e| e.to_string())
}

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

/// Helper to copy directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<(usize, u64)> {
    let mut count = 0;
    let mut bytes = 0;
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ty.is_dir() {
            let (sub_count, sub_bytes) = copy_dir_all(&entry.path(), &dest_child)?;
            count += sub_count;
            bytes += sub_bytes;
        } else {
            let b = std::fs::copy(entry.path(), dest_child)?;
            count += 1;
            bytes += b;
        }
    }
    Ok((count, bytes))
}

/// Copy files or directories to a target folder (Dual-Pane Commander Mode)
pub fn copy_files_cmd(
    sources: Vec<String>,
    destination_dir: String,
) -> Result<CopyFilesResult, String> {
    let dest = Path::new(&destination_dir);
    if !dest.is_dir() {
        return Err(format!(
            "Destination is not a directory: {}",
            destination_dir
        ));
    }
    let mut copied_count = 0;
    let mut total_bytes = 0;
    let mut failed_paths = Vec::new();

    for src_str in sources {
        let src = Path::new(&src_str);
        if !src.exists() {
            failed_paths.push(format!("{}: source not found", src_str));
            continue;
        }
        let file_name = match src.file_name() {
            Some(n) => n,
            None => {
                failed_paths.push(format!("{}: invalid filename", src_str));
                continue;
            }
        };
        let target_dest = dest.join(file_name);
        if src.is_dir() {
            match copy_dir_all(src, &target_dest) {
                Ok((c, b)) => {
                    copied_count += c;
                    total_bytes += b;
                }
                Err(e) => {
                    failed_paths.push(format!("{}: {}", src_str, e));
                }
            }
        } else {
            match std::fs::copy(src, &target_dest) {
                Ok(b) => {
                    copied_count += 1;
                    total_bytes += b;
                }
                Err(e) => {
                    failed_paths.push(format!("{}: {}", src_str, e));
                }
            }
        }
    }

    Ok(CopyFilesResult {
        copied_count,
        total_bytes,
        failed_paths,
    })
}

/// Move files or directories to a target folder (Dual-Pane Commander Mode)
pub fn move_files_cmd(
    sources: Vec<String>,
    destination_dir: String,
) -> Result<MoveFilesResult, String> {
    let dest = Path::new(&destination_dir);
    if !dest.is_dir() {
        return Err(format!(
            "Destination is not a directory: {}",
            destination_dir
        ));
    }
    let mut moved_count = 0;
    let mut failed_paths = Vec::new();

    for src_str in sources {
        let src = Path::new(&src_str);
        if !src.exists() {
            failed_paths.push(format!("{}: source not found", src_str));
            continue;
        }
        let file_name = match src.file_name() {
            Some(n) => n,
            None => {
                failed_paths.push(format!("{}: invalid filename", src_str));
                continue;
            }
        };
        let target_dest = dest.join(file_name);

        match std::fs::rename(src, &target_dest) {
            Ok(_) => {
                moved_count += 1;
            }
            Err(_) => {
                // Cross-device rename fallback: copy then remove original
                if src.is_dir() {
                    match copy_dir_all(src, &target_dest) {
                        Ok(_) => match std::fs::remove_dir_all(src) {
                            Ok(_) => {
                                moved_count += 1;
                            }
                            Err(e) => {
                                failed_paths.push(format!(
                                    "{}: copied but failed to delete original: {}",
                                    src_str, e
                                ));
                            }
                        },
                        Err(e) => {
                            failed_paths.push(format!("{}: {}", src_str, e));
                        }
                    }
                } else {
                    match std::fs::copy(src, &target_dest) {
                        Ok(_) => match std::fs::remove_file(src) {
                            Ok(_) => {
                                moved_count += 1;
                            }
                            Err(e) => {
                                failed_paths.push(format!(
                                    "{}: copied but failed to delete original: {}",
                                    src_str, e
                                ));
                            }
                        },
                        Err(e) => {
                            failed_paths.push(format!("{}: {}", src_str, e));
                        }
                    }
                }
            }
        }
    }

    Ok(MoveFilesResult {
        moved_count,
        failed_paths,
    })
}

/// Create a new subdirectory inside parent directory
pub fn create_directory_cmd(parent_dir: String, dir_name: String) -> Result<String, String> {
    let p = Path::new(&parent_dir);
    if !p.is_dir() {
        return Err(format!("Parent directory does not exist: {}", parent_dir));
    }
    let target = p.join(dir_name);
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Smart Virtual Folders
// ---------------------------------------------------------------------------

fn get_builtin_smart_folders() -> Vec<SmartFolder> {
    vec![
        SmartFolder {
            id: "smart-pdf".to_string(),
            name: "All PDF Documents".to_string(),
            icon: "📄".to_string(),
            description: "All indexed PDF manuals, books, and reports".to_string(),
            query: "pdf".to_string(),
            extensions: vec!["pdf".to_string()],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: true,
        },
        SmartFolder {
            id: "smart-docs".to_string(),
            name: "Word & Office Docs".to_string(),
            icon: "📝".to_string(),
            description: "Word, Excel, PowerPoint, and Markdown files".to_string(),
            query: "docx doc odt xlsx pptx md txt".to_string(),
            extensions: vec![
                "docx".to_string(),
                "doc".to_string(),
                "odt".to_string(),
                "xlsx".to_string(),
                "pptx".to_string(),
                "md".to_string(),
                "txt".to_string(),
            ],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: true,
        },
        SmartFolder {
            id: "smart-images".to_string(),
            name: "Images & Screenshots".to_string(),
            icon: "🖼️".to_string(),
            description: "Photos, screenshots, and vector graphics".to_string(),
            query: "png jpg jpeg webp bmp gif svg".to_string(),
            extensions: vec![
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "webp".to_string(),
                "bmp".to_string(),
                "gif".to_string(),
                "svg".to_string(),
            ],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: true,
        },
        SmartFolder {
            id: "smart-archives".to_string(),
            name: "Archives & Packages".to_string(),
            icon: "📦".to_string(),
            description: "ZIP, TAR, GZ, 7Z, and RAR compressed packages".to_string(),
            query: "zip tar gz tgz 7z rar".to_string(),
            extensions: vec![
                "zip".to_string(),
                "tar".to_string(),
                "gz".to_string(),
                "tgz".to_string(),
                "7z".to_string(),
                "rar".to_string(),
            ],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: true,
        },
        SmartFolder {
            id: "smart-code".to_string(),
            name: "Source Code & Scripts".to_string(),
            icon: "💻".to_string(),
            description: "Rust, TypeScript, Python, Go, C++, JSON, YAML source files".to_string(),
            query: "rs ts js py go c cpp java json toml yaml".to_string(),
            extensions: vec![
                "rs".to_string(),
                "ts".to_string(),
                "js".to_string(),
                "py".to_string(),
                "go".to_string(),
                "c".to_string(),
                "cpp".to_string(),
                "java".to_string(),
                "json".to_string(),
                "toml".to_string(),
                "yaml".to_string(),
            ],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: true,
        },
        SmartFolder {
            id: "smart-large".to_string(),
            name: "Large Files (>100MB)".to_string(),
            icon: "⚡".to_string(),
            description: "High disk-consumption files over 100 MB".to_string(),
            query: "*".to_string(),
            extensions: vec![],
            min_size_bytes: Some(100 * 1024 * 1024),
            max_size_bytes: None,
            is_builtin: true,
        },
        SmartFolder {
            id: "smart-recent".to_string(),
            name: "Recent Files (24 Hours)".to_string(),
            icon: "🕒".to_string(),
            description: "Files created or modified in the last 24 hours".to_string(),
            query: "*".to_string(),
            extensions: vec![],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: true,
        },
    ]
}

/// Retrieve all built-in and user-saved Smart Virtual Folders
pub fn get_smart_folders_cmd() -> Result<Vec<SmartFolder>, String> {
    let mut list = BUILTIN_SMART_FOLDERS
        .get_or_init(get_builtin_smart_folders)
        .clone();
    let custom = CUSTOM_SMART_FOLDERS.get_or_init(|| Mutex::new(Vec::new()));
    let customs = custom.lock().map_err(|e| e.to_string())?;
    list.extend(customs.clone());
    Ok(list)
}

/// Save a new or updated custom Smart Virtual Folder
pub fn save_smart_folder_cmd(mut folder: SmartFolder) -> Result<SmartFolder, String> {
    if folder.id.is_empty() {
        folder.id = format!(
            "custom-smart-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
    }
    folder.is_builtin = false;
    let custom = CUSTOM_SMART_FOLDERS.get_or_init(|| Mutex::new(Vec::new()));
    let mut customs = custom.lock().map_err(|e| e.to_string())?;
    if let Some(pos) = customs.iter().position(|f| f.id == folder.id) {
        customs[pos] = folder.clone();
    } else {
        customs.push(folder.clone());
    }
    Ok(folder)
}

/// Delete a custom Smart Virtual Folder by ID
pub fn delete_smart_folder_cmd(id: String) -> Result<bool, String> {
    let custom = CUSTOM_SMART_FOLDERS.get_or_init(|| Mutex::new(Vec::new()));
    let mut customs = custom.lock().map_err(|e| e.to_string())?;
    let orig_len = customs.len();
    customs.retain(|f| f.id != id);
    Ok(customs.len() < orig_len)
}

#[allow(clippy::too_many_arguments)]
fn find_matching_files_in_dir(
    dir: &Path,
    folder: &SmartFolder,
    depth: usize,
    max_depth: usize,
    now_secs: u64,
    results: &mut Vec<FileItem>,
    seen: &mut std::collections::HashSet<PathBuf>,
    limit: usize,
) {
    if depth > max_depth || results.len() >= limit {
        return;
    }
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for entry in read_dir.flatten() {
        if results.len() >= limit {
            break;
        }
        let p = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "node_modules" && name != "target" {
                find_matching_files_in_dir(
                    &p,
                    folder,
                    depth + 1,
                    max_depth,
                    now_secs,
                    results,
                    seen,
                    limit,
                );
            }
        } else {
            let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase());
            let ext_match = folder.extensions.is_empty()
                || ext
                    .as_ref()
                    .map(|e| {
                        folder
                            .extensions
                            .iter()
                            .any(|fe| fe.eq_ignore_ascii_case(e))
                    })
                    .unwrap_or(false);
            let min_match = folder
                .min_size_bytes
                .map(|min| meta.len() >= min)
                .unwrap_or(true);
            let max_match = folder
                .max_size_bytes
                .map(|max| meta.len() <= max)
                .unwrap_or(true);

            let mod_secs = meta
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let recent_match = if folder.id == "smart-recent" {
                now_secs.saturating_sub(mod_secs) <= 86400
            } else {
                true
            };

            if ext_match && min_match && max_match && recent_match && seen.insert(p.clone()) {
                let file_name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                results.push(FileItem {
                    path: p,
                    file_name,
                    extension: ext,
                    size_bytes: meta.len(),
                    is_dir: false,
                    modified_timestamp_secs: mod_secs,
                    is_readonly: meta.permissions().readonly(),
                });
            }
        }
    }
}

/// Execute dynamic query for a Smart Virtual Folder across directory context and Tantivy index
pub fn query_smart_folder_cmd(
    id: String,
    path_context: Option<String>,
    limit: usize,
) -> Result<Vec<FileItem>, String> {
    let folders = get_smart_folders_cmd()?;
    let folder = folders
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| format!("Smart folder not found: {}", id))?;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut matches = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    // 1. Scan directory context if specified
    if let Some(ctx_str) = path_context {
        let p = Path::new(&ctx_str);
        if p.is_dir() {
            find_matching_files_in_dir(
                p,
                &folder,
                0,
                3,
                now_secs,
                &mut matches,
                &mut seen_paths,
                limit,
            );
        }
    }

    // 2. Query Tantivy index
    let service = get_service();
    if let Ok(svc) = service.lock() {
        for ext in &folder.extensions {
            if let Ok(hits) = svc.search(ext, limit) {
                for item in hits {
                    if item.is_dir {
                        continue;
                    }
                    let ext_match = item
                        .extension
                        .as_ref()
                        .map(|e| e.eq_ignore_ascii_case(ext))
                        .unwrap_or(false);
                    let min_match = folder
                        .min_size_bytes
                        .map(|min| item.size_bytes >= min)
                        .unwrap_or(true);
                    let max_match = folder
                        .max_size_bytes
                        .map(|max| item.size_bytes <= max)
                        .unwrap_or(true);
                    if ext_match && min_match && max_match && seen_paths.insert(item.path.clone()) {
                        matches.push(item);
                        if matches.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if matches.len() >= limit {
                break;
            }
        }

        if folder.extensions.is_empty() && matches.len() < limit {
            if let Ok(hits) = svc.search(&folder.query, limit) {
                for item in hits {
                    if item.is_dir {
                        continue;
                    }
                    let min_match = folder
                        .min_size_bytes
                        .map(|min| item.size_bytes >= min)
                        .unwrap_or(true);
                    let max_match = folder
                        .max_size_bytes
                        .map(|max| item.size_bytes <= max)
                        .unwrap_or(true);
                    let recent_match = if folder.id == "smart-recent" {
                        now_secs.saturating_sub(item.modified_timestamp_secs) <= 86400
                    } else {
                        true
                    };
                    if min_match
                        && max_match
                        && recent_match
                        && seen_paths.insert(item.path.clone())
                    {
                        matches.push(item);
                        if matches.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
    }

    matches.sort_by_key(|b| std::cmp::Reverse(b.modified_timestamp_secs));
    if matches.len() > limit {
        matches.truncate(limit);
    }

    Ok(matches)
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
pub struct ArchiveEntryInfo {
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
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
    pub is_archive: bool,
    pub archive_entries: Vec<ArchiveEntryInfo>,
    pub text_content: Option<String>,
    pub total_lines: Option<usize>,
    pub pdf_pages: Option<usize>,
    pub pdf_title: Option<String>,
    pub pdf_author: Option<String>,
    pub blake3_hash: Option<String>,
    pub modified_timestamp_secs: u64,
}

/// Compute 256-bit BLAKE3 hash via a 64KB stack buffer stream (O(1) memory for any size)
pub fn compute_file_hash_cmd(path: String) -> Result<String, String> {
    use std::io::Read;
    let p = std::path::Path::new(&path);
    if !p.exists() || !p.is_file() {
        return Err(format!("File does not exist: {}", path));
    }
    let mut file = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Fast streaming extraction of archive entries without extracting contents to disk
pub fn inspect_archive_cmd(path: String) -> Result<Vec<ArchiveEntryInfo>, String> {
    use std::io::Read;
    let p = std::path::Path::new(&path);
    if !p.exists() || !p.is_file() {
        return Err(format!("Archive file does not exist: {}", path));
    }
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut entries = Vec::new();

    if ext == "zip" {
        if let Ok(mut file) = std::fs::File::open(p) {
            let mut buf = Vec::new();
            let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
            if meta.len() <= 20 * 1024 * 1024 {
                let _ = file.read_to_end(&mut buf);
            } else {
                let sample_len = 2 * 1024 * 1024;
                buf.resize(sample_len, 0);
                if let Ok(n) = file.read(&mut buf) {
                    buf.truncate(n);
                }
            }
            if let Ok(zip_entries) = crate::doc_extract::parse_zip_entries(&buf) {
                for e in zip_entries.into_iter().take(200) {
                    let is_dir = e.name.ends_with('/');
                    entries.push(ArchiveEntryInfo {
                        size_bytes: e.data.len() as u64,
                        is_dir,
                        name: e.name,
                    });
                }
            }
        }
    } else if ext == "tar" || ext == "gz" || ext == "tgz" {
        if let Ok(mut file) = std::fs::File::open(p) {
            let mut sample_buf = Vec::new();
            if ext == "tar" {
                sample_buf.resize(512 * 120, 0);
                if let Ok(n) = file.read(&mut sample_buf) {
                    sample_buf.truncate(n);
                }
            } else {
                let gz = flate2::read::GzDecoder::new(file);
                let mut limited = gz.take(512 * 120);
                let _ = limited.read_to_end(&mut sample_buf);
            }
            let mut cursor = 0;
            while cursor + 512 <= sample_buf.len() && entries.len() < 200 {
                let header = &sample_buf[cursor..cursor + 512];
                if header.iter().all(|&b| b == 0) {
                    break;
                }
                let name_end = header[..100].iter().position(|&b| b == 0).unwrap_or(100);
                let name = String::from_utf8_lossy(&header[..name_end])
                    .trim()
                    .to_string();
                let size_str = String::from_utf8_lossy(&header[124..136])
                    .trim()
                    .to_string();
                let size = u64::from_str_radix(size_str.trim_matches('\0'), 8).unwrap_or(0);
                let is_dir = name.ends_with('/') || header[156] == b'5';
                if !name.is_empty() {
                    entries.push(ArchiveEntryInfo {
                        name,
                        size_bytes: size,
                        is_dir,
                    });
                }
                let blocks = size.div_ceil(512);
                cursor += 512 + (blocks as usize * 512);
            }
        }
    }

    Ok(entries)
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
    let archive_exts = ["zip", "tar", "gz", "tgz", "bz2", "7z", "rar"];
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
    let is_archive = archive_exts.contains(&ext.as_str());
    let is_pdf = ext == "pdf";
    let is_text = text_exts.contains(&ext.as_str())
        || (!is_image && !is_media && !is_pdf && !is_archive && size_bytes < 256 * 1024);

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

    let archive_entries = if is_archive {
        inspect_archive_cmd(path.clone()).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Synchronously hash only small files <= 512KB for sub-5ms instant preview;
    // larger files compute hash on-demand via compute_file_hash_cmd
    let blake3_hash = if size_bytes <= 512 * 1024 {
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
        is_archive,
        archive_entries,
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
        "compute_file_hash_cmd" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing path parameter".to_string())?
                .to_string();
            let hash = compute_file_hash_cmd(path)?;
            serde_json::to_value(hash).map_err(|e| e.to_string())
        }
        "inspect_archive_cmd" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing path parameter".to_string())?
                .to_string();
            let entries = inspect_archive_cmd(path)?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
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
        "copy_files_cmd" => {
            let sources: Vec<String> = args
                .get("sources")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let destination_dir = resolve_valid_path(
                args.get("destination_dir")
                    .or_else(|| args.get("destinationDir"))
                    .and_then(|v| v.as_str()),
            );
            let res = copy_files_cmd(sources, destination_dir)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "move_files_cmd" => {
            let sources: Vec<String> = args
                .get("sources")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let destination_dir = resolve_valid_path(
                args.get("destination_dir")
                    .or_else(|| args.get("destinationDir"))
                    .and_then(|v| v.as_str()),
            );
            let res = move_files_cmd(sources, destination_dir)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "create_directory_cmd" => {
            let parent_dir = resolve_valid_path(
                args.get("parent_dir")
                    .or_else(|| args.get("parentDir"))
                    .and_then(|v| v.as_str()),
            );
            let dir_name = args
                .get("dir_name")
                .or_else(|| args.get("dirName"))
                .or_else(|| args.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("New Folder")
                .to_string();
            let res = create_directory_cmd(parent_dir, dir_name)?;
            Ok(json!({ "created_path": res }))
        }
        "get_smart_folders_cmd" => {
            let res = get_smart_folders_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "save_smart_folder_cmd" => {
            let folder_val = args
                .get("folder")
                .cloned()
                .ok_or_else(|| "Missing folder argument".to_string())?;
            let folder: SmartFolder =
                serde_json::from_value(folder_val).map_err(|e| e.to_string())?;
            let res = save_smart_folder_cmd(folder)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "delete_smart_folder_cmd" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let res = delete_smart_folder_cmd(id)?;
            Ok(json!({ "deleted": res }))
        }
        "query_smart_folder_cmd" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("smart-pdf")
                .to_string();
            let path_context = args
                .get("path_context")
                .or_else(|| args.get("pathContext"))
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
            let res = query_smart_folder_cmd(id, path_context, limit)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "get_license_status_cmd" | "get_license_status" => {
            let status = get_license_status_cmd()?;
            serde_json::to_value(status).map_err(|e| e.to_string())
        }
        "activate_license_cmd" | "activate_license" => {
            let license_json = args
                .get("license_json")
                .or_else(|| args.get("licenseJson"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing license_json parameter".to_string())?;
            let status = activate_license_cmd(license_json)?;
            serde_json::to_value(status).map_err(|e| e.to_string())
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

    #[test]
    fn test_commands_hash_and_archive_inspection() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("hash_sample.bin");
        std::fs::write(&file_path, b"abcdefghijklmnopqrstuvwxyz0123456789").unwrap();
        let file_str = file_path.to_string_lossy().to_string();

        // 1. compute_file_hash_cmd
        let hash = compute_file_hash_cmd(file_str.clone()).expect("Hashing should succeed");
        assert_eq!(hash.len(), 64);

        // IPC dispatch check
        let hash_val = dispatch_ipc_command(
            "compute_file_hash_cmd",
            serde_json::json!({ "path": file_str }),
        )
        .expect("IPC dispatch compute_file_hash_cmd should succeed");
        assert_eq!(hash_val.as_str().unwrap(), hash);

        // 2. inspect_archive_cmd on zip
        let zip_path = dir.path().join("test_pkg.zip");
        let mut zip_bytes = Vec::new();
        let file_data = b"inner content test";
        zip_bytes.extend_from_slice(b"PK\x03\x04");
        zip_bytes.extend_from_slice(&[20, 0]); // ver 2.0
        zip_bytes.extend_from_slice(&[0, 0]); // flags
        zip_bytes.extend_from_slice(&[0, 0]); // method 0 (store)
        zip_bytes.extend_from_slice(&[0, 0, 0, 0]); // time
        zip_bytes.extend_from_slice(&[0, 0, 0, 0]); // crc
        zip_bytes.extend_from_slice(&(file_data.len() as u32).to_le_bytes()); // comp
        zip_bytes.extend_from_slice(&(file_data.len() as u32).to_le_bytes()); // uncomp
        let inner_name = b"inner_file.txt";
        zip_bytes.extend_from_slice(&(inner_name.len() as u16).to_le_bytes());
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // extra
        zip_bytes.extend_from_slice(inner_name);
        zip_bytes.extend_from_slice(file_data);
        std::fs::write(&zip_path, zip_bytes).unwrap();

        let zip_str = zip_path.to_string_lossy().to_string();
        let entries = inspect_archive_cmd(zip_str.clone()).expect("Zip inspection should succeed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "inner_file.txt");
        assert_eq!(entries[0].size_bytes, file_data.len() as u64);

        // Preview command should also mark is_archive = true
        let preview = get_file_preview_cmd(zip_str).expect("Preview should succeed");
        assert!(preview.is_archive);
        assert_eq!(preview.archive_entries.len(), 1);
    }

    #[test]
    fn test_commands_commander_operations() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src_folder");
        let dst_dir = dir.path().join("dst_folder");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();

        let file1 = src_dir.join("item1.txt");
        std::fs::write(&file1, b"commander copy test").unwrap();

        let src_str = file1.to_string_lossy().to_string();
        let dst_str = dst_dir.to_string_lossy().to_string();

        // 1. Copy file
        let copy_res = copy_files_cmd(vec![src_str.clone()], dst_str.clone()).unwrap();
        assert_eq!(copy_res.copied_count, 1);
        assert_eq!(copy_res.total_bytes, 19);
        assert!(dst_dir.join("item1.txt").exists());
        assert!(file1.exists());

        // 2. Move file
        let move_target = dst_dir.join("sub_target");
        std::fs::create_dir_all(&move_target).unwrap();
        let move_target_str = move_target.to_string_lossy().to_string();

        let move_res = move_files_cmd(vec![src_str], move_target_str).unwrap();
        assert_eq!(move_res.moved_count, 1);
        assert!(!file1.exists());
        assert!(move_target.join("item1.txt").exists());

        // 3. Create directory
        let created = create_directory_cmd(dst_str.clone(), "new_subfolder".to_string()).unwrap();
        assert!(std::path::Path::new(&created).is_dir());

        // 4. Test IPC dispatch
        let ipc_copy_val = dispatch_ipc_command(
            "copy_files_cmd",
            serde_json::json!({
                "sources": [dst_dir.join("item1.txt").to_string_lossy().to_string()],
                "destination_dir": created,
            }),
        )
        .unwrap();
        assert_eq!(
            ipc_copy_val.get("copied_count").unwrap().as_u64().unwrap(),
            1
        );
    }

    #[test]
    fn test_commands_smart_folders() {
        let dir = tempdir().unwrap();
        let dir_str = dir.path().to_string_lossy().to_string();

        // Create mock PDF and doc files
        let pdf_file = dir.path().join("manual.pdf");
        std::fs::write(&pdf_file, b"%PDF-1.4 test pdf content").unwrap();

        let code_file = dir.path().join("main.rs");
        std::fs::write(&code_file, b"fn main() {}").unwrap();

        // 1. Get built-in smart folders
        let folders = get_smart_folders_cmd().unwrap();
        assert!(folders.iter().any(|f| f.id == "smart-pdf"));
        assert!(folders.iter().any(|f| f.id == "smart-code"));

        // 2. Query smart-pdf with path context
        let pdf_matches =
            query_smart_folder_cmd("smart-pdf".to_string(), Some(dir_str.clone()), 20).unwrap();
        assert_eq!(pdf_matches.len(), 1);
        assert_eq!(pdf_matches[0].file_name, "manual.pdf");

        // 3. Query smart-code with path context
        let code_matches =
            query_smart_folder_cmd("smart-code".to_string(), Some(dir_str), 20).unwrap();
        assert_eq!(code_matches.len(), 1);
        assert_eq!(code_matches[0].file_name, "main.rs");

        // 4. Save and delete custom smart folder
        let custom_folder = SmartFolder {
            id: "".to_string(),
            name: "My Custom Folder".to_string(),
            icon: "⭐".to_string(),
            description: "Custom test folder".to_string(),
            query: "test".to_string(),
            extensions: vec!["pdf".to_string()],
            min_size_bytes: None,
            max_size_bytes: None,
            is_builtin: false,
        };
        let saved = save_smart_folder_cmd(custom_folder).unwrap();
        assert!(!saved.id.is_empty());

        let all_folders = get_smart_folders_cmd().unwrap();
        assert!(all_folders.iter().any(|f| f.id == saved.id));

        let deleted = delete_smart_folder_cmd(saved.id.clone()).unwrap();
        assert!(deleted);
    }

    #[test]
    fn test_licensing_ipc_commands() {
        let status_val = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(status_val["tier"] == "Free" || status_val["tier"] == "Premium" || status_val["tier"] == "Business");

        // Mint a valid license
        let auth = shared_licensing::LicenseAuthority::official();
        let cert = auth.sign_license(shared_licensing::LicenseParameters {
            license_id: "AFE-PRO-100".to_string(),
            licensee_name: "Explorer User".to_string(),
            licensee_email: "explorer@test.com".to_string(),
            tier: shared_licensing::LicenseTier::Premium,
            app_id: shared_licensing::AppId::AiFileExplorer,
            issued_timestamp_secs: 1700000000,
            expiry_timestamp_secs: None,
            max_seats: 1,
            features: vec!["unlimited_ocr".to_string()],
        });
        let cert_json = serde_json::to_string(&cert).unwrap();

        // Activate valid license
        let act_val = dispatch_ipc_command("activate_license", json!({ "license_json": cert_json })).unwrap();
        assert_eq!(act_val["tier"], "Premium");
        assert_eq!(act_val["is_active"], true);
        assert_eq!(act_val["licensee_name"], "Explorer User");

        // Forged license rejection
        let mut forged_cert = cert.clone();
        forged_cert.licensee_name = "Hacker".to_string();
        let forged_json = serde_json::to_string(&forged_cert).unwrap();
        let err = dispatch_ipc_command("activate_license", json!({ "license_json": forged_json })).unwrap_err();
        assert!(err.contains("signature") || err.contains("InvalidSignature"));
    }
}
