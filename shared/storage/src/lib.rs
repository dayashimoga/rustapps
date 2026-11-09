//! Shared Storage Subsystem
//!
//! Provides SQLite connection management with WAL mode, automated migrations,
//! safe atomic filesystem I/O, and zero-copy memory-mapped file readers.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use zeroize::Zeroizing;

/// Storage error definitions
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Migration failed at version {version}: {reason}")]
    MigrationFailed { version: u32, reason: String },
    #[error("File exceeds maximum allowed read size of {limit_bytes} bytes")]
    FileTooLarge { limit_bytes: u64 },
    #[error("Memory mapping error: {0}")]
    MmapError(String),
    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),
    #[error("Audit log hash chain tampered at entry {index}: expected {expected}, got {actual}")]
    AuditChainTampered {
        index: usize,
        expected: String,
        actual: String,
    },
    #[error("Audit log previous hash mismatch at entry {index}")]
    AuditChainLinkBroken { index: usize },
    #[error("Database backup or restoration error: {0}")]
    BackupError(String),
}

/// Represents a schema migration
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub sql: &'static str,
}

/// Thread-safe SQLite database manager
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<rusqlite::Connection>>,
    db_path: Option<PathBuf>,
}

impl Database {
    /// Open or create an on-disk SQLite database with hardened WAL pragmas.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = rusqlite::Connection::open(path_ref)?;
        Self::apply_pragmas(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: Some(path_ref.to_path_buf()),
        })
    }

    /// Open an in-memory SQLite database (primarily for tests and ephemeral workspaces).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open_in_memory()?;
        Self::apply_pragmas(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: None,
        })
    }

    /// Apply high-performance, crash-resilient SQLite pragmas.
    fn apply_pragmas(conn: &rusqlite::Connection) -> Result<(), StorageError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA foreign_keys = ON;
             PRAGMA cache_size = -64000;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(())
    }

    /// Execute a closure with exclusive access to the underlying connection.
    pub fn with_connection<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&mut rusqlite::Connection) -> Result<T, rusqlite::Error>,
    {
        let mut conn = self.conn.lock().map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
                "Mutex poisoned: {}",
                e
            ))))
        })?;
        f(&mut conn).map_err(StorageError::Sqlite)
    }

    /// Get the database file path, if file-backed.
    pub fn path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    /// Create an authenticated AES-256-GCM encrypted snapshot (.bak) of the SQLite database.
    ///
    /// Executes a WAL checkpoint (TRUNCATE) to guarantee all transactions are safely synced,
    /// encrypts the raw database file into an `EncryptedEnvelope`, atomically writes the backup
    /// file, and rotates older snapshots exceeding `max_snapshots`.
    pub fn create_encrypted_backup<P: AsRef<Path>>(
        &self,
        backup_dir: P,
        key: &[u8; 32],
        max_snapshots: usize,
    ) -> Result<PathBuf, StorageError> {
        let db_path = self.db_path.as_ref().ok_or_else(|| {
            StorageError::BackupError("Cannot create backup of an in-memory database".to_string())
        })?;

        // 1. Force WAL checkpoint to flush all uncommitted/WAL pages into main SQLite file
        self.with_connection(|conn| {
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            Ok(())
        })?;

        // 2. Read SQLite database file bytes
        let db_bytes = std::fs::read(db_path)?;

        // 3. Encrypt via authenticated AES-256-GCM envelope
        let envelope = EncryptedEnvelope::encrypt(&db_bytes, key)?;
        let serialized = serde_json::to_vec_pretty(&envelope)
            .map_err(|e| StorageError::BackupError(format!("Failed to serialize backup: {}", e)))?;

        // 4. Ensure backup directory exists
        let backup_dir_ref = backup_dir.as_ref();
        std::fs::create_dir_all(backup_dir_ref)?;

        // 5. Generate timestamped backup file name: <db_filename>.<timestamp>.bak
        let db_file_name = db_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("database.db");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let backup_file_name = format!("{}.{}.bak", db_file_name, timestamp);
        let backup_path = backup_dir_ref.join(&backup_file_name);

        // 6. Write backup file atomically
        atomic_write_file(&backup_path, &serialized)?;

        // 7. Rotate older snapshots if exceeding max_snapshots
        if max_snapshots > 0 {
            Self::rotate_backups(backup_dir_ref, db_file_name, max_snapshots)?;
        }

        Ok(backup_path)
    }

    /// Rotate older backup snapshots matching `<db_file_name>.*.bak` so at most `max_snapshots` remain.
    fn rotate_backups(
        backup_dir: &Path,
        db_file_name: &str,
        max_snapshots: usize,
    ) -> Result<(), StorageError> {
        let prefix = format!("{}.", db_file_name);
        let mut backups = Vec::new();

        if let Ok(entries) = std::fs::read_dir(backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        if file_name.starts_with(&prefix) && file_name.ends_with(".bak") {
                            let mtime = entry
                                .metadata()
                                .and_then(|m| m.modified())
                                .unwrap_or(std::time::UNIX_EPOCH);
                            backups.push((path, mtime));
                        }
                    }
                }
            }
        }

        // Sort by modification time ascending (oldest first)
        backups.sort_by_key(|(_, mtime)| *mtime);

        if backups.len() > max_snapshots {
            let to_remove = backups.len() - max_snapshots;
            for (path, _) in backups.into_iter().take(to_remove) {
                let _ = std::fs::remove_file(path);
            }
        }

        Ok(())
    }

    /// Restore a SQLite database from an authenticated AES-256-GCM encrypted backup file.
    pub fn restore_encrypted_backup<P: AsRef<Path>, B: AsRef<Path>>(
        target_db_path: P,
        backup_file: B,
        key: &[u8; 32],
    ) -> Result<Self, StorageError> {
        let backup_bytes = std::fs::read(backup_file.as_ref())?;
        let envelope: EncryptedEnvelope = serde_json::from_slice(&backup_bytes)
            .map_err(|e| StorageError::BackupError(format!("Invalid backup format: {}", e)))?;

        let decrypted_bytes = envelope.decrypt(key)?;
        let target_ref = target_db_path.as_ref();

        atomic_write_file(target_ref, &decrypted_bytes)?;
        Self::open(target_ref)
    }
}

/// Migration execution engine
pub struct MigrationRunner;

impl MigrationRunner {
    /// Initialize the migrations table and apply all pending migrations in order.
    pub fn apply_migrations(db: &Database, migrations: &[Migration]) -> Result<(), StorageError> {
        db.with_connection(|conn| {
            let tx = conn.transaction()?;

            tx.execute(
                "CREATE TABLE IF NOT EXISTS _schema_migrations (
                    version INTEGER PRIMARY KEY,
                    description TEXT NOT NULL,
                    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );",
                [],
            )?;

            for migration in migrations {
                let already_applied: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM _schema_migrations WHERE version = ?1)",
                        [migration.version],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                if !already_applied {
                    tx.execute_batch(migration.sql).map_err(|e| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                            format!(
                                "Failed migration {} ({}): {}",
                                migration.version, migration.description, e
                            ),
                        )))
                    })?;

                    tx.execute(
                        "INSERT INTO _schema_migrations (version, description) VALUES (?1, ?2)",
                        rusqlite::params![migration.version, migration.description],
                    )?;
                }
            }

            tx.commit()?;
            Ok(())
        })
    }
}

/// Safe atomic file write: writes to temporary sibling file, flushes to disk, and atomically renames.
pub fn atomic_write_file<P: AsRef<Path>>(path: P, contents: &[u8]) -> Result<(), StorageError> {
    let dest_path = path.as_ref();
    let parent = dest_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let temp_path = parent.join(format!(
        ".{}.tmp.{}",
        dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file"),
        std::process::id()
    ));

    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;

        file.write_all(contents)?;
        file.sync_all()?;
    }

    std::fs::rename(&temp_path, dest_path)?;
    Ok(())
}

/// Safely read a file into memory with a maximum size limit constraint.
pub fn safe_read_file<P: AsRef<Path>>(path: P, limit_bytes: u64) -> Result<Vec<u8>, StorageError> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() > limit_bytes {
        return Err(StorageError::FileTooLarge { limit_bytes });
    }

    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    let mut reader = file.take(limit_bytes);
    reader.read_to_end(&mut buffer)?;

    Ok(buffer)
}

/// Zero-copy memory-mapped file reader for high-throughput log and index access.
///
/// Safety justification: Per ADR-0004 and ADR-0008, memory mapping is strictly
/// read-only (`memmap2::Mmap`) across immutable or append-only log files.
pub struct MmapReader {
    mmap: memmap2::Mmap,
    len: usize,
}

impl MmapReader {
    /// Open a file and map its entire contents into virtual memory.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let file = File::open(path.as_ref())?;
        let len = file.metadata()?.len() as usize;

        // SAFETY: ADR-0004 / ADR-0008 approved. Read-only mapping.
        // We open the file descriptor read-only and guarantee bounds-checked slicing via as_bytes().
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| StorageError::MmapError(format!("Mmap mapping failed: {}", e)))?
        };

        Ok(Self { mmap, len })
    }

    /// Access the mapped file as a raw byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap[..self.len]
    }

    /// Retrieve a sub-slice of bytes with strict bounds checking.
    pub fn slice(&self, start: usize, end: usize) -> Option<&[u8]> {
        if start <= end && end <= self.len {
            Some(&self.mmap[start..end])
        } else {
            None
        }
    }

    /// Length of mapped file in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether mapped file is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Authenticated encrypted envelope wrapping arbitrary record payload with AES-256-GCM
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub salt: [u8; 16],
}

impl EncryptedEnvelope {
    /// Encrypt plaintext using AES-256-GCM with a fresh random 96-bit nonce and 128-bit salt
    pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<Self, StorageError> {
        let mut nonce = [0u8; 12];
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut nonce);
        OsRng.fill_bytes(&mut salt);

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| StorageError::CryptoError(format!("Cipher init failed: {}", e)))?;

        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|e| StorageError::CryptoError(format!("Encryption failed: {}", e)))?;

        Ok(Self {
            ciphertext,
            nonce,
            salt,
        })
    }

    /// Decrypt ciphertext using AES-256-GCM
    pub fn decrypt(&self, key: &[u8; 32]) -> Result<Zeroizing<Vec<u8>>, StorageError> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| StorageError::CryptoError(format!("Cipher init failed: {}", e)))?;

        let decrypted = cipher
            .decrypt(Nonce::from_slice(&self.nonce), self.ciphertext.as_slice())
            .map_err(|e| {
                StorageError::CryptoError(format!("Decryption authentication failed: {}", e))
            })?;

        Ok(Zeroizing::new(decrypted))
    }
}

/// Tamper-evident audit chain record entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChainEntry {
    pub entry_id: u64,
    pub timestamp_secs: u64,
    pub actor: String,
    pub action: String,
    pub subject: String,
    pub prev_hash: [u8; 32],
    pub entry_hash: [u8; 32],
}

/// Computes deterministic SHA-256 hash for an audit entry
pub fn compute_audit_entry_hash(
    prev_hash: &[u8; 32],
    entry_id: u64,
    timestamp_secs: u64,
    actor: &str,
    action: &str,
    subject: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"AUDIT_CHAIN_V1:");
    hasher.update(prev_hash);
    hasher.update(entry_id.to_be_bytes());
    hasher.update(timestamp_secs.to_be_bytes());
    hasher.update((actor.len() as u32).to_be_bytes());
    hasher.update(actor.as_bytes());
    hasher.update((action.len() as u32).to_be_bytes());
    hasher.update(action.as_bytes());
    hasher.update((subject.len() as u32).to_be_bytes());
    hasher.update(subject.as_bytes());
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

/// Helper converting byte slice to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// In-memory and SQLite-backed tamper-evident audit log manager
#[derive(Debug, Default, Clone)]
pub struct TamperEvidentAuditLog {
    entries: Vec<AuditChainEntry>,
}

impl TamperEvidentAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn from_entries(entries: Vec<AuditChainEntry>) -> Self {
        Self { entries }
    }

    pub fn last_hash(&self) -> [u8; 32] {
        self.entries
            .last()
            .map(|e| e.entry_hash)
            .unwrap_or([0u8; 32])
    }

    pub fn append(
        &mut self,
        actor: &str,
        action: &str,
        subject: &str,
        timestamp_secs: u64,
    ) -> &AuditChainEntry {
        let entry_id = self.entries.len() as u64 + 1;
        let prev_hash = self.last_hash();
        let entry_hash =
            compute_audit_entry_hash(&prev_hash, entry_id, timestamp_secs, actor, action, subject);

        let entry = AuditChainEntry {
            entry_id,
            timestamp_secs,
            actor: actor.to_string(),
            action: action.to_string(),
            subject: subject.to_string(),
            prev_hash,
            entry_hash,
        };

        self.entries.push(entry);
        self.entries.last().expect("Entry was just added")
    }

    pub fn entries(&self) -> &[AuditChainEntry] {
        &self.entries
    }

    /// Verifies that all entries in the chain have valid cryptographic linkage
    pub fn verify_chain(entries: &[AuditChainEntry]) -> Result<(), StorageError> {
        let mut expected_prev_hash = [0u8; 32];

        for (idx, entry) in entries.iter().enumerate() {
            if entry.prev_hash != expected_prev_hash {
                return Err(StorageError::AuditChainLinkBroken { index: idx });
            }

            let computed = compute_audit_entry_hash(
                &entry.prev_hash,
                entry.entry_id,
                entry.timestamp_secs,
                &entry.actor,
                &entry.action,
                &entry.subject,
            );

            if computed != entry.entry_hash {
                return Err(StorageError::AuditChainTampered {
                    index: idx,
                    expected: bytes_to_hex(&computed),
                    actual: bytes_to_hex(&entry.entry_hash),
                });
            }

            expected_prev_hash = entry.entry_hash;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_database_wal_pragmas() {
        let db = Database::open_in_memory().expect("Should open in-memory db");
        db.with_connection(|conn| {
            let mode: String = conn.query_row("PRAGMA journal_mode;", [], |row| row.get(0))?;
            // in-memory sqlite may return "memory", which is expected for memory dbs
            assert!(mode == "memory" || mode == "wal");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_migrations() {
        let db = Database::open_in_memory().unwrap();
        let migrations = vec![
            Migration {
                version: 1,
                description: "create secrets table",
                sql: "CREATE TABLE secrets (id TEXT PRIMARY KEY, title TEXT NOT NULL);",
            },
            Migration {
                version: 2,
                description: "add payload column",
                sql: "ALTER TABLE secrets ADD COLUMN payload BLOB;",
            },
        ];

        MigrationRunner::apply_migrations(&db, &migrations).unwrap();

        // Re-applying same migrations should be idempotent
        MigrationRunner::apply_migrations(&db, &migrations).unwrap();

        db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO secrets (id, title, payload) VALUES (?1, ?2, ?3)",
                rusqlite::params!["sec_1", "AWS Root", b"encrypted_bytes"],
            )?;
            let title: String =
                conn.query_row("SELECT title FROM secrets WHERE id = 'sec_1'", [], |row| {
                    row.get(0)
                })?;
            assert_eq!(title, "AWS Root");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_atomic_write_and_safe_read() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("secure_config.json");
        let content = b"{\"database\": \"production.db\"}";

        atomic_write_file(&file_path, content).unwrap();
        assert!(file_path.exists());

        let read_back = safe_read_file(&file_path, 1024).unwrap();
        assert_eq!(read_back, content);

        // Test boundary size rejection
        let err = safe_read_file(&file_path, 5).unwrap_err();
        match err {
            StorageError::FileTooLarge { limit_bytes } => assert_eq!(limit_bytes, 5),
            other => panic!("Expected FileTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn test_mmap_reader() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("large_log.txt");
        let sample_text = b"2026-10-19 INFO System ready\n2026-10-19 WARN Disk pressure\n";
        std::fs::write(&file_path, sample_text).unwrap();

        let reader = MmapReader::open(&file_path).unwrap();
        assert_eq!(reader.len(), sample_text.len());
        assert_eq!(reader.as_bytes(), sample_text);

        let slice = reader.slice(0, 10).unwrap();
        assert_eq!(slice, &sample_text[0..10]);

        assert!(reader.slice(0, 9999).is_none());
    }

    #[test]
    fn test_encrypted_envelope() {
        let key = [42u8; 32];
        let plaintext = b"Confidential Vault Secret Payload 2026";

        let envelope = EncryptedEnvelope::encrypt(plaintext, &key).unwrap();
        assert_ne!(envelope.ciphertext, plaintext);

        let decrypted = envelope.decrypt(&key).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);

        // Wrong key must fail authenticated decryption
        let wrong_key = [99u8; 32];
        assert!(envelope.decrypt(&wrong_key).is_err());
    }

    #[test]
    fn test_tamper_evident_audit_chain() {
        let mut log = TamperEvidentAuditLog::new();
        log.append("admin", "CREATE", "vault:db-creds", 1700000000);
        log.append("dev", "READ", "vault:db-creds", 1700000100);
        log.append("admin", "UPDATE", "vault:db-creds", 1700000200);

        assert_eq!(log.entries().len(), 3);
        assert!(TamperEvidentAuditLog::verify_chain(log.entries()).is_ok());

        // Tampering test 1: Modify action
        let mut tampered_entries = log.entries().to_vec();
        tampered_entries[1].action = "DELETE".to_string();
        let err = TamperEvidentAuditLog::verify_chain(&tampered_entries).unwrap_err();
        assert!(matches!(
            err,
            StorageError::AuditChainTampered { index: 1, .. }
        ));

        // Tampering test 2: Break link
        let mut broken_link = log.entries().to_vec();
        broken_link[2].prev_hash = [0u8; 32];
        let err2 = TamperEvidentAuditLog::verify_chain(&broken_link).unwrap_err();
        assert!(matches!(
            err2,
            StorageError::AuditChainLinkBroken { index: 2 }
        ));
    }

    #[test]
    fn test_encrypted_database_backup_and_restore() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("production.db");
        let backup_dir = temp.path().join("backups");
        let restore_path = temp.path().join("restored.db");
        let key = [0x55u8; 32];

        // 1. Initialize DB and insert records
        let db = Database::open(&db_path).unwrap();
        db.with_connection(|conn| {
            conn.execute_batch(
                "CREATE TABLE secrets (id INTEGER PRIMARY KEY, key TEXT, value TEXT);
                 INSERT INTO secrets (key, value) VALUES ('api_token', 'sec_987654321');
                 INSERT INTO secrets (key, value) VALUES ('ssh_key', 'ed25519-secret');"
            )?;
            Ok(())
        }).unwrap();

        // 2. Create encrypted backup
        let backup_file = db.create_encrypted_backup(&backup_dir, &key, 3).unwrap();
        assert!(backup_file.exists());
        assert!(backup_file.to_string_lossy().ends_with(".bak"));

        // Verify the raw backup file is NOT plaintext SQLite
        let raw_backup = std::fs::read(&backup_file).unwrap();
        assert!(!raw_backup.starts_with(b"SQLite format 3"));

        // 3. Restore backup with correct key
        let restored_db = Database::restore_encrypted_backup(&restore_path, &backup_file, &key).unwrap();
        assert!(restore_path.exists());

        // Verify restored records
        let count: i64 = restored_db.with_connection(|conn| {
            conn.query_row("SELECT count(*) FROM secrets", [], |row| row.get(0))
        }).unwrap();
        assert_eq!(count, 2);

        let token: String = restored_db.with_connection(|conn| {
            conn.query_row("SELECT value FROM secrets WHERE key = 'api_token'", [], |row| row.get(0))
        }).unwrap();
        assert_eq!(token, "sec_987654321");

        // 4. Restore with wrong key must fail authenticated decryption
        let wrong_key = [0x99u8; 32];
        let bad_restore = temp.path().join("bad_restored.db");
        assert!(Database::restore_encrypted_backup(&bad_restore, &backup_file, &wrong_key).is_err());
    }

    #[test]
    fn test_backup_rotation() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("vault.db");
        let backup_dir = temp.path().join("backups");
        let key = [0x77u8; 32];

        let db = Database::open(&db_path).unwrap();
        db.with_connection(|conn| {
            conn.execute_batch("CREATE TABLE t (x INT); INSERT INTO t VALUES (1);")?;
            Ok(())
        }).unwrap();

        // Create 4 snapshots with max_snapshots = 2
        for _ in 0..4 {
            db.create_encrypted_backup(&backup_dir, &key, 2).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let entries: Vec<_> = std::fs::read_dir(&backup_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().to_string_lossy().ends_with(".bak"))
            .collect();

        assert_eq!(entries.len(), 2, "Backups must be rotated to exactly max_snapshots");
    }
}
