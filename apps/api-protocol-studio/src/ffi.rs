//! API & Protocol Studio - C-ABI FFI Interface
//!
//! Provides JSON-RPC and C-ABI export functions for Flutter mobile (Android/iOS) and native host wrappers.

use crate::commands::dispatch_ipc_command;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Helper converting a C string pointer to a Rust `&str`
///
/// # Safety
/// The pointer must be valid and null-terminated.
unsafe fn c_str_to_str<'a>(ptr: *const c_char) -> Result<&'a str, &'static str> {
    if ptr.is_null() {
        return Err("Null pointer passed");
    }
    // SAFETY: caller guarantees `ptr` points to a valid null-terminated C string
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| "Invalid UTF-8 in C string")
}

/// Helper converting Rust string to allocated raw C string pointer
fn to_c_string(val: &str) -> *mut c_char {
    CString::new(val)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Dispatches an IPC command across the C-ABI boundary using JSON strings
///
/// # Safety
/// `cmd` and `payload_json` must be valid null-terminated C strings.
/// The returned pointer must be freed using `aps_free_string`.
#[no_mangle]
pub unsafe extern "C" fn aps_dispatch_command(
    cmd: *const c_char,
    payload_json: *const c_char,
) -> *mut c_char {
    // SAFETY: Verified non-null and UTF-8 decoded in c_str_to_str
    let command_str = match unsafe { c_str_to_str(cmd) } {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!("{{\"error\":\"{}\"}}", e)),
    };

    // SAFETY: Verified non-null and UTF-8 decoded in c_str_to_str
    let payload_val = if payload_json.is_null() {
        serde_json::json!({})
    } else {
        match unsafe { c_str_to_str(payload_json) } {
            Ok(s) => serde_json::from_str(s).unwrap_or(serde_json::json!({})),
            Err(e) => return to_c_string(&format!("{{\"error\":\"{}\"}}", e)),
        }
    };

    match dispatch_ipc_command(command_str, payload_val) {
        Ok(val) => to_c_string(&serde_json::to_string(&val).unwrap_or_else(|_| "{}".to_string())),
        Err(err) => to_c_string(&format!("{{\"error\":\"{}\"}}", err.replace('"', "\\\""))),
    }
}

/// Frees a string pointer previously allocated by this library
///
/// # Safety
/// `ptr` must be a pointer allocated via `CString::into_raw`, or null.
#[no_mangle]
pub unsafe extern "C" fn aps_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: `ptr` was allocated by CString::into_raw
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aps_ffi_dispatch_lifecycle() {
        let cmd = CString::new("get_studio_stats").unwrap();
        let payload = CString::new("{}").unwrap();

        // SAFETY: Both pointers point to valid CString allocations
        let res_ptr = unsafe { aps_dispatch_command(cmd.as_ptr(), payload.as_ptr()) };
        assert!(!res_ptr.is_null());

        // SAFETY: res_ptr is valid string returned from aps_dispatch_command
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str().unwrap() };
        assert!(res_str.contains("total_collections"));

        // SAFETY: Free allocated string pointer
        unsafe { aps_free_string(res_ptr) };
    }
}
