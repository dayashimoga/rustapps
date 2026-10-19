#![forbid(unsafe_code)]
//! DevOps Secret Vault - Main Application Service

use shared_encryption::Argon2Params;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::biometrics::BiometricService;
use crate::clipboard::SecureClipboard;
use crate::crypto::{
    decrypt_secret_payload, encrypt_secret_payload, VaultKeyDerivationMetadata, VaultMasterKey,
};
use crate::error::VaultError;
use crate::models::{
    generate_uuid, CreateSecretInput, DecryptedSecret, EncryptedSecretRecord, SecretAuditAction,
    SecretAuditEntry, SecretMetadata, SecretSearchResult, UpdateSecretInput,
};
use crate::search::VaultSearchIndex;
use crate::storage::VaultStorage;

/// Active in-memory session when the vault is unlocked
struct UnlockedSession {
    master_key: VaultMasterKey,
    search_index: VaultSearchIndex,
}

/// Unified Vault Service coordinating storage, crypto, ephemeral search, and clipboard
#[derive(Clone)]
pub struct VaultService {
    storage: VaultStorage,
    session: Arc<RwLock<Option<UnlockedSession>>>,
    clipboard: SecureClipboard,
    biometrics: BiometricService,
    argon_params: Argon2Params,
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
        }
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
        let search_index = VaultSearchIndex::new()?;
        let mut session_guard = self.session.write().await;
        *session_guard = Some(UnlockedSession {
            master_key,
            search_index,
        });

        Ok(())
    }

    /// Unlock the vault using the master password
    pub async fn unlock(&self, master_password: &str) -> Result<(), VaultError> {
        let meta = self
            .storage
            .get_kdf_metadata()?
            .ok_or(VaultError::NotInitialized)?;

        let master_key = meta.unlock(master_password)?;
        let search_index = VaultSearchIndex::new()?;

        // Index all existing secret metadata into the ephemeral Tantivy index
        let existing_metadata = self.storage.list_all_metadata()?;
        search_index.index_batch(&existing_metadata)?;

        let mut session_guard = self.session.write().await;
        *session_guard = Some(UnlockedSession {
            master_key,
            search_index,
        });

        Ok(())
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

    /// Check if biometric hardware authentication is available
    pub fn is_biometric_available(&self) -> bool {
        self.biometrics.is_available()
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

        self.storage.insert_secret(&record)?;
        session.search_index.index_secret(&metadata)?;

        // Audit log
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: secret_id.clone(),
            action: SecretAuditAction::Created,
            timestamp: now,
            client: client.to_string(),
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
            .get_secret(id)?
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;

        let payload = decrypt_secret_payload(&session.master_key, &record.encrypted_payload)?;

        // Audit log
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: id.to_string(),
            action: SecretAuditAction::Read,
            timestamp: now_timestamp(),
            client: client.to_string(),
        };
        let _ = self.storage.insert_audit_log(&audit);

        Ok(DecryptedSecret {
            metadata: record.metadata,
            payload,
        })
    }

    /// Retrieve secret metadata without decrypting the payload
    pub async fn get_secret_metadata(&self, id: &str) -> Result<SecretMetadata, VaultError> {
        let record = self
            .storage
            .get_secret(id)?
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;
        Ok(record.metadata)
    }

    /// List all secret metadata records
    pub async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, VaultError> {
        if self.is_locked().await {
            return Err(VaultError::Locked);
        }
        self.storage.list_all_metadata()
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
            .get_secret(id)?
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

        self.storage.update_secret(&updated_record)?;
        session.search_index.index_secret(&updated_metadata)?;

        // Audit log
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: id.to_string(),
            action: SecretAuditAction::Updated,
            timestamp: now,
            client: client.to_string(),
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
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: id.to_string(),
            action: SecretAuditAction::Deleted,
            timestamp: now_timestamp(),
            client: client.to_string(),
        };
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
        let audit = SecretAuditEntry {
            id: generate_uuid(),
            secret_id: id.to_string(),
            action: SecretAuditAction::CopiedToClipboard,
            timestamp: now_timestamp(),
            client: client.to_string(),
        };
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
    }
}
