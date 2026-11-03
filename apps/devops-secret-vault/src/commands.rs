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

static DEFAULT_VAULT_SERVICE: std::sync::OnceLock<Arc<VaultService>> = std::sync::OnceLock::new();

pub fn get_default_vault_service() -> Result<Arc<VaultService>, String> {
    if let Some(svc) = DEFAULT_VAULT_SERVICE.get() {
        return Ok(Arc::clone(svc));
    }
    let db_path = if let Ok(env_path) = std::env::var("VAULT_DB_PATH") {
        std::path::PathBuf::from(env_path)
    } else if let Ok(paths) = platform_common::AppPaths::from_app_name("devops-secret-vault") {
        paths.data_dir.join("devops_vault.db")
    } else {
        std::path::PathBuf::from("devops_vault.db")
    };
    let storage = crate::storage::VaultStorage::open(&db_path).map_err(|e| e.to_string())?;
    let svc = Arc::new(VaultService::new(storage, None));
    let _ = DEFAULT_VAULT_SERVICE.set(Arc::clone(&svc));
    Ok(svc)
}

/// Dispatch desktop IPC commands from the Svelte 5 DevOps Secret Vault UI
pub fn dispatch_ipc_command(
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let svc = get_default_vault_service()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async move {
        match cmd {
            "get_vault_status" => {
                let status = get_vault_status(&svc).await?;
                serde_json::to_value(status).map_err(|e| e.to_string())
            }
            "initialize_vault" => {
                let pw = args
                    .get("masterPassword")
                    .or_else(|| args.get("master_password"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                initialize_vault(&svc, pw).await?;
                Ok(serde_json::json!(true))
            }
            "unlock_vault" => {
                let pw = args
                    .get("masterPassword")
                    .or_else(|| args.get("master_password"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                unlock_vault(&svc, pw).await?;
                Ok(serde_json::json!(true))
            }
            "lock_vault" => {
                lock_vault(&svc).await?;
                Ok(serde_json::json!(true))
            }
            "list_secrets" => {
                let list = list_secrets(&svc).await?;
                serde_json::to_value(list).map_err(|e| e.to_string())
            }
            "get_secret" => {
                let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let sec = get_secret(&svc, id).await?;
                serde_json::to_value(sec).map_err(|e| e.to_string())
            }
            "create_secret" => {
                let input_val = args.get("input").cloned().unwrap_or(args);
                let input: CreateSecretInput =
                    serde_json::from_value(input_val).map_err(|e| e.to_string())?;
                let meta = create_secret(&svc, input).await?;
                serde_json::to_value(meta).map_err(|e| e.to_string())
            }
            "update_secret" => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let input_val = args
                    .get("input")
                    .cloned()
                    .ok_or_else(|| "Missing input".to_string())?;
                let input: UpdateSecretInput =
                    serde_json::from_value(input_val).map_err(|e| e.to_string())?;
                let meta = update_secret(&svc, &id, input).await?;
                serde_json::to_value(meta).map_err(|e| e.to_string())
            }
            "delete_secret" => {
                let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                delete_secret(&svc, id).await?;
                Ok(serde_json::json!(true))
            }
            "search_secrets" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let hits = search_secrets(&svc, query, limit).await?;
                serde_json::to_value(hits).map_err(|e| e.to_string())
            }
            "copy_to_clipboard" => {
                let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let ttl = args
                    .get("ttlSecs")
                    .or_else(|| args.get("ttl_secs"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let rem = copy_to_clipboard(&svc, id, ttl).await?;
                Ok(serde_json::json!(rem))
            }
            "get_clipboard_remaining_seconds" => {
                let rem = get_clipboard_remaining_seconds(&svc).await;
                Ok(serde_json::json!(rem))
            }
            "clear_clipboard" => {
                clear_clipboard(&svc).await?;
                Ok(serde_json::json!(true))
            }
            "check_password_strength" => {
                let pw = args.get("password").and_then(|v| v.as_str()).unwrap_or("");
                let strength = check_password_strength(pw);
                serde_json::to_value(strength).map_err(|e| e.to_string())
            }
            "get_audit_logs" => {
                let secret_id = args
                    .get("secretId")
                    .or_else(|| args.get("secret_id"))
                    .and_then(|v| v.as_str());
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let logs = get_audit_logs(&svc, secret_id, limit).await?;
                serde_json::to_value(logs).map_err(|e| e.to_string())
            }
            "get_health_report" => {
                let report = get_health_report(&svc).await?;
                serde_json::to_value(report).map_err(|e| e.to_string())
            }
            "generate_totp_code" => {
                let secret_id = args
                    .get("secretId")
                    .or_else(|| args.get("secret_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let code = generate_totp_code(&svc, secret_id).await?;
                serde_json::to_value(code).map_err(|e| e.to_string())
            }
            "inspect_certificate" => {
                let secret_id = args
                    .get("secretId")
                    .or_else(|| args.get("secret_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let cert = inspect_certificate(&svc, secret_id).await?;
                serde_json::to_value(cert).map_err(|e| e.to_string())
            }
            "export_backup" => {
                let passphrase = args
                    .get("passphrase")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let output_path = args
                    .get("outputPath")
                    .or_else(|| args.get("output_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("vault.vaultbackup");
                let res = export_backup(&svc, passphrase, output_path).await?;
                serde_json::to_value(res).map_err(|e| e.to_string())
            }
            "import_backup" => {
                let passphrase = args
                    .get("passphrase")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let input_path = args
                    .get("inputPath")
                    .or_else(|| args.get("input_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("vault.vaultbackup");
                let overwrite = args
                    .get("overwrite")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let res = import_backup(&svc, passphrase, input_path, overwrite).await?;
                serde_json::to_value(res).map_err(|e| e.to_string())
            }
            "verify_audit_chain" => {
                let ok = verify_audit_chain(&svc).await?;
                Ok(serde_json::json!(ok))
            }
            "is_biometric_available" => Ok(serde_json::json!(is_biometric_available(&svc))),
            "enable_biometric_unlock" => {
                enable_biometric_unlock(&svc).await?;
                Ok(serde_json::json!(true))
            }
            "disable_biometric_unlock" => {
                disable_biometric_unlock(&svc)?;
                Ok(serde_json::json!(true))
            }
            "unlock_with_biometrics" => {
                unlock_with_biometrics(&svc).await?;
                Ok(serde_json::json!(true))
            }
            "split_master_key" => {
                let threshold = args.get("threshold").and_then(|v| v.as_u64()).unwrap_or(2) as u8;
                let total_shares = args
                    .get("totalShares")
                    .or_else(|| args.get("total_shares"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3) as u8;
                let shares = split_master_key(&svc, threshold, total_shares).await?;
                serde_json::to_value(shares).map_err(|e| e.to_string())
            }
            "unlock_with_shamir_shares" => {
                let shares_val = args
                    .get("shares")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
                let shares: Vec<crate::shamir::ShamirShare> =
                    serde_json::from_value(shares_val).map_err(|e| e.to_string())?;
                unlock_with_shamir_shares(&svc, shares).await?;
                Ok(serde_json::json!(true))
            }
            "parse_aws_ini" => {
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let target_profile = args
                    .get("targetProfile")
                    .or_else(|| args.get("target_profile"))
                    .and_then(|v| v.as_str());
                let creds = parse_aws_ini(content, target_profile)?;
                serde_json::to_value(creds).map_err(|e| e.to_string())
            }
            "parse_azure_json" => {
                let json_str = args
                    .get("jsonStr")
                    .or_else(|| args.get("json_str"))
                    .or_else(|| args.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let subscription_id = args
                    .get("subscriptionId")
                    .or_else(|| args.get("subscription_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("00000000-0000-0000-0000-000000000000");
                let cred = parse_azure_json(json_str, subscription_id)?;
                serde_json::to_value(cred).map_err(|e| e.to_string())
            }
            "parse_terraform_vars" => {
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let vars = parse_terraform_vars(content);
                serde_json::to_value(vars).map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown DevOps Secret Vault IPC command: {}", cmd)),
        }
    })
}
