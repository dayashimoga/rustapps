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

/// Get on-device security risk scoring and health report
pub async fn get_health_report(
    service: &VaultService,
) -> Result<crate::risk_scoring::VaultHealthReport, String> {
    service.get_health_report().await.map_err(|e| e.to_string())
}

/// Generate live RFC 6238 TOTP code for a secret
pub async fn generate_totp_code(
    service: &VaultService,
    secret_id: &str,
) -> Result<crate::totp::TotpCodeResponse, String> {
    service
        .generate_totp_code(secret_id)
        .await
        .map_err(|e| e.to_string())
}

/// Inspect X.509 certificate for expiry and weak algorithms
pub async fn inspect_certificate(
    service: &VaultService,
    secret_id: &str,
) -> Result<crate::cert_monitor::CertInspectionReport, String> {
    service
        .inspect_certificate(secret_id)
        .await
        .map_err(|e| e.to_string())
}

/// Export encrypted .vaultbackup archive
pub async fn export_backup(
    service: &VaultService,
    passphrase: &str,
    output_path: &str,
) -> Result<crate::backup::BackupExportSummary, String> {
    service
        .export_encrypted_backup(passphrase, std::path::Path::new(output_path))
        .await
        .map_err(|e| e.to_string())
}

/// Import encrypted .vaultbackup archive
pub async fn import_backup(
    service: &VaultService,
    passphrase: &str,
    input_path: &str,
    overwrite: bool,
) -> Result<crate::backup::BackupImportSummary, String> {
    service
        .import_encrypted_backup(passphrase, std::path::Path::new(input_path), overwrite)
        .await
        .map_err(|e| e.to_string())
}

/// Parse AWS credentials from INI syntax
pub fn parse_aws_ini(
    content: &str,
    target_profile: Option<&str>,
) -> Result<Vec<crate::cloud_creds::AwsCredential>, String> {
    crate::cloud_creds::AwsCredential::parse_ini(content, target_profile).map_err(|e| e.to_string())
}

/// Parse Azure service principal credentials from az cli JSON output
pub fn parse_azure_json(
    json_str: &str,
    subscription_id: &str,
) -> Result<crate::cloud_creds::AzureCredential, String> {
    crate::cloud_creds::AzureCredential::parse_az_cli_json(json_str, subscription_id)
        .map_err(|e| e.to_string())
}

/// Parse Terraform .tfvars content into key-value map
pub fn parse_terraform_vars(content: &str) -> std::collections::HashMap<String, String> {
    crate::cloud_creds::TerraformSecret::parse_tfvars(content)
}

/// Verify tamper-evident cryptographic hash chain of audit logs
pub async fn verify_audit_chain(service: &VaultService) -> Result<bool, String> {
    service
        .verify_audit_chain()
        .await
        .map_err(|e| e.to_string())
}

/// Check if native biometric unlock is supported and enrolled
pub fn is_biometric_available(service: &VaultService) -> bool {
    service.is_biometric_available()
}

/// Enable biometric unlock using active vault master key
pub async fn enable_biometric_unlock(service: &VaultService) -> Result<(), String> {
    service
        .enable_biometric_unlock()
        .await
        .map_err(|e| e.to_string())
}

/// Disable biometric unlock and purge key from OS keyring
pub fn disable_biometric_unlock(service: &VaultService) -> Result<(), String> {
    service
        .disable_biometric_unlock()
        .map_err(|e| e.to_string())
}

/// Unlock vault with native OS biometrics (Windows Hello, Touch ID, Polkit)
pub async fn unlock_with_biometrics(service: &VaultService) -> Result<(), String> {
    service
        .unlock_with_biometrics()
        .await
        .map_err(|e| e.to_string())
}

/// Split vault master key into k-of-n Shamir shares (Business Tier)
pub async fn split_master_key(
    service: &VaultService,
    threshold: u8,
    total_shares: u8,
) -> Result<Vec<crate::shamir::ShamirShare>, String> {
    let gate = shared_licensing::TierGate::enterprise();
    service
        .split_master_key(threshold, total_shares, &gate)
        .await
        .map_err(|e| e.to_string())
}

/// Unlock vault using k-of-n Shamir shares
pub async fn unlock_with_shamir_shares(
    service: &VaultService,
    shares: Vec<crate::shamir::ShamirShare>,
) -> Result<(), String> {
    service
        .unlock_with_shamir_shares(&shares)
        .await
        .map_err(|e| e.to_string())
}
