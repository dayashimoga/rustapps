#![forbid(unsafe_code)]
//! Persistent Workspaces and Bookmarks Subsystem
//!
//! Stores open-file handles, active regex filters, bookmarked lines, and viewport
//! scroll states through SQLite in WAL mode via `shared-storage`.

use shared_storage::{Database, Migration, MigrationRunner, StorageError};
use std::path::Path;
use thiserror::Error;

/// Workspace error definitions
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Workspace '{0}' not found")]
    NotFound(String),
}

/// Workspace state metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub active_file: Option<String>,
    pub active_filter: Option<String>,
    pub scroll_line: usize,
}

/// User bookmark on a specific log line
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BookmarkRecord {
    pub id: String,
    pub workspace_id: String,
    pub file_path: String,
    pub line_number: usize,
    pub label: String,
}

/// Workspace database manager
pub struct WorkspaceStore {
    db: Database,
}

impl WorkspaceStore {
    /// Initialize workspace store with schema migrations applied.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WorkspaceError> {
        let db = Database::open(path)?;
        Self::init_schema(&db)?;
        Ok(Self { db })
    }

    /// Open in-memory store for testing.
    pub fn open_in_memory() -> Result<Self, WorkspaceError> {
        let db = Database::open_in_memory()?;
        Self::init_schema(&db)?;
        Ok(Self { db })
    }

    fn init_schema(db: &Database) -> Result<(), WorkspaceError> {
        let migrations = vec![Migration {
            version: 1,
            description: "create workspaces and bookmarks tables",
            sql: "CREATE TABLE IF NOT EXISTS workspaces (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    active_file TEXT,
                    active_filter TEXT,
                    scroll_line INTEGER NOT NULL DEFAULT 0,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                  );
                  CREATE TABLE IF NOT EXISTS bookmarks (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    line_number INTEGER NOT NULL,
                    label TEXT NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                  );",
        }];

        MigrationRunner::apply_migrations(db, &migrations)?;
        Ok(())
    }

    /// Save or update a workspace.
    pub fn save_workspace(&self, ws: &WorkspaceRecord) -> Result<(), WorkspaceError> {
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO workspaces (id, name, active_file, active_filter, scroll_line)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   active_file = excluded.active_file,
                   active_filter = excluded.active_filter,
                   scroll_line = excluded.scroll_line,
                   updated_at = CURRENT_TIMESTAMP;",
                rusqlite::params![
                    ws.id,
                    ws.name,
                    ws.active_file,
                    ws.active_filter,
                    ws.scroll_line as i64
                ],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Load a workspace by ID.
    pub fn load_workspace(&self, id: &str) -> Result<Option<WorkspaceRecord>, WorkspaceError> {
        let record = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, active_file, active_filter, scroll_line FROM workspaces WHERE id = ?1",
            )?;
            let mut rows = stmt.query([id])?;
            if let Some(row) = rows.next()? {
                let scroll_line_i64: i64 = row.get(4)?;
                Ok(Some(WorkspaceRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    active_file: row.get(2)?,
                    active_filter: row.get(3)?,
                    scroll_line: scroll_line_i64 as usize,
                }))
            } else {
                Ok(None)
            }
        })?;
        Ok(record)
    }

    /// Add a line bookmark.
    pub fn add_bookmark(&self, bm: &BookmarkRecord) -> Result<(), WorkspaceError> {
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO bookmarks (id, workspace_id, file_path, line_number, label)
                 VALUES (?1, ?2, ?3, ?4, ?5);",
                rusqlite::params![
                    bm.id,
                    bm.workspace_id,
                    bm.file_path,
                    bm.line_number as i64,
                    bm.label
                ],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// List all bookmarks for a specific log file.
    pub fn list_bookmarks(&self, file_path: &str) -> Result<Vec<BookmarkRecord>, WorkspaceError> {
        let bookmarks = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, workspace_id, file_path, line_number, label FROM bookmarks WHERE file_path = ?1 ORDER BY line_number ASC",
            )?;
            let rows = stmt.query_map([file_path], |row| {
                let line_number_i64: i64 = row.get(3)?;
                Ok(BookmarkRecord {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    file_path: row.get(2)?,
                    line_number: line_number_i64 as usize,
                    label: row.get(4)?,
                })
            })?;

            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        })?;
        Ok(bookmarks)
    }

    /// Remove a bookmark by ID.
    pub fn remove_bookmark(&self, id: &str) -> Result<bool, WorkspaceError> {
        let count = self.db.with_connection(|conn| {
            let affected = conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
            Ok(affected)
        })?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_save_and_load() {
        let store = WorkspaceStore::open_in_memory().unwrap();
        let ws = WorkspaceRecord {
            id: "ws_default".to_string(),
            name: "Production Incident #402".to_string(),
            active_file: Some("/var/log/nginx/access.log".to_string()),
            active_filter: Some("500 Internal Server Error".to_string()),
            scroll_line: 12500,
        };

        store.save_workspace(&ws).unwrap();
        let loaded = store.load_workspace("ws_default").unwrap().unwrap();
        assert_eq!(loaded, ws);
    }

    #[test]
    fn test_bookmark_lifecycle() {
        let store = WorkspaceStore::open_in_memory().unwrap();
        let bm1 = BookmarkRecord {
            id: "bm_1".to_string(),
            workspace_id: "ws_default".to_string(),
            file_path: "/var/log/syslog".to_string(),
            line_number: 1042,
            label: "Kernel OOM event".to_string(),
        };

        store.add_bookmark(&bm1).unwrap();
        let list = store.list_bookmarks("/var/log/syslog").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label, "Kernel OOM event");

        let removed = store.remove_bookmark("bm_1").unwrap();
        assert!(removed);
        assert!(store.list_bookmarks("/var/log/syslog").unwrap().is_empty());
    }
}
