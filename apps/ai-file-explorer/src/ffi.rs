//! AI File Explorer - Reusable C-ABI Foreign Function Interface (FFI) for Desktop & Mobile
//!
//! Conforms to ADR-0015 for cross-platform desktop and mobile integration.
//! Exposes `afe_*` C-ABI exports for Flutter mobile (Android/iOS) and Tauri desktop IPC.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::models::{BatchRenameRule, ContentSearchQuery, RenamePreview};
use crate::service::FileExplorerService;
use shared_licensing::{LicenseTier, TierGate};

/// Opaque wrapper for `FileExplorerService` across C-ABI boundary
pub struct AfeHandle {
    service: FileExplorerService,
    _rt: Arc<Runtime>,
}

/// Helper converting Rust serialize type to raw C string pointer
fn to_c_string<T: serde::Serialize>(val: &T) -> *mut c_char {
    let json = serde_json::to_string(val).unwrap_or_else(|_| "{}".to_string());
    CString::new(json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Helper converting error string to raw C string JSON pointer
fn to_error_json(err_msg: &str) -> *mut c_char {
    let err_json = format!("{{\"error\": \"{}\"}}", err_msg.replace('"', "\\\""));
    CString::new(err_json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Create a new `AfeHandle` instance
///
/// # Safety
/// `data_dir` must be a valid null-terminated C string or null pointer.
#[no_mangle]
pub unsafe extern "C" fn afe_create_handle(data_dir: *const c_char) -> *mut AfeHandle {
    let rt = match Runtime::new() {
        Ok(r) => Arc::new(r),
        Err(_) => return std::ptr::null_mut(),
    };

    let service = if data_dir.is_null() {
        match FileExplorerService::new() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    } else {
        // SAFETY: data_dir is non-null
        let dir_str = match CStr::from_ptr(data_dir).to_str() {
            Ok(s) if !s.trim().is_empty() => s,
            _ => {
                return match FileExplorerService::new() {
                    Ok(s) => Box::into_raw(Box::new(AfeHandle {
                        service: s,
                        _rt: rt,
                    })),
                    Err(_) => std::ptr::null_mut(),
                };
            }
        };

        match FileExplorerService::open_persistent(dir_str) {
            Ok(s) => s,
            Err(_) => match FileExplorerService::new() {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            },
        }
    };

    Box::into_raw(Box::new(AfeHandle { service, _rt: rt }))
}

/// Free an `AfeHandle`
///
/// # Safety
/// `handle` must be a valid non-null pointer returned by `afe_create_handle` and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn afe_free_handle(handle: *mut AfeHandle) {
    if !handle.is_null() {
        // SAFETY: Pointer is non-null and was allocated by Box::into_raw
        let _ = Box::from_raw(handle);
    }
}

/// Free a string returned by any `afe_*` function
///
/// # Safety
/// `s` must have been returned by an `afe_*` function and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn afe_free_string(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: Pointer was returned by CString::into_raw
        let _ = CString::from_raw(s);
    }
}

/// List directory contents and return DirectoryListing JSON
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_list_directory(
    handle: *mut AfeHandle,
    path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to afe_list_directory");
    }

    // SAFETY: path is non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in directory path"),
    };

    let h = &*handle;
    match h.service.list_directory(path_str) {
        Ok(listing) => to_c_string(&listing),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Search indexed files by query string returning Vec<FileItem> JSON
///
/// # Safety
/// `handle` and `query` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_search_files(
    handle: *mut AfeHandle,
    query: *const c_char,
    limit: usize,
) -> *mut c_char {
    if handle.is_null() || query.is_null() {
        return to_error_json("Null pointer passed to afe_search_files");
    }

    // SAFETY: query is non-null
    let query_str = match CStr::from_ptr(query).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in search query"),
    };

    let h = &*handle;
    match h.service.search(query_str, limit) {
        Ok(items) => to_c_string(&items),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Index a target directory into the Tantivy search index
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_index_directory(
    handle: *mut AfeHandle,
    path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to afe_index_directory");
    }

    // SAFETY: path is non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in directory path"),
    };

    let h = &*handle;
    match h.service.list_directory(path_str) {
        Ok(listing) => match h.service.index_files(&listing.items) {
            Ok(count) => {
                let res = serde_json::json!({ "indexed_count": count });
                to_c_string(&res)
            }
            Err(e) => to_error_json(&e.to_string()),
        },
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Scan for duplicate files across search roots using 3-tier filtration
///
/// # Safety
/// `handle` and `roots_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_scan_duplicates(
    handle: *mut AfeHandle,
    roots_json: *const c_char,
    min_size_bytes: u64,
) -> *mut c_char {
    if handle.is_null() || roots_json.is_null() {
        return to_error_json("Null pointer passed to afe_scan_duplicates");
    }

    // SAFETY: roots_json is non-null
    let roots_str = match CStr::from_ptr(roots_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in roots JSON"),
    };

    let roots: Vec<String> = match serde_json::from_str(roots_str) {
        Ok(r) => r,
        Err(e) => return to_error_json(&format!("Failed to parse roots JSON: {}", e)),
    };

    let path_roots: Vec<PathBuf> = roots.into_iter().map(PathBuf::from).collect();
    let h = &*handle;
    match h.service.scan_duplicates(&path_roots, min_size_bytes) {
        Ok(summary) => to_c_string(&summary),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Compute storage analytics breakdown and return FolderStats JSON
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_analyze_storage(
    handle: *mut AfeHandle,
    path: *const c_char,
    top_limit: usize,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to afe_analyze_storage");
    }

    // SAFETY: path is non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in path"),
    };

    let h = &*handle;
    match h.service.analyze_storage(path_str, top_limit) {
        Ok(stats) => to_c_string(&stats),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Generate squarified treemap bounding rectangles and return TreemapNode JSON
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_generate_treemap(
    handle: *mut AfeHandle,
    path: *const c_char,
    width: f64,
    height: f64,
    max_depth: usize,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to afe_generate_treemap");
    }

    // SAFETY: path is non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in path"),
    };

    let h = &*handle;
    match h
        .service
        .generate_treemap(path_str, width, height, max_depth)
    {
        Ok(node) => to_c_string(&node),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Preview proposed batch rename and return Vec<RenamePreview> JSON
///
/// # Safety
/// `handle`, `paths_json`, and `rule_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_preview_batch_rename(
    handle: *mut AfeHandle,
    paths_json: *const c_char,
    rule_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || paths_json.is_null() || rule_json.is_null() {
        return to_error_json("Null pointer passed to afe_preview_batch_rename");
    }

    // SAFETY: pointers are non-null
    let paths_str = match CStr::from_ptr(paths_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in paths JSON"),
    };
    let rule_str = match CStr::from_ptr(rule_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in rule JSON"),
    };

    let paths: Vec<PathBuf> = match serde_json::from_str::<Vec<String>>(paths_str) {
        Ok(p) => p.into_iter().map(PathBuf::from).collect(),
        Err(e) => return to_error_json(&format!("Failed to parse paths JSON: {}", e)),
    };

    let rule: BatchRenameRule = match serde_json::from_str(rule_str) {
        Ok(r) => r,
        Err(e) => return to_error_json(&format!("Failed to parse rule JSON: {}", e)),
    };

    let h = &*handle;
    match h.service.preview_batch_rename(&paths, &rule) {
        Ok(previews) => to_c_string(&previews),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Execute atomic batch rename and return BatchRenameResult JSON
///
/// # Safety
/// `handle` and `previews_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_execute_batch_rename(
    handle: *mut AfeHandle,
    previews_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || previews_json.is_null() {
        return to_error_json("Null pointer passed to afe_execute_batch_rename");
    }

    // SAFETY: previews_json is non-null
    let previews_str = match CStr::from_ptr(previews_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in previews JSON"),
    };

    let previews: Vec<RenamePreview> = match serde_json::from_str(previews_str) {
        Ok(p) => p,
        Err(e) => return to_error_json(&format!("Failed to parse previews JSON: {}", e)),
    };

    let h = &*handle;
    match h.service.execute_batch_rename(&previews) {
        Ok(result) => to_c_string(&result),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Extract text and metadata from a document file and return ExtractedDocument JSON
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_extract_document(
    handle: *mut AfeHandle,
    path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to afe_extract_document");
    }

    // SAFETY: path is non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in document path"),
    };

    let h = &*handle;
    match h.service.extract_document(path_str) {
        Ok(doc) => {
            // Index into search engines
            let _ = h.service.index_extracted_document(&doc);
            to_c_string(&doc)
        }
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Execute keyword, semantic, or combined hybrid search and return ContentSearchResult JSON
///
/// # Safety
/// `handle` and `query_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_search_content(
    handle: *mut AfeHandle,
    query_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || query_json.is_null() {
        return to_error_json("Null pointer passed to afe_search_content");
    }

    // SAFETY: query_json is non-null
    let query_str = match CStr::from_ptr(query_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in search query JSON"),
    };

    let query: ContentSearchQuery = match serde_json::from_str(query_str) {
        Ok(q) => q,
        Err(e) => return to_error_json(&format!("Failed to parse ContentSearchQuery: {}", e)),
    };

    let h = &*handle;
    match h.service.search_content(&query) {
        Ok(result) => to_c_string(&result),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Perform offline OCR on an image file and return OcrResult JSON
///
/// # Safety
/// `handle` and `image_path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_ocr_image(
    handle: *mut AfeHandle,
    image_path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || image_path.is_null() {
        return to_error_json("Null pointer passed to afe_ocr_image");
    }

    // SAFETY: image_path is non-null
    let path_str = match CStr::from_ptr(image_path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in image path"),
    };

    let h = &*handle;
    match h.service.ocr_image(path_str) {
        Ok(res) => to_c_string(&res),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Index a screenshot image computing dHash and OCR text, returning ScreenshotFingerprint JSON
///
/// # Safety
/// `handle` and `image_path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_index_screenshot(
    handle: *mut AfeHandle,
    image_path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || image_path.is_null() {
        return to_error_json("Null pointer passed to afe_index_screenshot");
    }

    // SAFETY: image_path is non-null
    let path_str = match CStr::from_ptr(image_path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in image path"),
    };

    let h = &*handle;
    match h.service.index_screenshot(path_str) {
        Ok(fp) => to_c_string(&fp),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Search indexed screenshots by visual 64-bit dHash similarity
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn afe_search_screenshots_by_hash(
    handle: *mut AfeHandle,
    target_hash: u64,
    max_distance: u32,
    limit: usize,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null pointer passed to afe_search_screenshots_by_hash");
    }

    let h = &*handle;
    let results = h
        .service
        .search_screenshots_by_hash(target_hash, max_distance, limit);
    to_c_string(&results)
}

/// Search indexed screenshots by OCR text keywords
///
/// # Safety
/// `handle` and `query` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn afe_search_screenshots_by_text(
    handle: *mut AfeHandle,
    query: *const c_char,
    limit: usize,
) -> *mut c_char {
    if handle.is_null() || query.is_null() {
        return to_error_json("Null pointer passed to afe_search_screenshots_by_text");
    }

    // SAFETY: query is non-null
    let query_str = match CStr::from_ptr(query).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in query"),
    };

    let h = &*handle;
    let results = h.service.search_screenshots_by_text(query_str, limit);
    to_c_string(&results)
}

/// Scan a file or directory for PII and sensitive data compliance
///
/// If `path` is a directory, returns `ComplianceScanSummary` JSON.
/// If `path` is a file, returns `Vec<SensitiveFinding>` JSON.
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers. `tier` can be null (defaults to Business).
#[no_mangle]
pub unsafe extern "C" fn afe_scan_compliance(
    handle: *mut AfeHandle,
    path: *const c_char,
    tier: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to afe_scan_compliance");
    }

    // SAFETY: path is non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in path"),
    };

    let gate_tier = if tier.is_null() {
        LicenseTier::Business
    } else {
        // SAFETY: tier is non-null
        match CStr::from_ptr(tier).to_str() {
            Ok(s) if s.eq_ignore_ascii_case("free") => LicenseTier::Free,
            Ok(s) if s.eq_ignore_ascii_case("premium") => LicenseTier::Premium,
            _ => LicenseTier::Business,
        }
    };
    let gate = TierGate::for_tier(gate_tier);

    let target = Path::new(path_str);
    let h = &*handle;

    if target.is_dir() {
        match h.service.scan_directory_compliance(target, &gate) {
            Ok(summary) => to_c_string(&summary),
            Err(e) => to_error_json(&e.to_string()),
        }
    } else {
        match h.service.scan_file_compliance(target, &gate) {
            Ok(findings) => to_c_string(&findings),
            Err(e) => to_error_json(&e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_ffi_lifecycle_and_core_operations() {
        unsafe {
            // 1. Create handle
            let handle = afe_create_handle(std::ptr::null());
            assert!(!handle.is_null());

            // 2. Prepare temporary test directory
            let temp_dir = tempdir().unwrap();
            let sub_file = temp_dir.path().join("test_note.txt");
            let mut f = File::create(&sub_file).unwrap();
            f.write_all(b"Antigravity zero-mock offline file explorer test")
                .unwrap();

            let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

            // 3. List directory
            let list_ptr = afe_list_directory(handle, c_dir.as_ptr());
            assert!(!list_ptr.is_null());
            let list_str = CStr::from_ptr(list_ptr).to_str().unwrap();
            assert!(list_str.contains("test_note.txt"));
            afe_free_string(list_ptr);

            // 4. Index directory
            let idx_ptr = afe_index_directory(handle, c_dir.as_ptr());
            assert!(!idx_ptr.is_null());
            let idx_str = CStr::from_ptr(idx_ptr).to_str().unwrap();
            assert!(idx_str.contains("indexed_count"));
            afe_free_string(idx_ptr);

            // 5. Search files
            let query = CString::new("test_note").unwrap();
            let search_ptr = afe_search_files(handle, query.as_ptr(), 10);
            assert!(!search_ptr.is_null());
            let search_str = CStr::from_ptr(search_ptr).to_str().unwrap();
            assert!(search_str.contains("test_note.txt"));
            afe_free_string(search_ptr);

            // 6. Analyze storage
            let stats_ptr = afe_analyze_storage(handle, c_dir.as_ptr(), 5);
            assert!(!stats_ptr.is_null());
            let stats_str = CStr::from_ptr(stats_ptr).to_str().unwrap();
            assert!(stats_str.contains("total_files"));
            afe_free_string(stats_ptr);

            // 7. Extract document & search content
            let c_file = CString::new(sub_file.to_str().unwrap()).unwrap();
            let ext_ptr = afe_extract_document(handle, c_file.as_ptr());
            assert!(!ext_ptr.is_null());
            let ext_str = CStr::from_ptr(ext_ptr).to_str().unwrap();
            assert!(ext_str.contains("zero-mock"));
            afe_free_string(ext_ptr);

            let content_query_json = CString::new(
                r#"{"query": "zero-mock", "mode": "Combined", "limit": 5, "extension_filter": null}"#,
            )
            .unwrap();
            let c_res_ptr = afe_search_content(handle, content_query_json.as_ptr());
            assert!(!c_res_ptr.is_null());
            let c_res_str = CStr::from_ptr(c_res_ptr).to_str().unwrap();
            assert!(c_res_str.contains("zero-mock"));
            afe_free_string(c_res_ptr);

            // 8. Compliance scan
            let c_tier = CString::new("business").unwrap();
            let comp_ptr = afe_scan_compliance(handle, c_dir.as_ptr(), c_tier.as_ptr());
            assert!(!comp_ptr.is_null());
            let comp_str = CStr::from_ptr(comp_ptr).to_str().unwrap();
            assert!(comp_str.contains("scanned_files_count"));
            afe_free_string(comp_ptr);

            // 9. Free handle
            afe_free_handle(handle);
        }
    }
}
