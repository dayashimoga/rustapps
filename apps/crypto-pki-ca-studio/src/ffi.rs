//! C-ABI Foreign Function Interface (FFI) for Crypto PKI & CA Studio
//!
//! Provides C-ABI exports for mobile clients (Android/iOS via Flutter)
//! and native platform embedding.

use crate::commands::dispatch_ipc_command;
use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Dispatches an IPC command formatted as JSON through the C-ABI boundary.
///
/// # Safety
/// `cmd_ptr` and `payload_ptr` must be valid, null-terminated UTF-8 C strings.
/// The returned pointer must be freed with `crypto_pki_ca_studio_free_string`.
#[no_mangle]
pub unsafe extern "C" fn crypto_pki_ca_studio_ffi_dispatch_json(
    cmd_ptr: *const c_char,
    payload_ptr: *const c_char,
) -> *mut c_char {
    if cmd_ptr.is_null() {
        return to_error_json("Command pointer is null");
    }

    // SAFETY: cmd_ptr is checked for non-null and is assumed null-terminated UTF-8.
    let cmd = match CStr::from_ptr(cmd_ptr).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in command string"),
    };

    let payload: Value = if payload_ptr.is_null() {
        Value::Null
    } else {
        // SAFETY: payload_ptr is checked for non-null and is assumed null-terminated UTF-8.
        match CStr::from_ptr(payload_ptr).to_str() {
            Ok(s) => serde_json::from_str(s).unwrap_or(Value::Null),
            Err(_) => return to_error_json("Invalid UTF-8 in payload string"),
        }
    };

    match dispatch_ipc_command(cmd, payload) {
        Ok(res) => {
            let json_str = serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string());
            CString::new(json_str)
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut())
        }
        Err(e) => to_error_json(&e),
    }
}

/// Frees a string previously allocated by `crypto_pki_ca_studio_ffi_dispatch_json`.
///
/// # Safety
/// `ptr` must be a pointer returned by this library or null.
#[no_mangle]
pub unsafe extern "C" fn crypto_pki_ca_studio_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: ptr was created via CString::into_raw
        let _ = CString::from_raw(ptr);
    }
}

fn to_error_json(err_msg: &str) -> *mut c_char {
    let err_obj = serde_json::json!({
        "error": true,
        "message": err_msg
    });
    CString::new(err_obj.to_string())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_dispatch_roundtrip() {
        let cmd = CString::new("get_stats").unwrap();
        let payload = CString::new("{}").unwrap();

        unsafe {
            let res_ptr = crypto_pki_ca_studio_ffi_dispatch_json(cmd.as_ptr(), payload.as_ptr());
            assert!(!res_ptr.is_null());
            let res_str = CStr::from_ptr(res_ptr).to_str().unwrap();
            let json_val: Value = serde_json::from_str(res_str).unwrap();
            assert!(json_val.get("total_cas").is_some());
            crypto_pki_ca_studio_free_string(res_ptr);
        }
    }
}
