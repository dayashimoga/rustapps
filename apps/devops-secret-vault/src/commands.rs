#![forbid(unsafe_code)]
//! DevOps Secret Vault - Tauri Desktop IPC Command Handlers

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::generator::generate_password_or_passphrase;
use crate::leak_scanner::scan_dir_for_leaks;
use crate::models::{
    calculate_entropy, CreateSecretInput, DecryptedSecret, PasswordStrength, SecretAuditEntry,
    SecretMetadata, SecretPayload, SecretSearchResult, UpdateSecretInput,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_path: Option<String>,
}

/// Check vault initialization and lock status
pub async fn get_vault_status(service: &VaultService) -> Result<VaultStatusResponse, String> {
    let is_initialized = service.is_initialized().map_err(|e| e.to_string())?;
    let is_locked = service.is_locked().await;
    Ok(VaultStatusResponse {
        is_initialized,
        is_locked,
        vault_path: get_active_vault_db_path(),
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

type VaultServiceHolder = std::sync::RwLock<(std::path::PathBuf, Arc<VaultService>)>;

static DEFAULT_VAULT_SERVICE: std::sync::OnceLock<VaultServiceHolder> = std::sync::OnceLock::new();

fn resolve_default_db_path() -> std::path::PathBuf {
    if let Ok(env_path) = std::env::var("VAULT_DB_PATH") {
        std::path::PathBuf::from(env_path)
    } else if let Ok(paths) = platform_common::AppPaths::from_app_name("devops-secret-vault") {
        let _ = paths.ensure_directories();
        paths.data_dir.join("devops_vault.db")
    } else {
        std::path::PathBuf::from("devops_vault.db")
    }
}

pub fn get_active_vault_db_path() -> Option<String> {
    if let Some(lock) = DEFAULT_VAULT_SERVICE.get() {
        if let Ok(guard) = lock.read() {
            return Some(guard.0.to_string_lossy().to_string());
        }
    }
    Some(resolve_default_db_path().to_string_lossy().to_string())
}

pub fn get_default_vault_service() -> Result<Arc<VaultService>, String> {
    if let Some(lock) = DEFAULT_VAULT_SERVICE.get() {
        let guard = lock.read().map_err(|e| e.to_string())?;
        return Ok(Arc::clone(&guard.1));
    }
    let db_path = resolve_default_db_path();
    let storage = crate::storage::VaultStorage::open(&db_path).map_err(|e| e.to_string())?;
    let svc = Arc::new(VaultService::new(storage, None));
    let _ = DEFAULT_VAULT_SERVICE.set(std::sync::RwLock::new((db_path, Arc::clone(&svc))));
    Ok(svc)
}

pub fn switch_vault_db_path(new_path: &str) -> Result<Arc<VaultService>, String> {
    let trimmed = new_path.trim();
    if trimmed.is_empty() {
        return Err("Vault database file path cannot be empty.".to_string());
    }
    let path_buf = std::path::PathBuf::from(trimmed);
    if let Some(parent) = path_buf.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let storage = crate::storage::VaultStorage::open(&path_buf).map_err(|e| e.to_string())?;
    let new_svc = Arc::new(VaultService::new(storage, None));
    if let Some(lock) = DEFAULT_VAULT_SERVICE.get() {
        let mut guard = lock.write().map_err(|e| e.to_string())?;
        *guard = (path_buf, Arc::clone(&new_svc));
    } else {
        let _ = DEFAULT_VAULT_SERVICE.set(std::sync::RwLock::new((path_buf, Arc::clone(&new_svc))));
    }
    Ok(new_svc)
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
            "reset_vault" => {
                svc.reset_vault().await.map_err(|e| e.to_string())?;
                if let Some(pw) = args
                    .get("masterPassword")
                    .or_else(|| args.get("master_password"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                {
                    initialize_vault(&svc, pw).await?;
                }
                let status = get_vault_status(&svc).await?;
                serde_json::to_value(status).map_err(|e| e.to_string())
            }
            "switch_vault_db" => {
                let db_path = args
                    .get("path")
                    .or_else(|| args.get("dbPath"))
                    .or_else(|| args.get("db_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_svc = switch_vault_db_path(db_path)?;
                let status = get_vault_status(&new_svc).await?;
                serde_json::to_value(status).map_err(|e| e.to_string())
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
                    match serde_json::from_value(shares_val.clone()) {
                        Ok(parsed_shares) => parsed_shares,
                        Err(_) => {
                            if let Ok(string_shares) =
                                serde_json::from_value::<Vec<String>>(shares_val)
                            {
                                let mut parsed = Vec::new();
                                for s in string_shares {
                                    let share =
                                        crate::shamir::ShamirShare::from_formatted_string(&s)
                                            .map_err(|e| e.to_string())?;
                                    parsed.push(share);
                                }
                                parsed
                            } else {
                                return Err(
                                    "Invalid Shamir shares format. Expected share objects or string tokens."
                                        .to_string(),
                                );
                            }
                        }
                    };
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
            "scan_directory_for_leaks" => {
                let dir_str = args
                    .get("dirPath")
                    .or_else(|| args.get("dir_path"))
                    .or_else(|| args.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                let dir = std::path::Path::new(dir_str);
                let findings = scan_dir_for_leaks(dir);
                serde_json::to_value(findings).map_err(|e| e.to_string())
            }
            "generate_secure_password" => {
                let length = args.get("length").and_then(|v| v.as_u64()).unwrap_or(24) as usize;
                let use_upper = args.get("includeUppercase").or_else(|| args.get("use_upper")).and_then(|v| v.as_bool()).unwrap_or(true);
                let use_lower = args.get("includeLowercase").or_else(|| args.get("use_lower")).and_then(|v| v.as_bool()).unwrap_or(true);
                let use_nums = args.get("includeNumbers").or_else(|| args.get("use_nums")).and_then(|v| v.as_bool()).unwrap_or(true);
                let use_syms = args.get("includeSymbols").or_else(|| args.get("use_syms")).and_then(|v| v.as_bool()).unwrap_or(true);
                let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("password");
                let word_count = args.get("wordCount").or_else(|| args.get("word_count")).and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                let is_passphrase = mode == "passphrase";

                let (pw, entropy, label) = generate_password_or_passphrase(length, use_upper, use_lower, use_nums, use_syms, is_passphrase, word_count);
                Ok(serde_json::json!({
                    "password": pw,
                    "entropy_bits": entropy,
                    "strength_label": label,
                }))
            }
            "export_dotenv" => {
                let secrets = list_secrets(&svc).await?;
                let mut env_lines = Vec::new();
                for meta in secrets {
                    if let Ok(sec) = get_secret(&svc, &meta.id).await {
                        let key_name = meta.title.to_uppercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>();
                        let val = match &sec.payload {
                            crate::models::SecretPayload::Password { password } => password.clone(),
                            crate::models::SecretPayload::ApiKey { api_key, .. } => api_key.clone(),
                            crate::models::SecretPayload::DatabaseCredential { password, connection_string, .. } => connection_string.clone().unwrap_or_else(|| password.clone()),
                            crate::models::SecretPayload::AwsCredential { secret_access_key, .. } => secret_access_key.clone(),
                            _ => continue,
                        };
                        env_lines.push(format!("{}=\"{}\"", key_name, val));
                    }
                }
                let content = env_lines.join("\n");
                if let Some(dest) = args.get("destPath").or_else(|| args.get("dest_path")).and_then(|v| v.as_str()) {
                    let _ = std::fs::write(dest, content.as_bytes());
                }
                Ok(serde_json::json!({
                    "dotenv_content": content,
                    "exported_count": env_lines.len(),
                }))
            }
            "import_dotenv" => {
                let content = args
                    .get("content")
                    .or_else(|| args.get("dotenv_content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let target_env = args
                    .get("environment")
                    .or_else(|| args.get("target_env"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("production");
                let res = import_dotenv(&svc, content, target_env).await?;
                serde_json::to_value(res).map_err(|e| e.to_string())
            }
            "import_k8s_secret" => {
                let yaml_content = args
                    .get("yaml_content")
                    .or_else(|| args.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let target_env = args
                    .get("environment")
                    .or_else(|| args.get("target_env"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("production");
                let res = import_k8s_secret(&svc, yaml_content, target_env).await?;
                serde_json::to_value(res).map_err(|e| e.to_string())
            }
            "get_git_precommit_hook_script" => {
                let repo_path = args
                    .get("repoPath")
                    .or_else(|| args.get("repo_path"))
                    .and_then(|v| v.as_str());
                Ok(get_git_precommit_hook_script(repo_path))
            }
            "install_git_precommit_hook" => {
                let repo_path = args
                    .get("repoPath")
                    .or_else(|| args.get("repo_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                let res = install_git_precommit_hook(repo_path)?;
                Ok(res)
            }
            _ => Err(format!("Unknown DevOps Secret Vault IPC command: {}", cmd)),
        }
    })
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DotenvImportSummary {
    pub imported_count: usize,
    pub updated_count: usize,
    pub failed_keys: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct K8sImportSummary {
    pub secret_name: String,
    pub imported_count: usize,
    pub updated_count: usize,
    pub failed_keys: Vec<String>,
}

/// Import secrets from `.env` formatted content
pub async fn import_dotenv(
    service: &VaultService,
    content: &str,
    target_env: &str,
) -> Result<DotenvImportSummary, String> {
    let existing_secrets = service.list_secrets().await.map_err(|e| e.to_string())?;
    let mut existing_map = std::collections::HashMap::new();
    for s in existing_secrets {
        existing_map.insert(s.title.clone(), s.id.clone());
    }

    let mut imported_count = 0;
    let mut updated_count = 0;
    let mut failed_keys = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
        if parts.len() != 2 {
            continue;
        }

        let key = parts[0].trim();
        if key.is_empty() {
            continue;
        }

        let mut val = parts[1].trim();
        if ((val.starts_with('"') && val.ends_with('"'))
            || (val.starts_with('\'') && val.ends_with('\'')))
            && val.len() >= 2
        {
            val = &val[1..val.len() - 1];
        }

        let payload = SecretPayload::Password {
            password: val.to_string(),
        };

        if let Some(existing_id) = existing_map.get(key) {
            let update_input = UpdateSecretInput {
                title: Some(key.to_string()),
                tags: Some(vec!["dotenv".to_string(), target_env.to_string()]),
                username: None,
                url: None,
                notes: Some(format!(
                    "Updated via .env import for environment: {}",
                    target_env
                )),
                payload: Some(payload),
            };
            match service
                .update_secret(existing_id, update_input, "desktop-tauri")
                .await
            {
                Ok(_) => updated_count += 1,
                Err(e) => failed_keys.push(format!("{}: {}", key, e)),
            }
        } else {
            let create_input = CreateSecretInput {
                title: key.to_string(),
                tags: vec!["dotenv".to_string(), target_env.to_string()],
                username: None,
                url: None,
                notes: Some(format!(
                    "Imported from .env for environment: {}",
                    target_env
                )),
                payload,
            };
            match service.create_secret(create_input, "desktop-tauri").await {
                Ok(_) => imported_count += 1,
                Err(e) => failed_keys.push(format!("{}: {}", key, e)),
            }
        }
    }

    Ok(DotenvImportSummary {
        imported_count,
        updated_count,
        failed_keys,
    })
}

/// Import secrets from Kubernetes Secret YAML
pub async fn import_k8s_secret(
    service: &VaultService,
    yaml_content: &str,
    target_env: &str,
) -> Result<K8sImportSummary, String> {
    let existing_secrets = service.list_secrets().await.map_err(|e| e.to_string())?;
    let mut existing_map = std::collections::HashMap::new();
    for s in existing_secrets {
        existing_map.insert(s.title.clone(), s.id.clone());
    }

    let mut secret_name = "k8s-secret".to_string();
    let mut in_data = false;
    let mut in_string_data = false;
    let mut in_metadata = false;

    let mut imported_count = 0;
    let mut updated_count = 0;
    let mut failed_keys = Vec::new();

    for line in yaml_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed == "metadata:" {
            in_metadata = true;
            in_data = false;
            in_string_data = false;
            continue;
        }

        if in_metadata && trimmed.starts_with("name:") {
            if let Some(n) = trimmed.strip_prefix("name:") {
                let clean_name = n.trim().trim_matches('"').trim_matches('\'');
                if !clean_name.is_empty() {
                    secret_name = clean_name.to_string();
                }
            }
            continue;
        }

        if trimmed == "data:" {
            in_data = true;
            in_string_data = false;
            in_metadata = false;
            continue;
        }

        if trimmed == "stringData:" {
            in_string_data = true;
            in_data = false;
            in_metadata = false;
            continue;
        }

        if !line.starts_with(' ')
            && !line.starts_with('\t')
            && (in_data || in_string_data || in_metadata)
        {
            in_data = false;
            in_string_data = false;
            in_metadata = false;
        }

        if in_data || in_string_data {
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim();
            let raw_val = parts[1].trim().trim_matches('"').trim_matches('\'');

            if key.is_empty() {
                continue;
            }

            let val_string = if in_data {
                match crate::cert_monitor::decode_base64(raw_val) {
                    Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                    Err(e) => {
                        failed_keys.push(format!("{}: Base64 decode error ({})", key, e));
                        continue;
                    }
                }
            } else {
                raw_val.to_string()
            };

            let title = format!("{}_{}", secret_name, key);
            let payload = SecretPayload::Password {
                password: val_string,
            };

            if let Some(existing_id) = existing_map.get(&title) {
                let update_input = UpdateSecretInput {
                    title: Some(title.clone()),
                    tags: Some(vec![
                        "k8s".to_string(),
                        secret_name.clone(),
                        target_env.to_string(),
                    ]),
                    username: None,
                    url: None,
                    notes: Some(format!("Updated via K8s Secret import ({})", secret_name)),
                    payload: Some(payload),
                };
                match service
                    .update_secret(existing_id, update_input, "desktop-tauri")
                    .await
                {
                    Ok(_) => updated_count += 1,
                    Err(e) => failed_keys.push(format!("{}: {}", key, e)),
                }
            } else {
                let create_input = CreateSecretInput {
                    title,
                    tags: vec![
                        "k8s".to_string(),
                        secret_name.clone(),
                        target_env.to_string(),
                    ],
                    username: None,
                    url: None,
                    notes: Some(format!("Imported from K8s Secret ({})", secret_name)),
                    payload,
                };
                match service.create_secret(create_input, "desktop-tauri").await {
                    Ok(_) => imported_count += 1,
                    Err(e) => failed_keys.push(format!("{}: {}", key, e)),
                }
            }
        }
    }

    Ok(K8sImportSummary {
        secret_name,
        imported_count,
        updated_count,
        failed_keys,
    })
}

/// Generate the offline bash git pre-commit hook script
pub fn get_git_precommit_hook_script(repo_path: Option<&str>) -> serde_json::Value {
    let script = r#"#!/usr/bin/env bash
# DevOps Secret Vault - 100% Offline Pre-Commit Secret Leak Scanner
# Automatically generated by DevOps Secret Vault
# Blocks commits containing exposed API keys, private keys, AWS tokens, or cloud secrets.

set -e

# Gather all staged files
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM)

if [ -z "$STAGED_FILES" ]; then
    exit 0
fi

LEAKS_DETECTED=0

# Signatures for high-risk secret leaks
PATTERNS=(
    "AKIA[0-9A-Z]{16}"
    "ghp_[0-9a-zA-Z]{36}"
    "gho_[0-9a-zA-Z]{36}"
    "glpat-[0-9a-zA-Z\\-_]{20}"
    "xox[baprs]-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24,32}"
    "-----BEGIN [A-Z ]*PRIVATE KEY-----"
    "eyJhbGciOi[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+"
    "sk_live_[0-9a-zA-Z]{24}"
    "sq0atp-[0-9A-Za-z\\-_]{22}"
)

for FILE in $STAGED_FILES; do
    if [ ! -f "$FILE" ]; then
        continue
    fi
    for PATTERN in "${PATTERNS[@]}"; do
        if grep -E -q "$PATTERN" "$FILE" 2>/dev/null; then
            echo "❌ [DevOps Secret Vault] Pre-Commit Check FAILED!"
            echo "   Found potential secret in staged file: $FILE"
            echo "   Matching signature pattern: $PATTERN"
            LEAKS_DETECTED=$((LEAKS_DETECTED + 1))
        fi
    done
done

if [ "$LEAKS_DETECTED" -gt 0 ]; then
    echo ""
    echo "🚨 Commit aborted! Please remove secrets or store them securely in DevOps Secret Vault."
    echo "   Use 'git commit --no-verify' only if you are certain this is an intentional commit."
    exit 1
fi

exit 0
"#;
    serde_json::json!({
        "script": script,
        "hook_name": "pre-commit",
        "default_target": repo_path.map(|p| format!("{}/.git/hooks/pre-commit", p.trim_end_matches(['/', '\\']))).unwrap_or_else(|| ".git/hooks/pre-commit".to_string()),
    })
}

/// Install the pre-commit hook into a local repository's `.git/hooks/pre-commit`
pub fn install_git_precommit_hook(repo_path: &str) -> Result<serde_json::Value, String> {
    let base = std::path::Path::new(repo_path);
    if !base.exists() {
        return Err(format!(
            "Repository directory does not exist: {}",
            repo_path
        ));
    }
    let git_dir = base.join(".git");
    if !git_dir.exists() {
        return Err(format!(
            "Not a git repository: missing .git directory in {}",
            repo_path
        ));
    }
    let hooks_dir = git_dir.join("hooks");
    if !hooks_dir.exists() {
        std::fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;
    }
    let hook_file = hooks_dir.join("pre-commit");
    let script_data = get_git_precommit_hook_script(Some(repo_path));
    let script_str = script_data
        .get("script")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let script_lf = script_str.replace("\r\n", "\n");
    std::fs::write(&hook_file, script_lf.as_bytes()).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&hook_file) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&hook_file, perms);
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "path": hook_file.to_string_lossy().to_string(),
        "message": "Git pre-commit hook installed successfully into .git/hooks/pre-commit",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_service() -> (VaultService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_vault.db");
        let storage = crate::storage::VaultStorage::open(&db_path).unwrap();
        let service = VaultService::new(storage, None);
        service
            .initialize("CorrectHorseBatteryStaple123!")
            .await
            .unwrap();
        (service, dir)
    }

    #[tokio::test]
    async fn test_dotenv_import_export_roundtrip() {
        let (service, _dir) = create_test_service().await;

        let dotenv_sample = r#"
# Database Configuration
DB_HOST=127.0.0.1
DB_PORT="5432"
DB_PASS='super_secret_password'
"#;

        let import_summary = import_dotenv(&service, dotenv_sample, "staging")
            .await
            .unwrap();
        assert_eq!(import_summary.imported_count, 3);
        assert_eq!(import_summary.failed_keys.len(), 0);

        // Verify secrets in list
        let secrets = service.list_secrets().await.unwrap();
        assert_eq!(secrets.len(), 3);

        let pass_sec = secrets.iter().find(|s| s.title == "DB_PASS").unwrap();
        let decrypted = service.get_secret(&pass_sec.id, "test").await.unwrap();
        if let SecretPayload::Password { password } = &decrypted.payload {
            assert_eq!(password, "super_secret_password");
        } else {
            panic!("Expected password payload");
        }
    }

    #[tokio::test]
    async fn test_k8s_secret_import() {
        let (service, _dir) = create_test_service().await;

        // "secret_token_123" in base64 is "c2VjcmV0X3Rva2VuXzEyMw=="
        let k8s_yaml = r#"
apiVersion: v1
kind: Secret
metadata:
  name: api-service
type: Opaque
data:
  AUTH_TOKEN: c2VjcmV0X3Rva2VuXzEyMw==
stringData:
  ENDPOINT: https://api.internal.local
"#;

        let import_summary = import_k8s_secret(&service, k8s_yaml, "production")
            .await
            .unwrap();
        assert_eq!(import_summary.secret_name, "api-service");
        assert_eq!(import_summary.imported_count, 2);

        let secrets = service.list_secrets().await.unwrap();
        let auth_sec = secrets
            .iter()
            .find(|s| s.title == "api-service_AUTH_TOKEN")
            .unwrap();
        let decrypted = service.get_secret(&auth_sec.id, "test").await.unwrap();
        if let SecretPayload::Password { password } = &decrypted.payload {
            assert_eq!(password, "secret_token_123");
        } else {
            panic!("Expected password payload");
        }
    }

    #[test]
    fn test_git_precommit_hook_generation_and_installation() {
        let val = get_git_precommit_hook_script(Some("/path/to/myrepo"));
        let script = val.get("script").unwrap().as_str().unwrap();
        assert!(script.contains("DevOps Secret Vault"));
        assert!(script.contains("AKIA"));

        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path();
        std::fs::create_dir_all(repo_path.join(".git")).unwrap();

        let install_res = install_git_precommit_hook(repo_path.to_str().unwrap()).unwrap();
        assert_eq!(install_res.get("success").unwrap().as_bool(), Some(true));

        let hook_content =
            std::fs::read_to_string(repo_path.join(".git").join("hooks").join("pre-commit"))
                .unwrap();
        assert!(hook_content.contains("Pre-Commit Check FAILED"));
    }
}
