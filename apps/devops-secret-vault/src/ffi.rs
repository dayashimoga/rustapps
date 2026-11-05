//! DevOps Secret Vault - Reusable C-ABI Foreign Function Interface (FFI) for Tauri & Flutter
//!
//! Conforms to ADR-0015 for cross-platform desktop and mobile integration.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
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

/// Generate real-time RFC 6238 TOTP code for a secret (returns TotpCodeResponse JSON or null on error)
///
/// # Safety
/// `handle` and `secret_id` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_generate_totp(
    handle: *mut VaultHandle,
    secret_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() || secret_id.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let id_str = match CStr::from_ptr(secret_id).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match h.rt.block_on(h.service.generate_totp_code(id_str)) {
        Ok(res) => match serde_json::to_string(&res) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Generate a 5-dimensional Vault Security Health & Risk Report (returns VaultHealthReport JSON or null on error)
///
/// # Safety
/// `handle` must be a valid non-null pointer. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_get_health_report(handle: *mut VaultHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;

    match h.rt.block_on(h.service.get_health_report()) {
        Ok(report) => match serde_json::to_string(&report) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Inspect an X.509 Certificate secret (returns CertInspectionReport JSON or null on error)
///
/// # Safety
/// `handle` and `secret_id` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_inspect_certificate(
    handle: *mut VaultHandle,
    secret_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() || secret_id.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let id_str = match CStr::from_ptr(secret_id).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match h.rt.block_on(h.service.inspect_certificate(id_str)) {
        Ok(report) => match serde_json::to_string(&report) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Export an authenticated encrypted .vaultbackup archive (returns BackupExportSummary JSON or null on error)
///
/// # Safety
/// `handle`, `passphrase`, and `output_path` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_export_backup(
    handle: *mut VaultHandle,
    passphrase: *const c_char,
    output_path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || passphrase.is_null() || output_path.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let pwd = match CStr::from_ptr(passphrase).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let path_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match h
        .rt
        .block_on(h.service.export_encrypted_backup(pwd, Path::new(path_str)))
    {
        Ok(summary) => match serde_json::to_string(&summary) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Import and restore secrets from an authenticated encrypted .vaultbackup archive (returns BackupImportSummary JSON or null on error)
///
/// # Safety
/// `handle`, `passphrase`, and `input_path` must be valid non-null pointers. Result must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_import_backup(
    handle: *mut VaultHandle,
    passphrase: *const c_char,
    input_path: *const c_char,
    overwrite: i32,
) -> *mut c_char {
    if handle.is_null() || passphrase.is_null() || input_path.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let pwd = match CStr::from_ptr(passphrase).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let path_str = match CStr::from_ptr(input_path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match h.rt.block_on(
        h.service
            .import_encrypted_backup(pwd, Path::new(path_str), overwrite != 0),
    ) {
        Ok(summary) => match serde_json::to_string(&summary) {
            Ok(json) => match CString::new(json) {
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

/// Verify the complete cryptographic hash chain of the audit log (returns 1 if valid, 0 if tampered/error)
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn vault_verify_audit_chain(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let h = &*handle;
    match h.rt.block_on(h.service.verify_audit_chain()) {
        Ok(valid) => {
            if valid {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

/// Check if biometrics are supported on this device (1 = yes, 0 = no)
///
/// # Safety
/// `handle` must be a valid pointer from `vault_create_handle`.
#[no_mangle]
pub unsafe extern "C" fn vault_is_biometric_available(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let h = &*handle;
    if h.service.is_biometric_available() {
        1
    } else {
        0
    }
}

/// Enable biometric unlock by saving active master key in platform keyring (0 on success, -1 on failure)
///
/// # Safety
/// `handle` must be a valid pointer from `vault_create_handle`.
#[no_mangle]
pub unsafe extern "C" fn vault_enable_biometric_unlock(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.rt.block_on(h.service.enable_biometric_unlock()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Disable biometric unlock by clearing key from platform keyring (0 on success, -1 on failure)
///
/// # Safety
/// `handle` must be a valid pointer from `vault_create_handle`.
#[no_mangle]
pub unsafe extern "C" fn vault_disable_biometric_unlock(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.service.disable_biometric_unlock() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Unlock vault with native biometrics (0 on success, -1 on failure)
///
/// # Safety
/// `handle` must be a valid pointer from `vault_create_handle`.
#[no_mangle]
pub unsafe extern "C" fn vault_unlock_with_biometrics(handle: *mut VaultHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = &*handle;
    match h.rt.block_on(h.service.unlock_with_biometrics()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Split vault master key into Shamir shares JSON (Business Tier)
///
/// # Safety
/// `handle` must be a valid pointer from `vault_create_handle`. The returned string must be freed with `vault_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vault_split_master_key(
    handle: *mut VaultHandle,
    threshold: u8,
    total_shares: u8,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let h = &*handle;
    let gate = shared_licensing::TierGate::enterprise();
    match h
        .rt
        .block_on(h.service.split_master_key(threshold, total_shares, &gate))
    {
        Ok(shares) => match serde_json::to_string(&shares) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Unlock vault using Shamir shares JSON (0 on success, -1 on failure)
///
/// # Safety
/// `handle` must be a valid pointer from `vault_create_handle`. `shares_json` must be a valid null-terminated JSON string.
#[no_mangle]
pub unsafe extern "C" fn vault_unlock_with_shamir_shares(
    handle: *mut VaultHandle,
    shares_json: *const c_char,
) -> i32 {
    if handle.is_null() || shares_json.is_null() {
        return -1;
    }
    let h = &*handle;
    let c_str = match CStr::from_ptr(shares_json).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let shares: Vec<crate::shamir::ShamirShare> = match serde_json::from_str(c_str) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    match h.rt.block_on(h.service.unlock_with_shamir_shares(&shares)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

fn to_c_string<T: serde::Serialize>(val: &T) -> *mut c_char {
    let json = serde_json::to_string(val).unwrap_or_else(|_| "{}".to_string());
    CString::new(json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn to_error_json(err_msg: &str) -> *mut c_char {
    let err_json = format!("{{\"error\": \"{}\"}}", err_msg.replace('"', "\\\""));
    CString::new(err_json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Dispatch any desktop IPC command via unified JSON-RPC over C-ABI
///
/// # Safety
/// `cmd` must be a valid non-null null-terminated C string.
/// `args_json` can be null or a valid null-terminated C string containing JSON object.
#[no_mangle]
pub unsafe extern "C" fn vault_dispatch_command(
    _handle: *mut VaultHandle,
    cmd: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    if cmd.is_null() {
        return to_error_json("Null command pointer passed to vault_dispatch_command");
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

            // Test secret creation via JSON
            let secret_json = CString::new(
                r#"{
                "title": "Mobile AWS Key",
                "tags": ["mobile", "aws"],
                "payload": {
                    "type": "aws_credential",
                    "data": {
                        "access_key_id": "AKIAIOSFODNN7EXAMPLE",
                        "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
                        "region": "us-east-1"
                    }
                }
            }"#,
            )
            .unwrap();
            let create_ptr = vault_create_secret(handle, secret_json.as_ptr());
            assert!(!create_ptr.is_null());
            vault_free_string(create_ptr);

            // Test health report
            let health_ptr = vault_get_health_report(handle);
            assert!(!health_ptr.is_null());
            let health_str = CStr::from_ptr(health_ptr).to_str().unwrap();
            assert!(health_str.contains("overall_score"));
            vault_free_string(health_ptr);

            // Test TOTP creation and generation
            let totp_json = CString::new(
                r#"{
                "title": "GitHub 2FA",
                "tags": ["2fa", "mfa"],
                "payload": {
                    "type": "totp",
                    "data": {
                        "secret_base32": "JBSWY3DPEHPK3PXP",
                        "account_name": "developer"
                    }
                }
            }"#,
            )
            .unwrap();
            let totp_create_ptr = vault_create_secret(handle, totp_json.as_ptr());
            assert!(!totp_create_ptr.is_null());
            let totp_meta: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(totp_create_ptr).to_str().unwrap()).unwrap();
            let totp_id = totp_meta["id"].as_str().unwrap();
            vault_free_string(totp_create_ptr);

            let c_totp_id = CString::new(totp_id).unwrap();
            let code_ptr = vault_generate_totp(handle, c_totp_id.as_ptr());
            assert!(!code_ptr.is_null());
            let code_str = CStr::from_ptr(code_ptr).to_str().unwrap();
            assert!(code_str.contains("code"));
            vault_free_string(code_ptr);

            // Test backup export
            let temp_dir = tempfile::tempdir().unwrap();
            let backup_file = temp_dir.path().join("mobile_backup.vaultbackup");
            let c_backup_path = CString::new(backup_file.to_str().unwrap()).unwrap();
            let c_backup_pwd = CString::new("BackupPass123!").unwrap();

            let backup_ptr =
                vault_export_backup(handle, c_backup_pwd.as_ptr(), c_backup_path.as_ptr());
            assert!(!backup_ptr.is_null());
            vault_free_string(backup_ptr);
            assert!(backup_file.exists());

            // Verify audit chain
            assert_eq!(vault_verify_audit_chain(handle), 1);

            // Test Shamir split and unlock via FFI
            let shares_ptr = vault_split_master_key(handle, 2, 3);
            assert!(!shares_ptr.is_null());
            let shares_json = CStr::from_ptr(shares_ptr).to_str().unwrap().to_string();
            vault_free_string(shares_ptr);

            // Lock vault and reconstruct using shares
            assert_eq!(vault_lock(handle), 0);
            assert_eq!(vault_is_locked(handle), 1);

            let c_shares = CString::new(shares_json).unwrap();
            assert_eq!(
                vault_unlock_with_shamir_shares(handle, c_shares.as_ptr()),
                0
            );
            assert_eq!(vault_is_locked(handle), 0);

            // Test unified JSON-RPC C-ABI command dispatch
            let cmd_gen = CString::new("generate_secure_password").unwrap();
            let args_gen =
                CString::new(r#"{"length": 24, "symbols": true, "digits": true}"#).unwrap();
            let rpc_pwd_ptr = vault_dispatch_command(handle, cmd_gen.as_ptr(), args_gen.as_ptr());
            assert!(!rpc_pwd_ptr.is_null());
            let rpc_pwd_str = CStr::from_ptr(rpc_pwd_ptr).to_str().unwrap();
            assert!(rpc_pwd_str.contains("password"));
            vault_free_string(rpc_pwd_ptr);

            let cmd_status = CString::new("get_vault_status").unwrap();
            let rpc_status_ptr =
                vault_dispatch_command(handle, cmd_status.as_ptr(), std::ptr::null());
            assert!(!rpc_status_ptr.is_null());
            let rpc_status_str = CStr::from_ptr(rpc_status_ptr).to_str().unwrap();
            assert!(rpc_status_str.contains("is_locked"));
            vault_free_string(rpc_status_ptr);

            vault_free_handle(handle);
        }
    }
}
