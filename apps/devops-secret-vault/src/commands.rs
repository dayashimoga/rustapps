#![forbid(unsafe_code)]
//! DevOps Secret Vault - Tauri Desktop IPC Command Handlers

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::{
    calculate_entropy, CreateSecretInput, DecryptedSecret, PasswordStrength, SecretAuditEntry,
    SecretMetadata, SecretSearchResult, UpdateSecretInput,
};
use crate::service::VaultService;

/// Shared application state managed by Tauri
pub struct VaultAppState {
    pub service: Arc<VaultService>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStatusResponse {
    pub is_initialized: bool,
    pub is_locked: bool,
}

/// Check vault initialization and lock status
pub async fn get_vault_status(service: &VaultService) -> Result<VaultStatusResponse, String> {
    let is_initialized = service.is_initialized().map_err(|e| e.to_string())?;
    let is_locked = service.is_locked().await;
    Ok(VaultStatusResponse {
        is_initialized,
        is_locked,
    })
}

/// Initialize vault with master password
pub async fn initialize_vault(service: &VaultService, master_password: &str) -> Result<(), String> {
    service
        .initialize(master_password)
        .await
        .map_err(|e| e.to_string())
}

/// Unlock vault with master password
pub async fn unlock_vault(service: &VaultService, master_password: &str) -> Result<(), String> {
    service
        .unlock(master_password)
        .await
        .map_err(|e| e.to_string())
}

/// Lock vault immediately
pub async fn lock_vault(service: &VaultService) -> Result<(), String> {
    service.lock().await.map_err(|e| e.to_string())
}

/// List all secret metadata
pub async fn list_secrets(service: &VaultService) -> Result<Vec<SecretMetadata>, String> {
    service.list_secrets().await.map_err(|e| e.to_string())
}

/// Retrieve and decrypt a single secret
pub async fn get_secret(service: &VaultService, id: &str) -> Result<DecryptedSecret, String> {
    service
        .get_secret(id, "desktop-tauri")
        .await
        .map_err(|e| e.to_string())
}

/// Create a new secret
pub async fn create_secret(
    service: &VaultService,
    input: CreateSecretInput,
) -> Result<SecretMetadata, String> {
    service
        .create_secret(input, "desktop-tauri")
        .await
        .map_err(|e| e.to_string())
}

/// Update an existing secret
pub async fn update_secret(
    service: &VaultService,
    id: &str,
    input: UpdateSecretInput,
) -> Result<SecretMetadata, String> {
    service
        .update_secret(id, input, "desktop-tauri")
        .await
        .map_err(|e| e.to_string())
}

/// Delete a secret
pub async fn delete_secret(service: &VaultService, id: &str) -> Result<(), String> {
    service
        .delete_secret(id, "desktop-tauri")
        .await
        .map_err(|e| e.to_string())
}

/// Search secrets (<50ms Tantivy)
pub async fn search_secrets(
    service: &VaultService,
    query: &str,
    limit: usize,
) -> Result<Vec<SecretSearchResult>, String> {
    service
        .search_secrets(query, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Copy secret value to clipboard with auto-clear countdown
pub async fn copy_to_clipboard(
    service: &VaultService,
    id: &str,
    ttl_secs: Option<u32>,
) -> Result<u32, String> {
    service
        .copy_to_clipboard(id, ttl_secs, "desktop-tauri")
        .await
        .map_err(|e| e.to_string())
}

/// Get remaining clipboard countdown seconds
pub async fn get_clipboard_remaining_seconds(service: &VaultService) -> Option<u32> {
    service.clipboard_remaining_seconds().await
}

/// Clear clipboard immediately
pub async fn clear_clipboard(service: &VaultService) -> Result<(), String> {
    service.clear_clipboard().await.map_err(|e| e.to_string())
}

/// Calculate password entropy and strength characteristics
pub fn check_password_strength(password: &str) -> PasswordStrength {
    calculate_entropy(password)
}

/// Get audit logs
pub async fn get_audit_logs(
    service: &VaultService,
    secret_id: Option<&str>,
    limit: usize,
) -> Result<Vec<SecretAuditEntry>, String> {
    service
        .get_audit_logs(secret_id, limit)
        .await
        .map_err(|e| e.to_string())
}
