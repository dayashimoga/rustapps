//! Professional Audio & Video Editor - C-ABI Foreign Function Interface (FFI)
//!
//! Conforms to ADR-0015 for cross-platform desktop and mobile integration.
//! Exposes `ave_*` C-ABI endpoints for Flutter mobile (Android/iOS) and Tauri desktop IPC.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::models::Clip;
use crate::service::EditorService;
use crate::templates::{ExportPreset, ProjectTemplate};

/// Opaque wrapper for `EditorService` across C-ABI boundary
pub struct AveHandle {
    service: EditorService,
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

/// Create a new `AveHandle` instance
///
/// # Safety
/// `_data_dir` may be null or a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ave_create_handle(_data_dir: *const c_char) -> *mut AveHandle {
    let rt = match Runtime::new() {
        Ok(r) => Arc::new(r),
        Err(_) => return std::ptr::null_mut(),
    };

    let service = EditorService::new();
    Box::into_raw(Box::new(AveHandle { service, _rt: rt }))
}

/// Free `AveHandle` instance
///
/// # Safety
/// `handle` must be a valid pointer obtained from `ave_create_handle`.
#[no_mangle]
pub unsafe extern "C" fn ave_free_handle(handle: *mut AveHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Free a C string allocated by `ave_*` functions
///
/// # Safety
/// `ptr` must be a valid pointer obtained from `to_c_string` or `to_error_json`.
#[no_mangle]
pub unsafe extern "C" fn ave_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Retrieve current timeline project state as JSON
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_get_project_state(handle: *mut AveHandle) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_get_project_state");
    }

    let h = &*handle;
    match h.service.get_project() {
        Ok(proj) => to_c_string(&proj),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Create project optionally from template
///
/// # Safety
/// `handle` and `name` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_create_project(
    handle: *mut AveHandle,
    name: *const c_char,
    template_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() || name.is_null() {
        return to_error_json("Null pointer passed to ave_create_project");
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in project name"),
    };

    let tpl_str = if !template_id.is_null() {
        CStr::from_ptr(template_id).to_str().ok()
    } else {
        None
    };

    let h = &mut *handle;
    let new_service = if let Some(tid) = tpl_str {
        match EditorService::from_template(tid, name_str) {
            Ok(s) => s,
            Err(e) => return to_error_json(&e.to_string()),
        }
    } else {
        EditorService::new()
    };

    h.service = new_service;
    match h.service.get_project() {
        Ok(p) => to_c_string(&p),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Save project to `.avedproj` atomic file
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_save_project(handle: *mut AveHandle, path: *const c_char) -> i32 {
    if handle.is_null() || path.is_null() {
        return -1;
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let h = &*handle;
    match h.service.save_project(path_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Load project from `.avedproj` file
///
/// # Safety
/// `handle` and `path` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_load_project(
    handle: *mut AveHandle,
    path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return to_error_json("Null pointer passed to ave_load_project");
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in project path"),
    };

    let h = &*handle;
    match h.service.load_project(path_str) {
        Ok(_) => match h.service.get_project() {
            Ok(p) => to_c_string(&p),
            Err(e) => to_error_json(&e.to_string()),
        },
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Add clip to timeline
///
/// # Safety
/// `handle` and `clip_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_add_clip(
    handle: *mut AveHandle,
    clip_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || clip_json.is_null() {
        return to_error_json("Null pointer passed to ave_add_clip");
    }

    let json_str = match CStr::from_ptr(clip_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in clip_json"),
    };

    let clip: Clip = match serde_json::from_str(json_str) {
        Ok(c) => c,
        Err(e) => return to_error_json(&format!("Clip JSON parse error: {}", e)),
    };

    let h = &*handle;
    match h.service.add_clip(clip) {
        Ok(_) => to_c_string(&serde_json::json!({ "status": "ok" })),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Remove clip from timeline
///
/// # Safety
/// `handle` and `clip_id` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_remove_clip(handle: *mut AveHandle, clip_id: *const c_char) -> i32 {
    if handle.is_null() || clip_id.is_null() {
        return -1;
    }

    let id_str = match CStr::from_ptr(clip_id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let h = &*handle;
    match h.service.remove_clip(id_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Split clip at specified timestamp
///
/// # Safety
/// `handle` and `clip_id` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_split_clip(
    handle: *mut AveHandle,
    clip_id: *const c_char,
    timestamp_ms: u64,
) -> i32 {
    if handle.is_null() || clip_id.is_null() {
        return -1;
    }

    let id_str = match CStr::from_ptr(clip_id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let h = &*handle;
    match h.service.split_clip(id_str, timestamp_ms) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Ripple delete clip
///
/// # Safety
/// `handle` and `clip_id` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ave_ripple_delete(handle: *mut AveHandle, clip_id: *const c_char) -> i32 {
    if handle.is_null() || clip_id.is_null() {
        return -1;
    }

    let id_str = match CStr::from_ptr(clip_id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let h = &*handle;
    match h.service.ripple_delete(id_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Undo previous timeline action
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_undo(handle: *mut AveHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.service.undo() {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// Redo previously undone timeline action
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_redo(handle: *mut AveHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.service.redo() {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// Render frame at timestamp and return base64-encoded standard BMP image for preview
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_render_preview_bmp(
    handle: *mut AveHandle,
    timestamp_ms: u64,
    width: u32,
    height: u32,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_render_preview_bmp");
    }

    let h = &*handle;
    match h.service.render_frame(timestamp_ms, width, height) {
        Ok(frame) => {
            let bmp_bytes = frame.encode_to_bmp();
            const B64_CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut enc = Vec::new();
            for chunk in bmp_bytes.chunks(3) {
                let b0 = chunk[0];
                let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
                let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

                enc.push(B64_CHARS[(b0 >> 2) as usize]);
                enc.push(B64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
                if chunk.len() > 1 {
                    enc.push(B64_CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
                } else {
                    enc.push(b'=');
                }
                if chunk.len() > 2 {
                    enc.push(B64_CHARS[(b2 & 0x3f) as usize]);
                } else {
                    enc.push(b'=');
                }
            }
            let b64_str = String::from_utf8_lossy(&enc).to_string();
            let res = serde_json::json!({ "bmp_base64": b64_str });
            to_c_string(&res)
        }
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Compute broadcast video scopes
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_get_video_scopes(
    handle: *mut AveHandle,
    timestamp_ms: u64,
    width: u32,
    height: u32,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_get_video_scopes");
    }

    let h = &*handle;
    match h.service.get_video_scopes(timestamp_ms, width, height) {
        Ok(scopes) => to_c_string(&scopes),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// AI Silence Ripple-Cutter command
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_auto_cut_silence(handle: *mut AveHandle, threshold_dbfs: f32) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let h = &*handle;
    let config = crate::ai_editor::SilenceCutConfig {
        threshold_dbfs,
        ..Default::default()
    };
    match h.service.auto_cut_silence(&config) {
        Ok(count) => count as i32,
        Err(_) => -1,
    }
}

/// Export timeline to CMX 3600 EDL
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_export_edl(handle: *mut AveHandle) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_export_edl");
    }

    let h = &*handle;
    match h.service.export_edl() {
        Ok(edl) => {
            let res = serde_json::json!({ "edl_content": edl });
            to_c_string(&res)
        }
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Export timeline to Final Cut Pro XML
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_export_fcpxml(handle: *mut AveHandle) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_export_fcpxml");
    }

    let h = &*handle;
    match h.service.export_fcpxml() {
        Ok(fcpxml) => {
            let res = serde_json::json!({ "fcpxml_content": fcpxml });
            to_c_string(&res)
        }
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Retrieve all available project templates
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_get_templates(handle: *mut AveHandle) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_get_templates");
    }
    to_c_string(&ProjectTemplate::all_templates())
}

/// Retrieve all available export presets
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ave_get_export_presets(handle: *mut AveHandle) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null handle passed to ave_get_export_presets");
    }
    to_c_string(&ExportPreset::all_standard_presets())
}

/// Dispatch any desktop IPC command via unified JSON-RPC over C-ABI
///
/// # Safety
/// `cmd` must be a valid non-null null-terminated C string.
/// `args_json` can be null or a valid null-terminated C string containing JSON object.
#[no_mangle]
pub unsafe extern "C" fn ave_dispatch_command(
    _handle: *mut AveHandle,
    cmd: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    if cmd.is_null() {
        return to_error_json("Null command pointer passed to ave_dispatch_command");
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

    match crate::commands::dispatch_ipc_command(cmd_str, args) {
        Ok(res) => to_c_string(&res),
        Err(e) => to_error_json(&e),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_lifecycle_and_core_operations() {
        unsafe {
            let handle = ave_create_handle(std::ptr::null());
            assert!(!handle.is_null());

            // 1. Get project state
            let json_ptr = ave_get_project_state(handle);
            assert!(!json_ptr.is_null());
            let json_str = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert!(json_str.contains("Untitled Studio Project"));
            ave_free_string(json_ptr);

            // 2. Render preview BMP
            let bmp_ptr = ave_render_preview_bmp(handle, 0, 80, 45);
            assert!(!bmp_ptr.is_null());
            let bmp_str = CStr::from_ptr(bmp_ptr).to_str().unwrap();
            assert!(bmp_str.contains("bmp_base64"));
            ave_free_string(bmp_ptr);

            // 3. Export EDL
            let edl_ptr = ave_export_edl(handle);
            assert!(!edl_ptr.is_null());
            let edl_str = CStr::from_ptr(edl_ptr).to_str().unwrap();
            assert!(edl_str.contains("edl_content"));
            ave_free_string(edl_ptr);

            // 4. Test unified JSON-RPC C-ABI command dispatch
            let cmd_state = CString::new("get_project_state_cmd").unwrap();
            let rpc_state_ptr = ave_dispatch_command(handle, cmd_state.as_ptr(), std::ptr::null());
            assert!(!rpc_state_ptr.is_null());
            let rpc_state_str = CStr::from_ptr(rpc_state_ptr).to_str().unwrap();
            assert!(rpc_state_str.contains("project_id"));
            ave_free_string(rpc_state_ptr);

            let cmd_scopes = CString::new("get_video_scopes_cmd").unwrap();
            let args_scopes =
                CString::new(r#"{"timestampMs": 500, "width": 80, "height": 45}"#).unwrap();
            let rpc_scopes_ptr =
                ave_dispatch_command(handle, cmd_scopes.as_ptr(), args_scopes.as_ptr());
            assert!(!rpc_scopes_ptr.is_null());
            let rpc_scopes_str = CStr::from_ptr(rpc_scopes_ptr).to_str().unwrap();
            assert!(rpc_scopes_str.contains("histogram_r"));
            ave_free_string(rpc_scopes_ptr);

            ave_free_handle(handle);
        }
    }
}
