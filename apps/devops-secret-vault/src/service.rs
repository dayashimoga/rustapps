#![forbid(unsafe_code)]
//! DevOps Secret Vault - Main Application Service

use shared_encryption::Argon2Params;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::backup::{
    export_vault_backup, read_vault_backup, BackupExportSummary, BackupImportSummary,
};
use crate::biometrics::BiometricService;
use crate::cert_monitor::{inspect_x509_pem, CertInspectionReport};
use crate::clipboard::SecureClipboard;
use crate::crypto::{
    decrypt_secret_payload, encrypt_secret_payload, VaultKeyDerivationMetadata, VaultMasterKey,
};
use crate::error::VaultError;
use crate::models::{
    generate_uuid, CreateSecretInput, DecryptedSecret, EncryptedSecretRecord, SecretAuditAction,
    SecretAuditEntry, SecretMetadata, SecretPayload, SecretSearchResult, UpdateSecretInput,
};
use crate::risk_scoring::{evaluate_vault_health, VaultHealthReport};
use crate::search::VaultSearchIndex;
use crate::storage::VaultStorage;
use crate::totp::{TotpAlgorithm, TotpCodeResponse, TotpConfig};

/// Active in-memory session when the vault is unlocked
struct UnlockedSession {
    master_key: VaultMasterKey,
    search_index: VaultSearchIndex,
}

/// Failed unlock rate-limiter state
#[derive(Debug, Default)]
struct FailedUnlockTracker {
    failed_attempts: u32,
    last_failed_at: Option<std::time::Instant>,
}

/// Unified Vault Service coordinating storage, crypto, ephemeral search, and clipboard
#[derive(Clone)]
pub struct VaultService {
    storage: VaultStorage,
    session: Arc<RwLock<Option<UnlockedSession>>>,
    clipboard: SecureClipboard,
    biometrics: BiometricService,
    argon_params: Argon2Params,
    failed_unlocks: Arc<tokio::sync::Mutex<FailedUnlockTracker>>,
}

impl VaultService {
    /// Create a new Vault Service instance
    pub fn new(storage: VaultStorage, argon_params: Option<Argon2Params>) -> Self {
        Self {
            storage,
            session: Arc::new(RwLock::new(None)),
            clipboard: SecureClipboard::default(),
            biometrics: BiometricService::default(),
            argon_params: argon_params.unwrap_or_default(),
            failed_unlocks: Arc::new(tokio::sync::Mutex::new(FailedUnlockTracker::default())),
        }
    }

    /// Create with a custom biometric service (e.g. for testing)
    pub fn with_biometrics(
        storage: VaultStorage,
        biometrics: BiometricService,
        argon_params: Option<Argon2Params>,
    ) -> Self {
        Self {
            storage,
            session: Arc::new(RwLock::new(None)),
            clipboard: SecureClipboard::default(),
            biometrics,
            argon_params: argon_params.unwrap_or_default(),
            failed_unlocks: Arc::new(tokio::sync::Mutex::new(FailedUnlockTracker::default())),
        }
    }

    async fn check_unlock_rate_limit(&self) -> Result<(), VaultError> {
        let tracker = self.failed_unlocks.lock().await;
        if tracker.failed_attempts >= 3 {
            if let Some(last_failed) = tracker.last_failed_at {
                let backoff_secs = (1u64 << (tracker.failed_attempts - 3)).min(60);
                let elapsed = last_failed.elapsed().as_secs();
                if elapsed < backoff_secs {
                    return Err(VaultError::RateLimited {
                        retry_after_secs: backoff_secs - elapsed,
                    });
                }
            }
        }
        Ok(())
    }

    async fn record_successful_unlock(&self) {
        let mut tracker = self.failed_unlocks.lock().await;
        tracker.failed_attempts = 0;
        tracker.last_failed_at = None;
    }

    async fn record_failed_unlock(&self) {
        let mut tracker = self.failed_unlocks.lock().await;
        tracker.failed_attempts += 1;
        tracker.last_failed_at = Some(std::time::Instant::now());
    }

    async fn activate_session(&self, master_key: VaultMasterKey) -> Result<(), VaultError> {
        let search_index = VaultSearchIndex::new()?;
        let existing_metadata = self.storage.list_all_metadata_decrypted(&master_key)?;
        search_index.index_batch(&existing_metadata)?;

        let mut session_guard = self.session.write().await;
        *session_guard = Some(UnlockedSession {
            master_key,
            search_index,
        });
        Ok(())
    }

    /// Check if the vault has been initialized with a master password
    pub fn is_initialized(&self) -> Result<bool, VaultError> {
        let meta = self.storage.get_kdf_metadata()?;
        Ok(meta.is_some())
    }

    /// Initialize the vault with a master password
    pub async fn initialize(&self, master_password: &str) -> Result<(), VaultError> {
        if self.is_initialized()? {
            return Err(VaultError::AlreadyInitialized);
        }
        if master_password.trim().is_empty() {
            return Err(VaultError::ValidationError(
                "Master password cannot be empty".to_string(),
            ));
        }

        let (meta, master_key) =
            VaultKeyDerivationMetadata::create(master_password, &self.argon_params)?;
        self.storage.set_kdf_metadata(&meta)?;

        // Immediately unlock into an active session
        self.activate_session(master_key).await?;

        Ok(())
    }

    /// Unlock the vault using the master password
    pub async fn unlock(&self, master_password: &str) -> Result<(), VaultError> {
        // 1. Check rate-limit backoff on repeated failed unlock attempts (AUD-13)
        self.check_unlock_rate_limit().await?;

        let meta = self
            .storage
            .get_kdf_metadata()?
            .ok_or(VaultError::NotInitialized)?;

        let master_key = match meta.unlock(master_password) {
            Ok(k) => {
                self.record_successful_unlock().await;
                k
            }
            Err(e) => {
                self.record_failed_unlock().await;
                return Err(e);
            }
        };

        self.activate_session(master_key).await
    }

    /// Check whether native OS biometrics are available on this machine
    pub fn is_biometric_available(&self) -> bool {
        self.biometrics.is_available()
    }

    /// Enable biometric unlock by saving active master key in OS keyring
    pub async fn enable_biometric_unlock(&self) -> Result<(), VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;
        if !self.biometrics.is_available() {
            return Err(VaultError::Biometric(
                "Biometric hardware not available on this platform".to_string(),
            ));
        }
        self.biometrics.store_biometric_key(&session.master_key)?;
        Ok(())
    }

    /// Disable biometric unlock by clearing stored key from OS keyring
    pub fn disable_biometric_unlock(&self) -> Result<(), VaultError> {
        self.biometrics.clear_biometric_key()
    }

    /// Unlock vault using native OS biometric authentication and retrieved key
    pub async fn unlock_with_biometrics(&self) -> Result<(), VaultError> {
        self.check_unlock_rate_limit().await?;

        if !self.biometrics.is_available() {
            return Err(VaultError::Biometric(
                "Biometrics not available".to_string(),
            ));
        }

        let auth_ok = self
            .biometrics
            .authenticate("DevOps Secret Vault Biometric Unlock")?;
        if !auth_ok {
            self.record_failed_unlock().await;
            return Err(VaultError::Biometric(
                "Biometric verification cancelled or rejected".to_string(),
            ));
        }

        let stored_key = self.biometrics.retrieve_biometric_key()?.ok_or_else(|| {
            VaultError::Biometric(
                "No biometric key found in OS keyring. Unlock with master password and enable biometrics first.".to_string(),
            )
        })?;

        let meta = self
            .storage
            .get_kdf_metadata()?
            .ok_or(VaultError::NotInitialized)?;

        let verified_key = match meta.verify_master_key(stored_key) {
            Ok(k) => {
                self.record_successful_unlock().await;
                k
            }
            Err(e) => {
                self.record_failed_unlock().await;
                return Err(e);
            }
        };

        self.activate_session(verified_key).await
    }

    /// Split active vault master key into k-of-n Shamir shares (Business Tier)
    pub async fn split_master_key(
        &self,
        threshold: u8,
        total_shares: u8,
        tier_gate: &shared_licensing::TierGate,
    ) -> Result<Vec<crate::shamir::ShamirShare>, VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;
        crate::shamir::split_master_key(&session.master_key, threshold, total_shares, tier_gate)
    }

    /// Unlock vault by reconstructing master key from Shamir shares (Business Tier)
    pub async fn unlock_with_shamir_shares(
        &self,
        shares: &[crate::shamir::ShamirShare],
    ) -> Result<(), VaultError> {
        self.check_unlock_rate_limit().await?;

        let key_bytes = crate::shamir::reconstruct_secret(shares)?;
        if key_bytes.len() != 32 {
            self.record_failed_unlock().await;
            return Err(VaultError::Shamir(
                "Reconstructed secret length is not 32 bytes".to_string(),
            ));
        }

        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&key_bytes);
        let candidate_key = VaultMasterKey(key_arr);

        let meta = self
            .storage
            .get_kdf_metadata()?
            .ok_or(VaultError::NotInitialized)?;

        let verified_key = match meta.verify_master_key(candidate_key) {
            Ok(k) => {
                self.record_successful_unlock().await;
                k
            }
            Err(e) => {
                self.record_failed_unlock().await;
                return Err(e);
            }
        };

        self.activate_session(verified_key).await
    }

    /// Lock the vault, immediately zeroizing in-memory keys and destroying the RAM search index
    pub async fn lock(&self) -> Result<(), VaultError> {
        let mut session_guard = self.session.write().await;
        *session_guard = None;
        self.clipboard.clear().await?;
        Ok(())
    }

    /// Check if vault is currently locked
    pub async fn is_locked(&self) -> bool {
        let session_guard = self.session.read().await;
        session_guard.is_none()
    }

    /// Access biometric service
    pub fn biometrics(&self) -> &BiometricService {
        &self.biometrics
    }

    /// Create a new secret
    pub async fn create_secret(
        &self,
        input: CreateSecretInput,
        client: &str,
    ) -> Result<SecretMetadata, VaultError> {
        input.payload.validate()?;
        let now = now_timestamp();
        let secret_id = generate_uuid();

        let metadata = SecretMetadata {
            id: secret_id.clone(),
            title: input.title,
            secret_type: input.payload.secret_type(),
            tags: input.tags,
            username: input.username,
            url: input.url,
            notes: input.notes,
            version: 1,
            created_at: now,
            updated_at: now,
        };
        metadata.validate()?;

        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().ok_or(VaultError::Locked)?;

        // Encrypt payload with AES-256-GCM
        let encrypted_payload = encrypt_secret_payload(&session.master_key, &input.payload)?;

        let record = EncryptedSecretRecord {
            metadata: metadata.clone(),
            encrypted_payload,
        };

        self.storage
            .insert_secret_encrypted(&record, &session.master_key)?;
        session.search_index.index_secret(&metadata)?;

        // Audit log
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: secret_id.clone(),
            action: SecretAuditAction::Created,
            timestamp: now,
            client: client.to_string(),
            prev_hash: String::new(),
            entry_hash: String::new(),
        };
        let _ = self.storage.insert_audit_log(&audit);

        Ok(metadata)
    }

    /// Retrieve and decrypt a secret by ID
    pub async fn get_secret(&self, id: &str, client: &str) -> Result<DecryptedSecret, VaultError> {
        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().ok_or(VaultError::Locked)?;

        let record = self
            .storage
            .get_secret_decrypted(id, &session.master_key)?
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;

        let payload = decrypt_secret_payload(&session.master_key, &record.encrypted_payload)?;

        // Audit log
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: id.to_string(),
            action: SecretAuditAction::Read,
            timestamp: now_timestamp(),
            client: client.to_string(),
            prev_hash: String::new(),
            entry_hash: String::new(),
        };
        let _ = self.storage.insert_audit_log(&audit);

        Ok(DecryptedSecret {
            metadata: record.metadata,
            payload,
        })
    }

    /// Retrieve secret metadata without decrypting the payload
    pub async fn get_secret_metadata(&self, id: &str) -> Result<SecretMetadata, VaultError> {
        let session_guard = self.session.read().await;
        if let Some(ref session) = *session_guard {
            let record = self
                .storage
                .get_secret_decrypted(id, &session.master_key)?
                .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;
            Ok(record.metadata)
        } else {
            let record = self
                .storage
                .get_secret(id)?
                .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;
            Ok(record.metadata)
        }
    }

    /// List all secret metadata records
    pub async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, VaultError> {
        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().ok_or(VaultError::Locked)?;
        self.storage
            .list_all_metadata_decrypted(&session.master_key)
    }

    /// Update an existing secret
    pub async fn update_secret(
        &self,
        id: &str,
        input: UpdateSecretInput,
        client: &str,
    ) -> Result<SecretMetadata, VaultError> {
        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().ok_or(VaultError::Locked)?;

        let existing = self
            .storage
            .get_secret_decrypted(id, &session.master_key)?
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;

        let now = now_timestamp();
        let new_title = input.title.unwrap_or(existing.metadata.title);
        let new_tags = input.tags.unwrap_or(existing.metadata.tags);
        let new_username = input.username.or(existing.metadata.username);
        let new_url = input.url.or(existing.metadata.url);
        let new_notes = input.notes.or(existing.metadata.notes);

        let (new_payload_bytes, new_type) = if let Some(payload) = input.payload {
            payload.validate()?;
            let encrypted = encrypt_secret_payload(&session.master_key, &payload)?;
            (encrypted, payload.secret_type())
        } else {
            (existing.encrypted_payload, existing.metadata.secret_type)
        };

        let updated_metadata = SecretMetadata {
            id: id.to_string(),
            title: new_title,
            secret_type: new_type,
            tags: new_tags,
            username: new_username,
            url: new_url,
            notes: new_notes,
            version: existing.metadata.version + 1,
            created_at: existing.metadata.created_at,
            updated_at: now,
        };
        updated_metadata.validate()?;

        let updated_record = EncryptedSecretRecord {
            metadata: updated_metadata.clone(),
            encrypted_payload: new_payload_bytes,
        };

        self.storage
            .update_secret_encrypted(&updated_record, &session.master_key)?;
        session.search_index.index_secret(&updated_metadata)?;

        // Audit log
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: id.to_string(),
            action: SecretAuditAction::Updated,
            timestamp: now,
            client: client.to_string(),
            prev_hash: String::new(),
            entry_hash: String::new(),
        };
        let _ = self.storage.insert_audit_log(&audit);

        Ok(updated_metadata)
    }

    /// Delete a secret
    pub async fn delete_secret(&self, id: &str, client: &str) -> Result<(), VaultError> {
        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().ok_or(VaultError::Locked)?;

        let deleted = self.storage.delete_secret(id)?;
        if !deleted {
            return Err(VaultError::SecretNotFound(id.to_string()));
        }

        session.search_index.remove_secret(id)?;

        // Audit log
        let audit = SecretAuditEntry::new(
            generate_uuid(),
            id.to_string(),
            SecretAuditAction::Deleted,
            now_timestamp(),
            client.to_string(),
        );
        let _ = self.storage.insert_audit_log(&audit);

        Ok(())
    }

    /// Search secrets using the sub-50ms ephemeral Tantivy index
    pub async fn search_secrets(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SecretSearchResult>, VaultError> {
        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().ok_or(VaultError::Locked)?;
        session.search_index.search(query, limit)
    }

    /// Copy secret value to secure clipboard with auto-clear countdown
    pub async fn copy_to_clipboard(
        &self,
        id: &str,
        custom_ttl_secs: Option<u32>,
        client: &str,
    ) -> Result<u32, VaultError> {
        let decrypted = self.get_secret(id, client).await?;
        let secret_value = decrypted.payload.primary_secret_value();

        let ttl = self.clipboard.copy(secret_value, custom_ttl_secs).await?;

        // Audit log
        let audit = SecretAuditEntry::new(
            generate_uuid(),
            id.to_string(),
            SecretAuditAction::CopiedToClipboard,
            now_timestamp(),
            client.to_string(),
        );
        let _ = self.storage.insert_audit_log(&audit);

        Ok(ttl)
    }

    /// Get remaining seconds on auto-clearing clipboard
    pub async fn clipboard_remaining_seconds(&self) -> Option<u32> {
        self.clipboard.remaining_seconds().await
    }

    /// Clear clipboard immediately
    pub async fn clear_clipboard(&self) -> Result<(), VaultError> {
        self.clipboard.clear().await
    }

    /// Retrieve audit logs
    pub async fn get_audit_logs(
        &self,
        secret_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SecretAuditEntry>, VaultError> {
        if self.is_locked().await {
            return Err(VaultError::Locked);
        }
        self.storage.list_audit_logs(secret_id, limit)
    }

    /// Peek clipboard value for tests
    pub async fn peek_clipboard_for_test(&self) -> Option<String> {
        self.clipboard.peek_for_test().await
    }

    /// Generate a 5-dimensional Vault Security Health & Risk Report
    pub async fn get_health_report(&self) -> Result<VaultHealthReport, VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;

        let encrypted_records = self
            .storage
            .list_all_encrypted_secrets_decrypted(&session.master_key)?;
        let mut decrypted_secrets = Vec::with_capacity(encrypted_records.len());

        for record in encrypted_records {
            let payload = decrypt_secret_payload(&session.master_key, &record.encrypted_payload)?;
            decrypted_secrets.push(DecryptedSecret {
                metadata: record.metadata,
                payload,
            });
        }

        let report = evaluate_vault_health(&decrypted_secrets);
        Ok(report)
    }

    /// Generate real-time RFC 6238 TOTP code for a secret
    pub async fn generate_totp_code(
        &self,
        secret_id: &str,
    ) -> Result<TotpCodeResponse, VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;

        let record = self
            .storage
            .get_secret_decrypted(secret_id, &session.master_key)?
            .ok_or_else(|| VaultError::SecretNotFound(secret_id.to_string()))?;

        let payload = decrypt_secret_payload(&session.master_key, &record.encrypted_payload)?;
        match &payload {
            SecretPayload::Totp {
                secret_base32,
                issuer,
                account_name,
                algorithm,
                digits,
                period_seconds,
            } => {
                let alg = algorithm
                    .as_deref()
                    .map(TotpAlgorithm::from_str_lossy)
                    .unwrap_or(TotpAlgorithm::Sha1);
                let config = TotpConfig {
                    secret_base32: secret_base32.clone(),
                    issuer: issuer.clone(),
                    account_name: account_name.clone(),
                    algorithm: alg,
                    digits: digits.unwrap_or(6),
                    step_seconds: period_seconds.unwrap_or(30),
                };
                config.generate_current()
            }
            _ => Err(VaultError::ValidationError(
                "Secret is not a TOTP credential".to_string(),
            )),
        }
    }

    /// Inspect an X.509 Certificate secret
    pub async fn inspect_certificate(
        &self,
        secret_id: &str,
    ) -> Result<CertInspectionReport, VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;

        let record = self
            .storage
            .get_secret_decrypted(secret_id, &session.master_key)?
            .ok_or_else(|| VaultError::SecretNotFound(secret_id.to_string()))?;

        let payload = decrypt_secret_payload(&session.master_key, &record.encrypted_payload)?;
        match &payload {
            SecretPayload::Certificate {
                certificate_pem, ..
            } => inspect_x509_pem(certificate_pem),
            _ => Err(VaultError::ValidationError(
                "Secret is not an X.509 certificate".to_string(),
            )),
        }
    }

    /// Export an authenticated encrypted .vaultbackup archive
    pub async fn export_encrypted_backup(
        &self,
        passphrase: &str,
        output_path: &Path,
    ) -> Result<BackupExportSummary, VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;

        let encrypted_records = self
            .storage
            .list_all_encrypted_secrets_decrypted(&session.master_key)?;
        let mut decrypted_secrets = Vec::with_capacity(encrypted_records.len());

        for record in encrypted_records {
            let payload = decrypt_secret_payload(&session.master_key, &record.encrypted_payload)?;
            decrypted_secrets.push(DecryptedSecret {
                metadata: record.metadata,
                payload,
            });
        }

        export_vault_backup(
            &decrypted_secrets,
            passphrase,
            output_path,
            Some(self.argon_params.clone()),
        )
    }

    /// Import and restore secrets from an authenticated encrypted .vaultbackup archive
    pub async fn import_encrypted_backup(
        &self,
        passphrase: &str,
        input_path: &Path,
        overwrite: bool,
    ) -> Result<BackupImportSummary, VaultError> {
        let guard = self.session.read().await;
        let session = guard.as_ref().ok_or(VaultError::Locked)?;

        let envelope = read_vault_backup(input_path, passphrase)?;

        let mut total_imported = 0;
        let mut total_skipped = 0;
        let mut total_overwritten = 0;

        for secret in envelope.secrets {
            let exists = self.storage.get_secret(&secret.metadata.id)?.is_some();
            if exists && !overwrite {
                total_skipped += 1;
                continue;
            }

            secret.metadata.validate()?;
            secret.payload.validate()?;

            let encrypted_payload = encrypt_secret_payload(&session.master_key, &secret.payload)?;
            let record = EncryptedSecretRecord {
                metadata: secret.metadata.clone(),
                encrypted_payload,
            };

            if exists {
                self.storage
                    .update_secret_encrypted(&record, &session.master_key)?;
                total_overwritten += 1;
            } else {
                self.storage
                    .insert_secret_encrypted(&record, &session.master_key)?;
                total_imported += 1;
            }

            let _ = session.search_index.index_secret(&secret.metadata);
        }

        Ok(BackupImportSummary {
            total_imported,
            total_skipped,
            total_overwritten,
            original_exported_at: envelope.exported_at,
        })
    }

    /// Verify the complete cryptographic SHA-256 hash chain of the audit log
    pub async fn verify_audit_chain(&self) -> Result<bool, VaultError> {
        self.storage.verify_audit_chain()
    }
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SecretPayload;

    #[tokio::test]
    async fn test_vault_service_lifecycle() {
        let storage = VaultStorage::open_in_memory().unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        assert!(!service.is_initialized().unwrap());

        // Initialize vault
        service
            .initialize("CorrectMasterPassword123!")
            .await
            .unwrap();
        assert!(service.is_initialized().unwrap());
        assert!(!service.is_locked().await);

        // Create secret
        let input = CreateSecretInput {
            title: "Production SSH Bastion".to_string(),
            tags: vec!["ssh".to_string(), "prod".to_string()],
            username: Some("deployer".to_string()),
            url: Some("bastion.infra.com".to_string()),
            notes: Some("Port 2222".to_string()),
            payload: SecretPayload::SshKey {
                private_key:
                    "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----"
                        .to_string(),
                public_key: Some("ssh-ed25519 AAAAC3...".to_string()),
                passphrase: None,
            },
        };

        let meta = service.create_secret(input, "test_client").await.unwrap();
        assert_eq!(meta.title, "Production SSH Bastion");

        // Read secret
        let decrypted = service.get_secret(&meta.id, "test_client").await.unwrap();
        assert_eq!(decrypted.metadata.title, "Production SSH Bastion");
        match &decrypted.payload {
            SecretPayload::SshKey { private_key, .. } => {
                assert!(private_key.contains("OPENSSH PRIVATE KEY"));
            }
            _ => panic!("Expected SshKey payload"),
        }

        // Search secret
        let search_hits = service.search_secrets("bastion", 10).await.unwrap();
        assert_eq!(search_hits.len(), 1);
        assert_eq!(search_hits[0].id, meta.id);

        // Copy to clipboard
        let ttl = service
            .copy_to_clipboard(&meta.id, Some(30), "test_client")
            .await
            .unwrap();
        assert_eq!(ttl, 30);
        let peeked = service.peek_clipboard_for_test().await;
        assert!(peeked.is_some());

        // Lock vault
        service.lock().await.unwrap();
        assert!(service.is_locked().await);
        assert_eq!(service.peek_clipboard_for_test().await, None);

        // Trying to read secret while locked must fail
        assert!(service.get_secret(&meta.id, "test_client").await.is_err());

        // Unlock with wrong password must fail
        assert!(service.unlock("WrongPassword").await.is_err());
        assert!(service.is_locked().await);

        // Unlock with correct password must succeed
        service.unlock("CorrectMasterPassword123!").await.unwrap();
        assert!(!service.is_locked().await);

        // Search works immediately after unlock
        let hits_after_unlock = service.search_secrets("bastion", 10).await.unwrap();
        assert_eq!(hits_after_unlock.len(), 1);

        // Verify audit logs
        let audit_logs = service.get_audit_logs(Some(&meta.id), 10).await.unwrap();
        assert!(audit_logs.len() >= 3); // Created, Read, CopiedToClipboard
        assert!(service.verify_audit_chain().await.unwrap());
    }

    #[tokio::test]
    async fn test_failed_unlock_rate_limiting_backoff() {
        let storage = VaultStorage::open_in_memory().unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        service.initialize("SecretPassword2026!").await.unwrap();
        service.lock().await.unwrap();

        // 1st fail
        let err1 = service.unlock("Wrong1").await.unwrap_err();
        assert!(matches!(err1, VaultError::InvalidMasterPassword));

        // 2nd fail
        let err2 = service.unlock("Wrong2").await.unwrap_err();
        assert!(matches!(err2, VaultError::InvalidMasterPassword));

        // 3rd fail -> triggers rate limit tracker
        let err3 = service.unlock("Wrong3").await.unwrap_err();
        assert!(matches!(err3, VaultError::InvalidMasterPassword));

        // Immediate 4th attempt must be blocked by rate limiter
        let rate_err = service.unlock("SecretPassword2026!").await.unwrap_err();
        assert!(
            matches!(rate_err, VaultError::RateLimited { .. }),
            "Immediate attempt after 3 failures must be rate limited"
        );
    }

    #[tokio::test]
    async fn test_biometric_unlock_lifecycle() {
        let storage = VaultStorage::open_in_memory().unwrap();
        let test_keyring = Arc::new(platform_common::InMemoryKeyring::for_test(true, true));
        let biometrics = BiometricService::new(test_keyring);
        let service = VaultService::with_biometrics(
            storage,
            biometrics,
            Some(Argon2Params::fast_for_tests()),
        );

        service.initialize("MasterPassword123!").await.unwrap();
        assert!(!service.is_locked().await);

        // Enable biometric unlock while vault is open
        service.enable_biometric_unlock().await.unwrap();

        // Lock vault
        service.lock().await.unwrap();
        assert!(service.is_locked().await);

        // Unlock with biometrics
        service.unlock_with_biometrics().await.unwrap();
        assert!(!service.is_locked().await);

        // Disable biometrics
        service.disable_biometric_unlock().unwrap();
        service.lock().await.unwrap();

        // Unlock with biometrics must now fail since key was removed
        assert!(service.unlock_with_biometrics().await.is_err());
    }

    #[tokio::test]
    async fn test_shamir_master_key_split_and_unlock() {
        let storage = VaultStorage::open_in_memory().unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        service.initialize("PassphraseShamir99!").await.unwrap();

        // Create a test secret
        service
            .create_secret(
                CreateSecretInput {
                    title: "Kubernetes Admin Token".to_string(),
                    tags: vec!["k8s".to_string()],
                    username: None,
                    url: None,
                    notes: None,
                    payload: SecretPayload::ApiKey {
                        api_key: "k8s-cluster-token-12345".to_string(),
                        api_secret: None,
                        endpoint_url: None,
                    },
                },
                "test",
            )
            .await
            .unwrap();

        // Split master key 2-of-3
        let gate = shared_licensing::TierGate::enterprise();
        let shares = service.split_master_key(2, 3, &gate).await.unwrap();
        assert_eq!(shares.len(), 3);

        // Lock vault
        service.lock().await.unwrap();
        assert!(service.is_locked().await);

        // Unlock with 2 shares (e.g. shares[0] and shares[2])
        let recovery_shares = vec![shares[0].clone(), shares[2].clone()];
        service
            .unlock_with_shamir_shares(&recovery_shares)
            .await
            .unwrap();
        assert!(!service.is_locked().await);

        // Search immediately returns data
        let hits = service.search_secrets("Kubernetes", 5).await.unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[tokio::test]
    async fn test_encrypted_backup_conflict_resolution() {
        let storage = VaultStorage::open_in_memory().unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        service.initialize("MasterPassword123!").await.unwrap();

        // Create 2 initial secrets
        let sec1 = service
            .create_secret(
                CreateSecretInput {
                    title: "DB Master Password".to_string(),
                    tags: vec!["db".to_string()],
                    username: Some("admin".to_string()),
                    url: None,
                    notes: None,
                    payload: SecretPayload::Password {
                        password: "InitialPassword123!".to_string(),
                    },
                },
                "test",
            )
            .await
            .unwrap();

        let _sec2 = service
            .create_secret(
                CreateSecretInput {
                    title: "API Gateway Token".to_string(),
                    tags: vec!["api".to_string()],
                    username: None,
                    url: None,
                    notes: None,
                    payload: SecretPayload::ApiKey {
                        api_key: "gw-token-999".to_string(),
                        api_secret: None,
                        endpoint_url: None,
                    },
                },
                "test",
            )
            .await
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let backup_file = temp_dir.path().join("vault_backup.vaultbackup");

        // Export backup
        let export_summary = service
            .export_encrypted_backup("BackupPass123!", &backup_file)
            .await
            .unwrap();
        assert_eq!(export_summary.total_secrets, 2);

        // Modify sec1 in the vault
        service
            .update_secret(
                &sec1.id,
                crate::models::UpdateSecretInput {
                    title: Some("DB Master Password (MODIFIED)".to_string()),
                    tags: Some(vec!["db".to_string(), "modified".to_string()]),
                    username: Some("admin".to_string()),
                    url: None,
                    notes: None,
                    payload: Some(SecretPayload::Password {
                        password: "ModifiedPassword456!".to_string(),
                    }),
                },
                "test",
            )
            .await
            .unwrap();

        // Check modified title
        let current_sec1 = service.get_secret(&sec1.id, "test").await.unwrap();
        assert_eq!(current_sec1.metadata.title, "DB Master Password (MODIFIED)");

        // 1. Import with overwrite = false (skip on conflict)
        let import_skip = service
            .import_encrypted_backup("BackupPass123!", &backup_file, false)
            .await
            .unwrap();
        assert_eq!(import_skip.total_imported, 0);
        assert_eq!(import_skip.total_overwritten, 0);
        assert_eq!(import_skip.total_skipped, 2);

        // Verify sec1 still has modified title
        let current_sec1_after_skip = service.get_secret(&sec1.id, "test").await.unwrap();
        assert_eq!(
            current_sec1_after_skip.metadata.title,
            "DB Master Password (MODIFIED)"
        );

        // 2. Import with overwrite = true (replace on conflict)
        let import_overwrite = service
            .import_encrypted_backup("BackupPass123!", &backup_file, true)
            .await
            .unwrap();
        assert_eq!(import_overwrite.total_imported, 0);
        assert_eq!(import_overwrite.total_overwritten, 2);
        assert_eq!(import_overwrite.total_skipped, 0);

        // Verify sec1 restored to original title from backup
        let current_sec1_after_overwrite = service.get_secret(&sec1.id, "test").await.unwrap();
        assert_eq!(
            current_sec1_after_overwrite.metadata.title,
            "DB Master Password"
        );
    }
}
