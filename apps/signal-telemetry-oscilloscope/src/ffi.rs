#![allow(unsafe_code)]
//! C-ABI Foreign Function Interface for Desktop and Mobile Bindings

use crate::commands::dispatch_ipc_command;
use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Dispatches JSON-RPC IPC command over C-ABI
///
/// # Safety
/// Pointers must be valid, null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn scope_dispatch_command(
    command: *const c_char,
    payload_json: *const c_char,
) -> *mut c_char {
    if command.is_null() {
        return CString::new("{\"error\": \"Null command pointer\"}")
            .unwrap()
            .into_raw();
    }

    // SAFETY: caller guarantees null-terminated C string
    let cmd_str = match unsafe { CStr::from_ptr(command) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            return CString::new("{\"error\": \"Invalid UTF-8 in command\"}")
                .unwrap()
                .into_raw();
        }
    };

    let payload: Value = if payload_json.is_null() {
        Value::Null
    } else {
        // SAFETY: caller guarantees null-terminated C string if not null
        match unsafe { CStr::from_ptr(payload_json) }.to_str() {
            Ok(s) => serde_json::from_str(s).unwrap_or(Value::Null),
            Err(_) => {
                return CString::new("{\"error\": \"Invalid UTF-8 in payload\"}")
                    .unwrap()
                    .into_raw();
            }
        }
    };

    let result = match dispatch_ipc_command(cmd_str, payload) {
        Ok(v) => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        Err(e) => format!("{{\"error\": \"{}\"}}", e),
    };

    CString::new(result).unwrap().into_raw()
}

/// Frees string allocated by `scope_dispatch_command`
///
/// # Safety
/// Pointer must have been allocated via CString::into_raw in this crate.
#[no_mangle]
pub unsafe extern "C" fn scope_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: pointer originated from CString::into_raw
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_dispatch_roundtrip() {
        let cmd = CString::new("get_oscilloscope_stats").unwrap();
        let payload = CString::new("{}").unwrap();

        unsafe {
            let res_ptr = scope_dispatch_command(cmd.as_ptr(), payload.as_ptr());
            assert!(!res_ptr.is_null());
            let res_str = CStr::from_ptr(res_ptr).to_str().unwrap();
            assert!(res_str.contains("total_captures"));
            scope_free_string(res_ptr);
        }
    }
}
