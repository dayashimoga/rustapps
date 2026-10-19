//! DevOps Secret Vault - Reusable C-ABI Foreign Function Interface (FFI) for Tauri & Flutter
//!
//! Conforms to ADR-0015 for cross-platform desktop and mobile integration.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::models::CreateSecretInput;
use crate::service::VaultService;
use crate::storage::VaultStorage;

/// Opaque wrapper for `VaultService` passed across C-ABI boundary
pub struct VaultHandle {
    service: VaultService,
    rt: Arc<Runtime>,
}

/// Create a new VaultService instance from a database file path (or ":memory:")
///
/// # Safety
/// `db_path` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn vault_create_handle(db_path: *const c_char) -> *mut VaultHandle {
    if db_path.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = match CStr::from_ptr(db_path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let rt = match Runtime::new() {
        Ok(r) => Arc::new(r),
        Err(_) => return std::ptr::null_mut(),
    };

    let storage = if c_str == ":memory:" {
        match VaultStorage::open_in_memory() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    } else {
        match VaultStorage::open(c_str) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };

    let service = VaultService::new(storage, None);
    Box::into_raw(Box::new(VaultHandle { service, rt }))
}

/// Free a `VaultHandle` pointer
///
/// # Safety
/// `handle` must have been returned by `vault_create_handle` and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn vault_free_handle(handle: *mut VaultHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Check if vault is initialized (returns 1 if initialized, 0 if not, -1 on error)
///
/// # Safety
/// `handle` must be a valid pointer to a `VaultHandle`.
#[no_mangle]
pub unsafe extern "C" fn vault_is_initialized(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.service.is_initialized() {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// Initialize the vault with a master password (returns 0 on success, -1 on error)
///
/// # Safety
/// `handle` and `password` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn vault_initialize(
    handle: *mut VaultHandle,
    password: *const c_char,
) -> i32 {
    if handle.is_null() || password.is_null() {
        return -1;
    }
    let h = &*handle;
    let pwd = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match h.rt.block_on(h.service.initialize(pwd)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Unlock the vault (returns 0 on success, -1 on failure/wrong password)
///
/// # Safety
/// `handle` and `password` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn vault_unlock(handle: *mut VaultHandle, password: *const c_char) -> i32 {
    if handle.is_null() || password.is_null() {
        return -1;
    }
    let h = &*handle;
    let pwd = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match h.rt.block_on(h.service.unlock(pwd)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Lock the vault (returns 0 on success, -1 on error)
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn vault_lock(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.rt.block_on(h.service.lock()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Check if vault is locked (returns 1 if locked, 0 if unlocked, -1 on error)
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn vault_is_locked(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    if h.rt.block_on(h.service.is_locked()) {
        1
    } else {
        0
    }
}

/// Execute search and return JSON array of `SecretSearchResult` (or null on error)
///
/// # Safety
/// `handle` and `query` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_search(
    handle: *mut VaultHandle,
    query: *const c_char,
    limit: usize,
) -> *mut c_char {
    if handle.is_null() || query.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let q = match CStr::from_ptr(query).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match h.rt.block_on(h.service.search_secrets(q, limit)) {
        Ok(results) => match serde_json::to_string(&results) {
            Ok(json) => match CString::new(json) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create a new secret from a JSON payload string (returns metadata JSON or null on error)
///
/// # Safety
/// `handle` and `input_json` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_create_secret(
    handle: *mut VaultHandle,
    input_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || input_json.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let json_str = match CStr::from_ptr(input_json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let input: CreateSecretInput = match serde_json::from_str(json_str) {
        Ok(i) => i,
        Err(_) => return std::ptr::null_mut(),
    };

    match h.rt.block_on(h.service.create_secret(input, "mobile-ffi")) {
        Ok(meta) => match serde_json::to_string(&meta) {
            Ok(res_json) => match CString::new(res_json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Retrieve and decrypt a secret by ID (returns DecryptedSecret JSON or null on error)
///
/// # Safety
/// `handle` and `id` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_get_secret(
    handle: *mut VaultHandle,
    id: *const c_char,
) -> *mut c_char {
    if handle.is_null() || id.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let id_str = match CStr::from_ptr(id).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match h.rt.block_on(h.service.get_secret(id_str, "mobile-ffi")) {
        Ok(decrypted) => match serde_json::to_string(&decrypted) {
            Ok(res_json) => match CString::new(res_json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string returned by any `vault_*` function
///
/// # Safety
/// `s` must have been returned by an FFI call that produces a raw CString and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn vault_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_lifecycle() {
        unsafe {
            let path = CString::new(":memory:").unwrap();
            let handle = vault_create_handle(path.as_ptr());
            assert!(!handle.is_null());

            assert_eq!(vault_is_initialized(handle), 0);

            let pwd = CString::new("FFIPassword123!").unwrap();
            assert_eq!(vault_initialize(handle, pwd.as_ptr()), 0);
            assert_eq!(vault_is_initialized(handle), 1);
            assert_eq!(vault_is_locked(handle), 0);

            assert_eq!(vault_lock(handle), 0);
            assert_eq!(vault_is_locked(handle), 1);

            assert_eq!(vault_unlock(handle, pwd.as_ptr()), 0);
            assert_eq!(vault_is_locked(handle), 0);

            let query = CString::new("*").unwrap();
            let res_ptr = vault_search(handle, query.as_ptr(), 10);
            assert!(!res_ptr.is_null());
            vault_free_string(res_ptr);

            vault_free_handle(handle);
        }
    }
}
