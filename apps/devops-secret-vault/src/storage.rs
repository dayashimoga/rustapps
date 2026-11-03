#![forbid(unsafe_code)]
//! DevOps Secret Vault - SQLite Encrypted Storage Engine with WAL Mode

use shared_storage::{Database, Migration, MigrationRunner};
use std::path::Path;

use crate::crypto::{
    decrypt_metadata_fields, encrypt_metadata_fields, VaultKeyDerivationMetadata, VaultMasterKey,
};
use crate::error::VaultError;
use crate::models::{
    EncryptedMetadataFields, EncryptedSecretRecord, SecretAuditAction, SecretAuditEntry,
    SecretMetadata, SecretType,
};

const METADATA_KEY_KDF: &str = "vault_kdf_metadata";
const AUDIT_GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Vault storage layer wrapping thread-safe SQLite WAL database
#[derive(Clone)]
pub struct VaultStorage {
    db: Database,
}

impl VaultStorage {
    /// Open or create an on-disk SQLite vault database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, VaultError> {
        let db = Database::open(path)?;
        Self::initialize_schema(&db)?;
        Ok(Self { db })
    }

    /// Open an ephemeral in-memory vault database for tests
    pub fn open_in_memory() -> Result<Self, VaultError> {
        let db = Database::open_in_memory()?;
        Self::initialize_schema(&db)?;
        Ok(Self { db })
    }

    fn initialize_schema(db: &Database) -> Result<(), VaultError> {
        let migrations = [
            Migration {
                version: 1,
                description: "Initial vault schema with metadata, secrets, and audit tables",
                sql: "
                    CREATE TABLE IF NOT EXISTS vault_metadata (
                        key TEXT PRIMARY KEY,
                        value BLOB NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS secrets (
                        id TEXT PRIMARY KEY,
                        title TEXT NOT NULL,
                        secret_type TEXT NOT NULL,
                        tags TEXT NOT NULL,
                        username TEXT,
                        url TEXT,
                        notes TEXT,
                        encrypted_payload BLOB NOT NULL,
                        version INTEGER NOT NULL DEFAULT 1,
                        created_at INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL
                    );

                    CREATE INDEX IF NOT EXISTS idx_secrets_title ON secrets(title);
                    CREATE INDEX IF NOT EXISTS idx_secrets_type ON secrets(secret_type);

                    CREATE TABLE IF NOT EXISTS audit_logs (
                        id TEXT PRIMARY KEY,
                        secret_id TEXT NOT NULL,
                        action TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        client TEXT NOT NULL
                    );

                    CREATE INDEX IF NOT EXISTS idx_audit_secret_id ON audit_logs(secret_id);
                    CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
                ",
            },
            Migration {
                version: 2,
                description: "Add encrypted_metadata column and cryptographic hash chain to audit_logs",
                sql: "
                    ALTER TABLE secrets ADD COLUMN encrypted_metadata BLOB;
                    ALTER TABLE audit_logs ADD COLUMN prev_hash TEXT NOT NULL DEFAULT '0000000000000000000000000000000000000000000000000000000000000000';
                    ALTER TABLE audit_logs ADD COLUMN entry_hash TEXT NOT NULL DEFAULT '';
                ",
            },
        ];

        MigrationRunner::apply_migrations(db, &migrations)?;
        Ok(())
    }

    /// Retrieve key derivation metadata if initialized
    pub fn get_kdf_metadata(&self) -> Result<Option<VaultKeyDerivationMetadata>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare("SELECT value FROM vault_metadata WHERE key = ?1")?;
                let mut rows = stmt.query([METADATA_KEY_KDF])?;

                if let Some(row) = rows.next()? {
                    let blob: Vec<u8> = row.get(0)?;
                    let meta: VaultKeyDerivationMetadata = serde_json::from_slice(&blob)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    Ok(Some(meta))
                } else {
                    Ok(None)
                }
            })
            .map_err(VaultError::Storage)
    }

    /// Save key derivation metadata on initialization
    pub fn set_kdf_metadata(&self, meta: &VaultKeyDerivationMetadata) -> Result<(), VaultError> {
        let blob = serde_json::to_vec(meta)?;
        self.db
            .with_connection(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO vault_metadata (key, value) VALUES (?1, ?2)",
                    rusqlite::params![METADATA_KEY_KDF, blob],
                )?;
                Ok(())
            })
            .map_err(VaultError::Storage)
    }

    /// Insert a newly created encrypted secret record with AES-256-GCM metadata encryption
    pub fn insert_secret_encrypted(
        &self,
        record: &EncryptedSecretRecord,
        master_key: &VaultMasterKey,
    ) -> Result<(), VaultError> {
        let meta_fields = EncryptedMetadataFields {
            tags: record.metadata.tags.clone(),
            username: record.metadata.username.clone(),
            url: record.metadata.url.clone(),
            notes: record.metadata.notes.clone(),
        };
        let encrypted_meta = encrypt_metadata_fields(master_key, &meta_fields)?;
        let secret_type_str = record.metadata.secret_type.as_str();

        self.db
            .with_connection(|conn| {
                conn.execute(
                    "INSERT INTO secrets (
                    id, title, secret_type, tags, username, url, notes,
                    encrypted_payload, version, created_at, updated_at, encrypted_metadata
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    rusqlite::params![
                        record.metadata.id,
                        record.metadata.title,
                        secret_type_str,
                        "",
                        None::<String>,
                        None::<String>,
                        None::<String>,
                        record.encrypted_payload,
                        record.metadata.version,
                        record.metadata.created_at,
                        record.metadata.updated_at,
                        Some(encrypted_meta),
                    ],
                )?;
                Ok(())
            })
            .map_err(VaultError::Storage)
    }

    /// Insert a secret record with fallback unencrypted metadata (for tests/legacy)
    pub fn insert_secret(&self, record: &EncryptedSecretRecord) -> Result<(), VaultError> {
        let tags_json = serde_json::to_string(&record.metadata.tags)?;
        let secret_type_str = record.metadata.secret_type.as_str();

        self.db
            .with_connection(|conn| {
                conn.execute(
                    "INSERT INTO secrets (
                    id, title, secret_type, tags, username, url, notes,
                    encrypted_payload, version, created_at, updated_at, encrypted_metadata
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
                    rusqlite::params![
                        record.metadata.id,
                        record.metadata.title,
                        secret_type_str,
                        tags_json,
                        record.metadata.username,
                        record.metadata.url,
                        record.metadata.notes,
                        record.encrypted_payload,
                        record.metadata.version,
                        record.metadata.created_at,
                        record.metadata.updated_at,
                    ],
                )?;
                Ok(())
            })
            .map_err(VaultError::Storage)
    }

    /// Get encrypted secret by ID, decrypting metadata with master key
    pub fn get_secret_decrypted(
        &self,
        id: &str,
        master_key: &VaultMasterKey,
    ) -> Result<Option<EncryptedSecretRecord>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, secret_type, tags, username, url, notes,
                        encrypted_payload, version, created_at, updated_at, encrypted_metadata
                 FROM secrets WHERE id = ?1",
                )?;

                let mut rows = stmt.query([id])?;
                if let Some(row) = rows.next()? {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let secret_type_str: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let username: Option<String> = row.get(4)?;
                    let url: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    let encrypted_payload: Vec<u8> = row.get(7)?;
                    let version: u32 = row.get(8)?;
                    let created_at: i64 = row.get(9)?;
                    let updated_at: i64 = row.get(10)?;
                    let encrypted_meta: Option<Vec<u8>> = row.get(11).unwrap_or(None);

                    let (tags, username, url, notes) = if let Some(ref cipher) = encrypted_meta {
                        if !cipher.is_empty() {
                            match decrypt_metadata_fields(master_key, cipher) {
                                Ok(fields) => {
                                    (fields.tags, fields.username, fields.url, fields.notes)
                                }
                                Err(_) => (
                                    serde_json::from_str(&tags_json).unwrap_or_default(),
                                    username,
                                    url,
                                    notes,
                                ),
                            }
                        } else {
                            (
                                serde_json::from_str(&tags_json).unwrap_or_default(),
                                username,
                                url,
                                notes,
                            )
                        }
                    } else {
                        (
                            serde_json::from_str(&tags_json).unwrap_or_default(),
                            username,
                            url,
                            notes,
                        )
                    };

                    let secret_type = SecretType::from_str_lossy(&secret_type_str);

                    Ok(Some(EncryptedSecretRecord {
                        metadata: SecretMetadata {
                            id,
                            title,
                            secret_type,
                            tags,
                            username,
                            url,
                            notes,
                            version,
                            created_at,
                            updated_at,
                        },
                        encrypted_payload,
                    }))
                } else {
                    Ok(None)
                }
            })
            .map_err(VaultError::Storage)
    }

    /// Get encrypted secret by ID (fallback without decryption)
    pub fn get_secret(&self, id: &str) -> Result<Option<EncryptedSecretRecord>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, secret_type, tags, username, url, notes,
                        encrypted_payload, version, created_at, updated_at
                 FROM secrets WHERE id = ?1",
                )?;

                let mut rows = stmt.query([id])?;
                if let Some(row) = rows.next()? {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let secret_type_str: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let username: Option<String> = row.get(4)?;
                    let url: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    let encrypted_payload: Vec<u8> = row.get(7)?;
                    let version: u32 = row.get(8)?;
                    let created_at: i64 = row.get(9)?;
                    let updated_at: i64 = row.get(10)?;

                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                    let secret_type = SecretType::from_str_lossy(&secret_type_str);

                    Ok(Some(EncryptedSecretRecord {
                        metadata: SecretMetadata {
                            id,
                            title,
                            secret_type,
                            tags,
                            username,
                            url,
                            notes,
                            version,
                            created_at,
                            updated_at,
                        },
                        encrypted_payload,
                    }))
                } else {
                    Ok(None)
                }
            })
            .map_err(VaultError::Storage)
    }

    /// List all non-sensitive secret metadata records, decrypting metadata with master key
    pub fn list_all_metadata_decrypted(
        &self,
        master_key: &VaultMasterKey,
    ) -> Result<Vec<SecretMetadata>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, secret_type, tags, username, url, notes,
                        version, created_at, updated_at, encrypted_metadata
                 FROM secrets ORDER BY updated_at DESC",
                )?;

                let rows = stmt.query_map([], |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let secret_type_str: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let username: Option<String> = row.get(4)?;
                    let url: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    let version: u32 = row.get(7)?;
                    let created_at: i64 = row.get(8)?;
                    let updated_at: i64 = row.get(9)?;
                    let encrypted_meta: Option<Vec<u8>> = row.get(10).unwrap_or(None);

                    let (tags, username, url, notes) = if let Some(ref cipher) = encrypted_meta {
                        if !cipher.is_empty() {
                            match decrypt_metadata_fields(master_key, cipher) {
                                Ok(fields) => {
                                    (fields.tags, fields.username, fields.url, fields.notes)
                                }
                                Err(_) => (
                                    serde_json::from_str(&tags_json).unwrap_or_default(),
                                    username,
                                    url,
                                    notes,
                                ),
                            }
                        } else {
                            (
                                serde_json::from_str(&tags_json).unwrap_or_default(),
                                username,
                                url,
                                notes,
                            )
                        }
                    } else {
                        (
                            serde_json::from_str(&tags_json).unwrap_or_default(),
                            username,
                            url,
                            notes,
                        )
                    };

                    let secret_type = SecretType::from_str_lossy(&secret_type_str);

                    Ok(SecretMetadata {
                        id,
                        title,
                        secret_type,
                        tags,
                        username,
                        url,
                        notes,
                        version,
                        created_at,
                        updated_at,
                    })
                })?;

                let mut results = Vec::new();
                for r in rows {
                    results.push(r?);
                }
                Ok(results)
            })
            .map_err(VaultError::Storage)
    }

    /// List all non-sensitive secret metadata records (cleartext fallback)
    pub fn list_all_metadata(&self) -> Result<Vec<SecretMetadata>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, secret_type, tags, username, url, notes,
                        version, created_at, updated_at
                 FROM secrets ORDER BY updated_at DESC",
                )?;

                let rows = stmt.query_map([], |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let secret_type_str: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let username: Option<String> = row.get(4)?;
                    let url: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    let version: u32 = row.get(7)?;
                    let created_at: i64 = row.get(8)?;
                    let updated_at: i64 = row.get(9)?;

                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                    let secret_type = SecretType::from_str_lossy(&secret_type_str);

                    Ok(SecretMetadata {
                        id,
                        title,
                        secret_type,
                        tags,
                        username,
                        url,
                        notes,
                        version,
                        created_at,
                        updated_at,
                    })
                })?;

                let mut results = Vec::new();
                for r in rows {
                    results.push(r?);
                }
                Ok(results)
            })
            .map_err(VaultError::Storage)
    }

    /// List all encrypted secret records with decrypted metadata and encrypted payloads
    pub fn list_all_encrypted_secrets_decrypted(
        &self,
        master_key: &VaultMasterKey,
    ) -> Result<Vec<EncryptedSecretRecord>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, secret_type, tags, username, url, notes,
                        encrypted_payload, version, created_at, updated_at, encrypted_metadata
                 FROM secrets ORDER BY updated_at DESC",
                )?;

                let rows = stmt.query_map([], |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let secret_type_str: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let username: Option<String> = row.get(4)?;
                    let url: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    let encrypted_payload: Vec<u8> = row.get(7)?;
                    let version: u32 = row.get(8)?;
                    let created_at: i64 = row.get(9)?;
                    let updated_at: i64 = row.get(10)?;
                    let encrypted_meta: Option<Vec<u8>> = row.get(11).unwrap_or(None);

                    let (tags, username, url, notes) = if let Some(ref cipher) = encrypted_meta {
                        if !cipher.is_empty() {
                            match decrypt_metadata_fields(master_key, cipher) {
                                Ok(fields) => {
                                    (fields.tags, fields.username, fields.url, fields.notes)
                                }
                                Err(_) => (
                                    serde_json::from_str(&tags_json).unwrap_or_default(),
                                    username,
                                    url,
                                    notes,
                                ),
                            }
                        } else {
                            (
                                serde_json::from_str(&tags_json).unwrap_or_default(),
                                username,
                                url,
                                notes,
                            )
                        }
                    } else {
                        (
                            serde_json::from_str(&tags_json).unwrap_or_default(),
                            username,
                            url,
                            notes,
                        )
                    };

                    let secret_type = SecretType::from_str_lossy(&secret_type_str);

                    Ok(EncryptedSecretRecord {
                        metadata: SecretMetadata {
                            id,
                            title,
                            secret_type,
                            tags,
                            username,
                            url,
                            notes,
                            version,
                            created_at,
                            updated_at,
                        },
                        encrypted_payload,
                    })
                })?;

                let mut results = Vec::new();
                for r in rows {
                    results.push(r?);
                }
                Ok(results)
            })
            .map_err(VaultError::Storage)
    }

    /// List all encrypted secret records with payloads (cleartext fallback)
    pub fn list_all_encrypted_secrets(&self) -> Result<Vec<EncryptedSecretRecord>, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, secret_type, tags, username, url, notes,
                        encrypted_payload, version, created_at, updated_at
                 FROM secrets ORDER BY updated_at DESC",
                )?;

                let rows = stmt.query_map([], |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let secret_type_str: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let username: Option<String> = row.get(4)?;
                    let url: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    let encrypted_payload: Vec<u8> = row.get(7)?;
                    let version: u32 = row.get(8)?;
                    let created_at: i64 = row.get(9)?;
                    let updated_at: i64 = row.get(10)?;

                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                    let secret_type = SecretType::from_str_lossy(&secret_type_str);

                    Ok(EncryptedSecretRecord {
                        metadata: SecretMetadata {
                            id,
                            title,
                            secret_type,
                            tags,
                            username,
                            url,
                            notes,
                            version,
                            created_at,
                            updated_at,
                        },
                        encrypted_payload,
                    })
                })?;

                let mut results = Vec::new();
                for r in rows {
                    results.push(r?);
                }
                Ok(results)
            })
            .map_err(VaultError::Storage)
    }

    /// Update an existing encrypted secret with AES-256-GCM metadata encryption
    pub fn update_secret_encrypted(
        &self,
        record: &EncryptedSecretRecord,
        master_key: &VaultMasterKey,
    ) -> Result<(), VaultError> {
        let meta_fields = EncryptedMetadataFields {
            tags: record.metadata.tags.clone(),
            username: record.metadata.username.clone(),
            url: record.metadata.url.clone(),
            notes: record.metadata.notes.clone(),
        };
        let encrypted_meta = encrypt_metadata_fields(master_key, &meta_fields)?;
        let secret_type_str = record.metadata.secret_type.as_str();

        self.db
            .with_connection(|conn| {
                let updated = conn.execute(
                    "UPDATE secrets SET
                    title = ?1,
                    secret_type = ?2,
                    tags = ?3,
                    username = ?4,
                    url = ?5,
                    notes = ?6,
                    encrypted_payload = ?7,
                    version = ?8,
                    updated_at = ?9,
                    encrypted_metadata = ?10
                 WHERE id = ?11",
                    rusqlite::params![
                        record.metadata.title,
                        secret_type_str,
                        "",
                        None::<String>,
                        None::<String>,
                        None::<String>,
                        record.encrypted_payload,
                        record.metadata.version,
                        record.metadata.updated_at,
                        Some(encrypted_meta),
                        record.metadata.id,
                    ],
                )?;

                if updated == 0 {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
                Ok(())
            })
            .map_err(|e| match e {
                shared_storage::StorageError::Sqlite(rusqlite::Error::QueryReturnedNoRows) => {
                    VaultError::SecretNotFound(record.metadata.id.clone())
                }
                other => VaultError::Storage(other),
            })
    }

    /// Update an existing encrypted secret (fallback without metadata encryption)
    pub fn update_secret(&self, record: &EncryptedSecretRecord) -> Result<(), VaultError> {
        let tags_json = serde_json::to_string(&record.metadata.tags)?;
        let secret_type_str = record.metadata.secret_type.as_str();

        self.db
            .with_connection(|conn| {
                let updated = conn.execute(
                    "UPDATE secrets SET
                    title = ?1,
                    secret_type = ?2,
                    tags = ?3,
                    username = ?4,
                    url = ?5,
                    notes = ?6,
                    encrypted_payload = ?7,
                    version = ?8,
                    updated_at = ?9
                 WHERE id = ?10",
                    rusqlite::params![
                        record.metadata.title,
                        secret_type_str,
                        tags_json,
                        record.metadata.username,
                        record.metadata.url,
                        record.metadata.notes,
                        record.encrypted_payload,
                        record.metadata.version,
                        record.metadata.updated_at,
                        record.metadata.id,
                    ],
                )?;

                if updated == 0 {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
                Ok(())
            })
            .map_err(|e| match e {
                shared_storage::StorageError::Sqlite(rusqlite::Error::QueryReturnedNoRows) => {
                    VaultError::SecretNotFound(record.metadata.id.clone())
                }
                other => VaultError::Storage(other),
            })
    }

    /// Delete secret by ID
    pub fn delete_secret(&self, id: &str) -> Result<bool, VaultError> {
        self.db
            .with_connection(|conn| {
                let count = conn.execute("DELETE FROM secrets WHERE id = ?1", [id])?;
                Ok(count > 0)
            })
            .map_err(VaultError::Storage)
    }

    /// Record an audit log event with cryptographically chained SHA-256 integrity hash
    pub fn insert_audit_log(&self, entry: &SecretAuditEntry) -> Result<(), VaultError> {
        let action_str = entry.action.as_str();
        self.db
            .with_connection(|conn| {
                let prev_hash: String = conn
                    .query_row(
                        "SELECT entry_hash FROM audit_logs WHERE entry_hash != '' ORDER BY rowid DESC LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or_else(|_| AUDIT_GENESIS_PREV_HASH.to_string());

                let entry_hash = SecretAuditEntry::compute_hash(
                    &prev_hash,
                    &entry.id,
                    &entry.secret_id,
                    action_str,
                    entry.timestamp,
                    &entry.client,
                );

                conn.execute(
                    "INSERT INTO audit_logs (id, secret_id, action, timestamp, client, prev_hash, entry_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        entry.id,
                        entry.secret_id,
                        action_str,
                        entry.timestamp,
                        entry.client,
                        prev_hash,
                        entry_hash,
                    ],
                )?;
                Ok(())
            })
            .map_err(VaultError::Storage)
    }

    /// List audit logs for a secret or across all secrets
    pub fn list_audit_logs(
        &self,
        secret_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SecretAuditEntry>, VaultError> {
        self.db
            .with_connection(|conn| {
                let sql = if secret_id.is_some() {
                    "SELECT id, secret_id, action, timestamp, client, prev_hash, entry_hash
                 FROM audit_logs WHERE secret_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
                } else {
                    "SELECT id, secret_id, action, timestamp, client, prev_hash, entry_hash
                 FROM audit_logs ORDER BY timestamp DESC LIMIT ?1"
                };

                let mut stmt = conn.prepare(sql)?;
                let rows = if let Some(sid) = secret_id {
                    stmt.query_map(rusqlite::params![sid, limit as i64], Self::map_audit_row)?
                } else {
                    stmt.query_map(rusqlite::params![limit as i64], Self::map_audit_row)?
                };

                let mut entries = Vec::new();
                for r in rows {
                    entries.push(r?);
                }
                Ok(entries)
            })
            .map_err(VaultError::Storage)
    }

    /// Verify the complete cryptographic hash chain of the audit log
    pub fn verify_audit_chain(&self) -> Result<bool, VaultError> {
        self.db
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, secret_id, action, timestamp, client, prev_hash, entry_hash
                     FROM audit_logs ORDER BY rowid ASC",
                )?;

                let rows = stmt.query_map([], |row| {
                    let id: String = row.get(0)?;
                    let secret_id: String = row.get(1)?;
                    let action: String = row.get(2)?;
                    let timestamp: i64 = row.get(3)?;
                    let client: String = row.get(4)?;
                    let prev_hash: String = row.get(5)?;
                    let entry_hash: String = row.get(6)?;
                    Ok((
                        id,
                        secret_id,
                        action,
                        timestamp,
                        client,
                        prev_hash,
                        entry_hash,
                    ))
                })?;

                let mut expected_prev_hash = AUDIT_GENESIS_PREV_HASH.to_string();
                for (idx, r) in rows.enumerate() {
                    let (id, secret_id, action, timestamp, client, prev_hash, entry_hash) = r?;
                    if prev_hash != expected_prev_hash {
                        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                            std::io::Error::other(format!(
                                "Audit log chain link broken at index {}: expected prev_hash {}, found {}",
                                idx, expected_prev_hash, prev_hash
                            )),
                        )));
                    }

                    let computed = SecretAuditEntry::compute_hash(
                        &prev_hash,
                        &id,
                        &secret_id,
                        &action,
                        timestamp,
                        &client,
                    );
                    if computed != entry_hash {
                        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                            std::io::Error::other(format!(
                                "Audit log entry tampering detected at entry {}: computed {}, stored {}",
                                id, computed, entry_hash
                            )),
                        )));
                    }

                    expected_prev_hash = entry_hash;
                }

                Ok(true)
            })
            .map_err(VaultError::Storage)
    }

    fn map_audit_row(row: &rusqlite::Row<'_>) -> Result<SecretAuditEntry, rusqlite::Error> {
        let id: String = row.get(0)?;
        let secret_id: String = row.get(1)?;
        let action_str: String = row.get(2)?;
        let timestamp: i64 = row.get(3)?;
        let client: String = row.get(4)?;
        let prev_hash: String = row.get(5).unwrap_or_default();
        let entry_hash: String = row.get(6).unwrap_or_default();

        let action = SecretAuditAction::from_str_lossy(&action_str);
        Ok(SecretAuditEntry {
            id,
            secret_id,
            action,
            timestamp,
            client,
            prev_hash,
            entry_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::generate_uuid;

    #[test]
    fn test_storage_crud_and_metadata() {
        let storage = VaultStorage::open_in_memory().unwrap();
        assert!(storage.get_kdf_metadata().unwrap().is_none());

        // Insert metadata
        let kdf_meta = VaultKeyDerivationMetadata {
            salt: [0x11; 16],
            verifier_ciphertext: vec![1, 2, 3, 4],
            memory_cost_kib: 1024,
            time_cost_iterations: 1,
            parallelism: 1,
        };
        storage.set_kdf_metadata(&kdf_meta).unwrap();

        let retrieved = storage.get_kdf_metadata().unwrap().expect("Should exist");
        assert_eq!(retrieved.salt, [0x11; 16]);

        // Insert Secret
        let secret_id = generate_uuid();
        let record = EncryptedSecretRecord {
            metadata: SecretMetadata {
                id: secret_id.clone(),
                title: "PostgreSQL Production DB".to_string(),
                secret_type: SecretType::DatabaseCredential,
                tags: vec!["prod".to_string(), "database".to_string()],
                username: Some("postgres_admin".to_string()),
                url: Some("db.internal.net".to_string()),
                notes: Some("Read replica available".to_string()),
                version: 1,
                created_at: 1000,
                updated_at: 1000,
            },
            encrypted_payload: vec![0xAA, 0xBB, 0xCC],
        };

        storage.insert_secret(&record).unwrap();

        // Get secret
        let fetched = storage
            .get_secret(&secret_id)
            .unwrap()
            .expect("Secret must exist");
        assert_eq!(fetched.metadata.title, "PostgreSQL Production DB");
        assert_eq!(fetched.encrypted_payload, vec![0xAA, 0xBB, 0xCC]);

        // List metadata
        let all_meta = storage.list_all_metadata().unwrap();
        assert_eq!(all_meta.len(), 1);
        assert_eq!(all_meta[0].id, secret_id);

        // Delete secret
        let deleted = storage.delete_secret(&secret_id).unwrap();
        assert!(deleted);
        assert!(storage.get_secret(&secret_id).unwrap().is_none());
    }

    #[test]
    fn test_metadata_envelope_encryption_at_rest() {
        let storage = VaultStorage::open_in_memory().unwrap();
        let master_key = VaultMasterKey([0x88u8; 32]);
        let secret_id = generate_uuid();

        let record = EncryptedSecretRecord {
            metadata: SecretMetadata {
                id: secret_id.clone(),
                title: "Kubernetes Admin Cluster".to_string(),
                secret_type: SecretType::ApiKey,
                tags: vec!["k8s".to_string(), "infra".to_string()],
                username: Some("k8s_superadmin".to_string()),
                url: Some("https://k8s.prod.internal:6443".to_string()),
                notes: Some("Root administrative certificate".to_string()),
                version: 1,
                created_at: 2000,
                updated_at: 2000,
            },
            encrypted_payload: vec![1, 2, 3, 4, 5],
        };

        // Insert encrypted
        storage
            .insert_secret_encrypted(&record, &master_key)
            .unwrap();

        // Direct database verification: verify zero cleartext sensitive metadata on disk
        storage
            .db
            .with_connection(|conn| {
                let mut stmt = conn
                    .prepare("SELECT username, url, notes, tags FROM secrets WHERE id = ?1")
                    .unwrap();
                let mut rows = stmt.query([&secret_id]).unwrap();
                let row = rows.next().unwrap().unwrap();

                let raw_username: Option<String> = row.get(0).unwrap();
                let raw_url: Option<String> = row.get(1).unwrap();
                let raw_notes: Option<String> = row.get(2).unwrap();
                let raw_tags: String = row.get(3).unwrap();

                assert!(
                    raw_username.is_none(),
                    "Username must be NULL in raw SQLite table"
                );
                assert!(raw_url.is_none(), "URL must be NULL in raw SQLite table");
                assert!(
                    raw_notes.is_none(),
                    "Notes must be NULL in raw SQLite table"
                );
                assert!(
                    raw_tags.is_empty(),
                    "Tags must be empty in raw SQLite table"
                );
                Ok(())
            })
            .unwrap();

        // Decrypted retrieval via master key
        let fetched = storage
            .get_secret_decrypted(&secret_id, &master_key)
            .unwrap()
            .expect("Secret must exist");
        assert_eq!(fetched.metadata.username.as_deref(), Some("k8s_superadmin"));
        assert_eq!(
            fetched.metadata.url.as_deref(),
            Some("https://k8s.prod.internal:6443")
        );
        assert_eq!(
            fetched.metadata.notes.as_deref(),
            Some("Root administrative certificate")
        );
        assert_eq!(fetched.metadata.tags, vec!["k8s", "infra"]);

        // List metadata decrypted
        let list = storage.list_all_metadata_decrypted(&master_key).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].username.as_deref(), Some("k8s_superadmin"));
    }

    #[test]
    fn test_audit_logging_and_tamper_detection() {
        let storage = VaultStorage::open_in_memory().unwrap();

        let e1 = SecretAuditEntry::new(
            generate_uuid(),
            "sec-1".to_string(),
            SecretAuditAction::Created,
            100,
            "desktop".to_string(),
        );
        let e2 = SecretAuditEntry::new(
            generate_uuid(),
            "sec-1".to_string(),
            SecretAuditAction::Read,
            200,
            "desktop".to_string(),
        );
        let e3 = SecretAuditEntry::new(
            generate_uuid(),
            "sec-2".to_string(),
            SecretAuditAction::Created,
            300,
            "cli".to_string(),
        );

        storage.insert_audit_log(&e1).unwrap();
        storage.insert_audit_log(&e2).unwrap();
        storage.insert_audit_log(&e3).unwrap();

        // Check hash chain holds
        assert!(storage.verify_audit_chain().unwrap());

        let logs = storage.list_audit_logs(None, 10).unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[2].prev_hash, AUDIT_GENESIS_PREV_HASH);
        assert_eq!(logs[1].prev_hash, logs[2].entry_hash);
        assert_eq!(logs[0].prev_hash, logs[1].entry_hash);

        // Tamper test: mutate action in row 2 directly in SQLite
        storage
            .db
            .with_connection(|conn| {
                conn.execute(
                    "UPDATE audit_logs SET client = 'hacker_client' WHERE id = ?1",
                    [&e2.id],
                )
                .unwrap();
                Ok(())
            })
            .unwrap();

        // Verification must fail with tampering error
        let verify_result = storage.verify_audit_chain();
        assert!(
            verify_result.is_err(),
            "Tampered audit log entry must fail verification"
        );
    }
}
