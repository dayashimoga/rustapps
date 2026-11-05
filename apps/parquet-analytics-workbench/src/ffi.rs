#![allow(unsafe_code)]
//! C-ABI Foreign Function Interface Bridge for Mobile & Embedded Hosts

use crate::commands::dispatch_ipc_command;
use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Dispatches an IPC command string via C-ABI JSON-RPC
///
/// # Safety
/// Both `cmd_ptr` and `payload_ptr` must be valid, null-terminated UTF-8 C-strings.
/// The caller is responsible for freeing the returned pointer via `paw_free_string`.
#[no_mangle]
pub unsafe extern "C" fn paw_dispatch_command(
    cmd_ptr: *const c_char,
    payload_ptr: *const c_char,
) -> *mut c_char {
    if cmd_ptr.is_null() {
        let err_json = serde_json::json!({ "error": "Command pointer was null" }).to_string();
        return CString::new(err_json).unwrap_or_default().into_raw();
    }

    let command_str = match unsafe { CStr::from_ptr(cmd_ptr) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            let err_json =
                serde_json::json!({ "error": format!("Invalid UTF-8 in command: {}", e) })
                    .to_string();
            return CString::new(err_json).unwrap_or_default().into_raw();
        }
    };

    let payload: Value = if payload_ptr.is_null() {
        serde_json::json!({})
    } else {
        match unsafe { CStr::from_ptr(payload_ptr) }.to_str() {
            Ok(s) => serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        }
    };

    let result_json = match dispatch_ipc_command(command_str, payload) {
        Ok(res) => res.to_string(),
        Err(err) => serde_json::json!({ "error": err }).to_string(),
    };

    CString::new(result_json).unwrap_or_default().into_raw()
}

/// Frees a C-string allocated by `paw_dispatch_command`
///
/// # Safety
/// `ptr` must be a valid pointer allocated by `CString::into_raw` from this library.
#[no_mangle]
pub unsafe extern "C" fn paw_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_dispatch_roundtrip() {
        let cmd = CString::new("get_workbench_stats").unwrap();
        let payload = CString::new("{}").unwrap();

        let res_ptr = unsafe { paw_dispatch_command(cmd.as_ptr(), payload.as_ptr()) };
        assert!(!res_ptr.is_null());

        let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().unwrap();
        assert!(res_str.contains("total_datasets"));

        unsafe { paw_free_string(res_ptr) };
    }
}
