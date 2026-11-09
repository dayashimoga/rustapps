//! Universal Log Viewer - C-ABI Foreign Function Interface (FFI)
//!
//! Provides C-ABI exports (ulv_*) for mobile clients (Android/iOS via Flutter)
//! and desktop native integrations.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::commands::LogViewerService;
use crate::search_engine::LogQuery;
use crate::workspace::WorkspaceStore;
use shared_telemetry::LocalTelemetry;

/// Opaque wrapper for `LogViewerService` across C-ABI boundary
pub struct LogViewerHandle {
    service: Arc<LogViewerService>,
    rt: Arc<Runtime>,
}

/// Helper converting Rust string to raw C string pointer
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

/// Create a new Universal Log Viewer service instance
#[no_mangle]
pub extern "C" fn ulv_create_handle() -> *mut LogViewerHandle {
    let rt = match Runtime::new() {
        Ok(r) => Arc::new(r),
        Err(_) => return std::ptr::null_mut(),
    };

    let ws_store = match WorkspaceStore::open_in_memory() {
        Ok(s) => Arc::new(s),
        Err(_) => return std::ptr::null_mut(),
    };

    let telemetry = LocalTelemetry::new(false);
    let service = Arc::new(LogViewerService::new(ws_store, telemetry));

    Box::into_raw(Box::new(LogViewerHandle { service, rt }))
}

/// Free a `LogViewerHandle`
///
/// # Safety
/// `handle` must be a valid pointer returned by `ulv_create_handle` and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn ulv_free_handle(handle: *mut LogViewerHandle) {
    if !handle.is_null() {
        // SAFETY: Pointer is non-null and was allocated by Box::into_raw
        let _ = Box::from_raw(handle);
    }
}

/// Open log file and return OpenedFileInfo JSON
///
/// # Safety
/// `handle` must be valid, and `path` must be a non-null null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ulv_open_file(
    handle: *mut LogViewerHandle,
    path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to ulv_open_file");
    }

    // SAFETY: path is non-null and handle is verified non-null
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in file path"),
    };

    let wrapper = &*handle;
    match wrapper.rt.block_on(wrapper.service.open_file(path_str)) {
        Ok(info) => to_c_string(&info),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Retrieve lines slice JSON
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ulv_get_lines_slice(
    handle: *mut LogViewerHandle,
    start_line: usize,
    count: usize,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ulv_get_lines_slice");
    }

    let wrapper = &*handle;
    match wrapper
        .rt
        .block_on(wrapper.service.get_lines_slice(start_line, count))
    {
        Ok(lines) => to_c_string(&lines),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Execute search query across active file
///
/// # Safety
/// `handle` must be valid, and `query_pattern` must be a non-null null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ulv_search(
    handle: *mut LogViewerHandle,
    query_pattern: *const c_char,
    is_regex: bool,
    case_sensitive: bool,
    max_results: usize,
) -> *mut c_char {
    if handle.is_null() || query_pattern.is_null() {
        return to_error_json("Null pointer passed to ulv_search");
    }

    let pattern_str = match CStr::from_ptr(query_pattern).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in search pattern"),
    };

    let query = LogQuery {
        pattern: pattern_str.to_string(),
        is_regex,
        case_sensitive,
        limit: max_results,
        start_line: 0,
    };

    let wrapper = &*handle;
    match wrapper.rt.block_on(wrapper.service.search(query)) {
        Ok(report) => to_c_string(&report),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Retrieve timeline histogram JSON
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ulv_get_timeline_histogram(
    handle: *mut LogViewerHandle,
    target_buckets: usize,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ulv_get_timeline_histogram");
    }

    let wrapper = &*handle;
    match wrapper
        .rt
        .block_on(wrapper.service.get_timeline_histogram(target_buckets))
    {
        Ok(hist) => to_c_string(&hist),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Mine template clusters JSON
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ulv_mine_templates(
    handle: *mut LogViewerHandle,
    max_records: usize,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ulv_mine_templates");
    }

    let wrapper = &*handle;
    match wrapper
        .rt
        .block_on(wrapper.service.mine_templates(max_records))
    {
        Ok(clusters) => to_c_string(&clusters),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Cluster errors JSON
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ulv_cluster_errors(
    handle: *mut LogViewerHandle,
    max_records: usize,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ulv_cluster_errors");
    }

    let wrapper = &*handle;
    match wrapper
        .rt
        .block_on(wrapper.service.cluster_errors(max_records))
    {
        Ok(clusters) => to_c_string(&clusters),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Dispatch any desktop IPC command via unified JSON-RPC over C-ABI
///
/// # Safety
/// `cmd` must be a valid non-null null-terminated C string.
/// `args_json` can be null or a valid null-terminated C string containing JSON object.
#[no_mangle]
pub unsafe extern "C" fn ulv_dispatch_command(
    handle: *mut LogViewerHandle,
    cmd: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    if cmd.is_null() {
        return to_error_json("Null command pointer passed to ulv_dispatch_command");
    }

    let cmd_str = match CStr::from_ptr(cmd).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in command string"),
    };

    let args: serde_json::Value = if args_json.is_null() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        match CStr::from_ptr(args_json).to_str() {
            Ok(s) if !s.trim().is_empty() => {
                serde_json::from_str(s).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
            }
            _ => serde_json::Value::Object(serde_json::Map::new()),
        }
    };

    let res = if !handle.is_null() {
        let wrapper = &*handle;
        crate::commands::dispatch_ipc_command_with_service(&wrapper.service, cmd_str, args)
    } else {
        crate::commands::dispatch_ipc_command(cmd_str, args)
    };

    match res {
        Ok(res) => to_c_string(&res),
        Err(e) => to_error_json(&e),
    }
}

/// Free a C string previously allocated by `ulv_*` functions
///
/// # Safety
/// `ptr` must have been returned by an `ulv_*` FFI function or be null.
#[no_mangle]
pub unsafe extern "C" fn ulv_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: Pointer is non-null and was created by CString::into_raw
        let _ = CString::from_raw(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulv_ffi_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("ffi_test.log");
        let content =
            b"2026-10-19T10:00:00Z [INFO] System booted\n2026-10-19T10:00:01Z [ERROR] Disk error\n";
        std::fs::write(&log_path, content).unwrap();

        unsafe {
            // 1. Create handle
            let handle = ulv_create_handle();
            assert!(!handle.is_null());

            // 2. Open file
            let c_path = CString::new(log_path.to_str().unwrap()).unwrap();
            let open_json_ptr = ulv_open_file(handle, c_path.as_ptr());
            assert!(!open_json_ptr.is_null());
            let open_json = CStr::from_ptr(open_json_ptr).to_str().unwrap();
            assert!(open_json.contains("\"total_lines\":2"));
            ulv_free_string(open_json_ptr);

            // 3. Get lines slice
            let lines_ptr = ulv_get_lines_slice(handle, 0, 2);
            assert!(!lines_ptr.is_null());
            let lines_json = CStr::from_ptr(lines_ptr).to_str().unwrap();
            assert!(lines_json.contains("System booted"));
            assert!(lines_json.contains("Disk error"));
            ulv_free_string(lines_ptr);

            // 4. Search
            let q_pat = CString::new("Disk").unwrap();
            let search_ptr = ulv_search(handle, q_pat.as_ptr(), false, false, 10);
            assert!(!search_ptr.is_null());
            let search_json = CStr::from_ptr(search_ptr).to_str().unwrap();
            assert!(search_json.contains("\"total_matched_in_scan\":1"));
            ulv_free_string(search_ptr);

            // 5. Timeline Histogram
            let hist_ptr = ulv_get_timeline_histogram(handle, 2);
            assert!(!hist_ptr.is_null());
            let hist_json = CStr::from_ptr(hist_ptr).to_str().unwrap();
            assert!(hist_json.contains("\"total_records\":2"));
            ulv_free_string(hist_ptr);

            // 6. Mine templates
            let templates_ptr = ulv_mine_templates(handle, 10);
            assert!(!templates_ptr.is_null());
            ulv_free_string(templates_ptr);

            // 7. Cluster errors
            let err_clusters_ptr = ulv_cluster_errors(handle, 10);
            assert!(!err_clusters_ptr.is_null());
            ulv_free_string(err_clusters_ptr);

            // 8. Test unified JSON-RPC C-ABI command dispatch
            let cmd_open = CString::new("open_file").unwrap();
            let args_open = CString::new(format!(
                r#"{{"path": "{}"}}"#,
                log_path.to_str().unwrap().replace('\\', "/")
            ))
            .unwrap();
            let rpc_open_ptr = ulv_dispatch_command(handle, cmd_open.as_ptr(), args_open.as_ptr());
            assert!(!rpc_open_ptr.is_null());
            ulv_free_string(rpc_open_ptr);

            let cmd_lines = CString::new("get_lines_slice").unwrap();
            let args_lines = CString::new(r#"{"offset": 0, "limit": 2}"#).unwrap();
            let rpc_lines_ptr =
                ulv_dispatch_command(handle, cmd_lines.as_ptr(), args_lines.as_ptr());
            assert!(!rpc_lines_ptr.is_null());
            let rpc_lines_str = CStr::from_ptr(rpc_lines_ptr).to_str().unwrap();
            assert!(rpc_lines_str.contains("System booted"));
            ulv_free_string(rpc_lines_ptr);

            // 9. Free handle
            ulv_free_handle(handle);
        }
    }
}
