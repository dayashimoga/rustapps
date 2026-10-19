//! Shared Storage Subsystem
//!
//! Provides SQLite connection management with WAL mode, automated migrations,
//! safe atomic filesystem I/O, and zero-copy memory-mapped file readers.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

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
}
