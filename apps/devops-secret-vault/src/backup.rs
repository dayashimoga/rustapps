#![forbid(unsafe_code)]
//! DevOps Secret Vault - Authenticated Portable Encrypted Backup Archive Format
//!
//! Implements the tamper-evident `.vaultbackup` binary archive specification:
//! - 16-byte Magic Header: `VAULT_BACKUP_V1\0`
//! - 16-byte Cryptographic Salt for Argon2id key derivation
//! - 12-byte Argon2id Parameter block (memory_cost, time_iterations, parallelism)
//! - 12-byte AES-256-GCM Random Nonce
//! - Authenticated AES-256-GCM Ciphertext + 16-byte integrity tag
//! - Plaintext payload includes SHA-256 checksum verification against corruption

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_encryption::{
    decrypt_aes_gcm, derive_key_argon2id, encrypt_aes_gcm, generate_salt, Argon2Params,
};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::VaultError;
use crate::models::DecryptedSecret;

const BACKUP_MAGIC_HEADER: &[u8; 16] = b"VAULT_BACKUP_V1\0";

/// Plaintext archive envelope before encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEnvelope {
    pub version: u32,
    pub exported_at: i64,
    pub app_version: String,
    pub secrets: Vec<DecryptedSecret>,
    pub sha256_checksum: String,
}

/// Export summary result returned to caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupExportSummary {
    pub output_path: String,
    pub total_secrets: usize,
    pub file_size_bytes: u64,
    pub exported_at: i64,
}

/// Import summary result returned to caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupImportSummary {
    pub total_imported: usize,
    pub total_skipped: usize,
    pub total_overwritten: usize,
    pub original_exported_at: i64,
}

// ---------------------------------------------------------------------------
// Export Engine
// ---------------------------------------------------------------------------

/// Create and export a secure, authenticated `.vaultbackup` archive
pub fn export_vault_backup(
    secrets: &[DecryptedSecret],
    passphrase: &str,
    output_path: &Path,
    argon_params: Option<Argon2Params>,
) -> Result<BackupExportSummary, VaultError> {
    if passphrase.trim().is_empty() {
        return Err(VaultError::ValidationError(
            "Backup passphrase cannot be empty".to_string(),
        ));
    }

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 1. Serialize secrets and compute SHA-256 checksum
    let secrets_json = serde_json::to_vec(secrets)?;
    let mut hasher = Sha256::new();
    hasher.update(&secrets_json);
    let checksum_hex = format!("{:x}", hasher.finalize());

    let envelope = BackupEnvelope {
        version: 1,
        exported_at: now_epoch,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        secrets: secrets.to_vec(),
        sha256_checksum: checksum_hex,
    };

    let envelope_bytes = serde_json::to_vec(&envelope)?;

    // 2. Derive key using Argon2id
    let salt = generate_salt();
    let params = argon_params.unwrap_or_default();
    let derived_key = derive_key_argon2id(passphrase.as_bytes(), &salt, &params)?;

    // 3. Encrypt envelope with AES-256-GCM (returns [12-byte nonce || ciphertext || 16-byte tag])
    let encrypted_payload = encrypt_aes_gcm(&derived_key, &envelope_bytes)?;

    // 4. Construct binary archive format
    let mut file = File::create(output_path)?;
    // Magic (16 bytes)
    file.write_all(BACKUP_MAGIC_HEADER)?;
    // Salt (16 bytes)
    file.write_all(&salt)?;
    // Params (12 bytes: 3 x u32 big-endian)
    file.write_all(&params.memory_cost_kib.to_be_bytes())?;
    file.write_all(&params.time_cost_iterations.to_be_bytes())?;
    file.write_all(&params.parallelism.to_be_bytes())?;
    // Encrypted payload (12-byte nonce + ciphertext + tag)
    file.write_all(&encrypted_payload)?;
    file.flush()?;

    let metadata = std::fs::metadata(output_path)?;

    Ok(BackupExportSummary {
        output_path: output_path.to_string_lossy().to_string(),
        total_secrets: secrets.len(),
        file_size_bytes: metadata.len(),
        exported_at: now_epoch,
    })
}

// ---------------------------------------------------------------------------
// Import Engine
// ---------------------------------------------------------------------------

/// Decrypt and restore secrets from an authenticated `.vaultbackup` archive
pub fn read_vault_backup(
    input_path: &Path,
    passphrase: &str,
) -> Result<BackupEnvelope, VaultError> {
    if !input_path.exists() {
        return Err(VaultError::ValidationError(format!(
            "Backup file '{}' does not exist",
            input_path.display()
        )));
    }

    let mut file = File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Minimum size: 16 (magic) + 16 (salt) + 12 (params) + 12 (nonce) + 16 (tag) = 72 bytes
    if buffer.len() < 72 {
        return Err(VaultError::ValidationError(
            "Backup file is corrupted or too short".to_string(),
        ));
    }

    let mut cursor = 0;

    // 1. Verify Magic Header
    if &buffer[cursor..cursor + 16] != BACKUP_MAGIC_HEADER {
        return Err(VaultError::ValidationError(
            "Invalid backup file header (not a valid .vaultbackup archive)".to_string(),
        ));
    }
    cursor += 16;

    // 2. Read Salt
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&buffer[cursor..cursor + 16]);
    cursor += 16;

    // 3. Read Argon2 Parameters
    let memory_cost_kib = u32::from_be_bytes(buffer[cursor..cursor + 4].try_into().unwrap());
    cursor += 4;
    let time_cost_iterations = u32::from_be_bytes(buffer[cursor..cursor + 4].try_into().unwrap());
    cursor += 4;
    let parallelism = u32::from_be_bytes(buffer[cursor..cursor + 4].try_into().unwrap());
    cursor += 4;

    let params = Argon2Params {
        memory_cost_kib,
        time_cost_iterations,
        parallelism,
    };

    // 4. Encrypted payload
    let encrypted_payload = &buffer[cursor..];

    // 5. Derive key and Decrypt AES-256-GCM
    let derived_key = derive_key_argon2id(passphrase.as_bytes(), &salt, &params)?;
    let decrypted = decrypt_aes_gcm(&derived_key, encrypted_payload)
        .map_err(|_| VaultError::InvalidMasterPassword)?;

    // 6. Parse JSON envelope
    let envelope: BackupEnvelope = serde_json::from_slice(decrypted.as_slice())?;

    // 7. Verify Integrity Checksum
    let secrets_json = serde_json::to_vec(&envelope.secrets)?;
    let mut hasher = Sha256::new();
    hasher.update(&secrets_json);
    let calculated_checksum = format!("{:x}", hasher.finalize());

    if calculated_checksum != envelope.sha256_checksum {
        return Err(VaultError::ValidationError(
            "Backup archive checksum mismatch: file is corrupted or tampered".to_string(),
        ));
    }

    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SecretMetadata, SecretPayload, SecretType};
    use tempfile::NamedTempFile;

    #[test]
    fn test_backup_roundtrip() {
        let temp_file = NamedTempFile::new().unwrap();
        let backup_path = temp_file.path();

        let secret = DecryptedSecret {
            metadata: SecretMetadata {
                id: "test-sec-1".to_string(),
                title: "Production SSH Bastion".to_string(),
                secret_type: SecretType::SshKey,
                tags: vec!["prod".to_string(), "ssh".to_string()],
                username: Some("ubuntu".to_string()),
                url: Some("bastion.corp.local".to_string()),
                notes: Some("Requires YubiKey".to_string()),
                version: 1,
                created_at: 1700000000,
                updated_at: 1700000000,
            },
            payload: SecretPayload::SshKey {
                private_key:
                    "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----"
                        .to_string(),
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...".to_string()),
                passphrase: None,
            },
        };

        let secrets = vec![secret];
        let passphrase = "SuperSecureBackupPassphrase2026!";
        let test_params = Argon2Params::fast_for_tests();

        let summary = export_vault_backup(&secrets, passphrase, backup_path, Some(test_params))
            .expect("Backup export should succeed");

        assert_eq!(summary.total_secrets, 1);
        assert!(summary.file_size_bytes > 72);

        // Valid restore
        let envelope = read_vault_backup(backup_path, passphrase)
            .expect("Backup restore should succeed with correct passphrase");
        assert_eq!(envelope.secrets.len(), 1);
        assert_eq!(envelope.secrets[0].metadata.title, "Production SSH Bastion");

        // Wrong passphrase fails
        assert!(read_vault_backup(backup_path, "WrongPassword").is_err());
    }
}
