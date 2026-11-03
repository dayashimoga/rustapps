#![forbid(unsafe_code)]
//! SQLite WAL-Backed Offline Phrasebook, History & Terminology Glossary Store
//!
//! Provides zero-network, local persistence for categorized phrasebook entries,
//! favorite expressions, full translation audit history, and customizable
//! domain terminology glossaries.

use crate::models::{GlossaryTerm, LanguageCode, PhraseCategory, PhrasebookItem};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Phrasebook storage error definitions
#[derive(Debug, Error)]
pub enum PhrasebookError {
    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Item with id '{0}' was not found")]
    NotFound(String),
}

/// Audit record for historical translation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationHistoryRecord {
    pub id: String,
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
    pub original_text: String,
    pub translated_text: String,
    pub timestamp_secs: u64,
    pub confidence: f32,
}

/// Persistent thread-safe SQLite phrasebook and glossary repository
pub struct PhrasebookStore {
    conn: Arc<Mutex<Connection>>,
}

impl PhrasebookStore {
    /// Open or create SQLite phrasebook database with WAL journaling
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PhrasebookError> {
        let conn = Connection::open(path)?;
        Self::initialize_connection(conn)
    }

    /// Open ephemeral in-memory SQLite phrasebook database (useful for unit tests)
    pub fn open_in_memory() -> Result<Self, PhrasebookError> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_connection(conn)
    }

    fn initialize_connection(conn: Connection) -> Result<Self, PhrasebookError> {
        // Configure WAL mode and performance pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        // Initialize schema tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS phrasebook_items (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                source_text TEXT NOT NULL,
                source_lang TEXT NOT NULL,
                target_text TEXT NOT NULL,
                target_lang TEXT NOT NULL,
                notes TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                created_at_secs INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_phrasebook_category ON phrasebook_items(category);
            CREATE INDEX IF NOT EXISTS idx_phrasebook_favorite ON phrasebook_items(is_favorite);

            CREATE TABLE IF NOT EXISTS translation_history (
                id TEXT PRIMARY KEY,
                source_lang TEXT NOT NULL,
                target_lang TEXT NOT NULL,
                original_text TEXT NOT NULL,
                translated_text TEXT NOT NULL,
                timestamp_secs INTEGER NOT NULL,
                confidence REAL NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON translation_history(timestamp_secs DESC);

            CREATE TABLE IF NOT EXISTS domain_glossaries (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                source_term TEXT NOT NULL,
                target_term TEXT NOT NULL,
                case_sensitive INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_glossary_domain ON domain_glossaries(domain);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_glossary_unique ON domain_glossaries(domain, source_term);",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // -----------------------------------------------------------------------
    // Phrasebook CRUD
    // -----------------------------------------------------------------------

    /// Insert or update phrasebook item
    pub fn add_phrase(&self, item: &PhrasebookItem) -> Result<(), PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let cat_str = serde_json::to_string(&item.category)?;
        let cat_clean = cat_str.trim_matches('"');

        conn.execute(
            "INSERT OR REPLACE INTO phrasebook_items 
             (id, category, source_text, source_lang, target_text, target_lang, notes, is_favorite, created_at_secs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                item.id,
                cat_clean,
                item.source_text,
                item.source_lang.as_str(),
                item.target_text,
                item.target_lang.as_str(),
                item.notes,
                if item.is_favorite { 1 } else { 0 },
                item.created_at_secs,
            ],
        )?;
        Ok(())
    }

    /// Retrieve single phrase by ID
    pub fn get_phrase(&self, id: &str) -> Result<Option<PhrasebookItem>, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, category, source_text, source_lang, target_text, target_lang, notes, is_favorite, created_at_secs
             FROM phrasebook_items WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let cat_str: String = row.get(1)?;
            let category: PhraseCategory =
                serde_json::from_str(&format!("\"{}\"", cat_str)).unwrap_or(PhraseCategory::Custom);
            let s_lang_str: String = row.get(3)?;
            let t_lang_str: String = row.get(5)?;
            let source_lang = LanguageCode::parse(&s_lang_str).unwrap_or(LanguageCode::En);
            let target_lang = LanguageCode::parse(&t_lang_str).unwrap_or(LanguageCode::Es);
            let is_fav: i32 = row.get(7)?;

            Ok(Some(PhrasebookItem {
                id: row.get(0)?,
                category,
                source_text: row.get(2)?,
                source_lang,
                target_text: row.get(4)?,
                target_lang,
                notes: row.get(6)?,
                is_favorite: is_fav == 1,
                created_at_secs: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List phrases with optional category filter and favorite filter
    pub fn list_phrases(
        &self,
        category: Option<PhraseCategory>,
        favorite_only: bool,
    ) -> Result<Vec<PhrasebookItem>, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, category, source_text, source_lang, target_text, target_lang, notes, is_favorite, created_at_secs FROM phrasebook_items WHERE 1=1".to_string();

        if favorite_only {
            sql.push_str(" AND is_favorite = 1");
        }

        let cat_clean = category.map(|c| {
            let s = serde_json::to_string(&c).unwrap_or_default();
            s.trim_matches('"').to_string()
        });

        if cat_clean.is_some() {
            sql.push_str(" AND category = ?1");
        }
        sql.push_str(" ORDER BY created_at_secs DESC");

        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(ref cat) = cat_clean {
            stmt.query_map(params![cat], Self::map_phrase_row)?
        } else {
            stmt.query_map([], Self::map_phrase_row)?
        };

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    fn map_phrase_row(row: &rusqlite::Row) -> rusqlite::Result<PhrasebookItem> {
        let cat_str: String = row.get(1)?;
        let category: PhraseCategory =
            serde_json::from_str(&format!("\"{}\"", cat_str)).unwrap_or(PhraseCategory::Custom);
        let s_lang_str: String = row.get(3)?;
        let t_lang_str: String = row.get(5)?;
        let source_lang = LanguageCode::parse(&s_lang_str).unwrap_or(LanguageCode::En);
        let target_lang = LanguageCode::parse(&t_lang_str).unwrap_or(LanguageCode::Es);
        let is_fav: i32 = row.get(7)?;

        Ok(PhrasebookItem {
            id: row.get(0)?,
            category,
            source_text: row.get(2)?,
            source_lang,
            target_text: row.get(4)?,
            target_lang,
            notes: row.get(6)?,
            is_favorite: is_fav == 1,
            created_at_secs: row.get(8)?,
        })
    }

    /// Toggle favorite status of a phrasebook item (returns new status)
    pub fn toggle_favorite(&self, id: &str) -> Result<bool, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT is_favorite FROM phrasebook_items WHERE id = ?1")?;
        let current_fav: i32 = stmt
            .query_row(params![id], |row| row.get(0))
            .map_err(|_| PhrasebookError::NotFound(id.to_string()))?;

        let new_fav = if current_fav == 1 { 0 } else { 1 };
        conn.execute(
            "UPDATE phrasebook_items SET is_favorite = ?1 WHERE id = ?2",
            params![new_fav, id],
        )?;

        Ok(new_fav == 1)
    }

    /// Delete phrasebook item by ID (returns true if deleted)
    pub fn delete_phrase(&self, id: &str) -> Result<bool, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM phrasebook_items WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // -----------------------------------------------------------------------
    // Translation History
    // -----------------------------------------------------------------------

    /// Append translation execution to audit history
    pub fn add_history(&self, record: &TranslationHistoryRecord) -> Result<(), PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO translation_history 
             (id, source_lang, target_lang, original_text, translated_text, timestamp_secs, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.id,
                record.source_lang.as_str(),
                record.target_lang.as_str(),
                record.original_text,
                record.translated_text,
                record.timestamp_secs,
                record.confidence,
            ],
        )?;
        Ok(())
    }

    /// Retrieve recent translation history ordered newest first
    pub fn get_history(
        &self,
        limit: usize,
    ) -> Result<Vec<TranslationHistoryRecord>, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, source_lang, target_lang, original_text, translated_text, timestamp_secs, confidence
             FROM translation_history ORDER BY timestamp_secs DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let s_lang_str: String = row.get(1)?;
            let t_lang_str: String = row.get(2)?;
            let source_lang = LanguageCode::parse(&s_lang_str).unwrap_or(LanguageCode::En);
            let target_lang = LanguageCode::parse(&t_lang_str).unwrap_or(LanguageCode::Es);

            Ok(TranslationHistoryRecord {
                id: row.get(0)?,
                source_lang,
                target_lang,
                original_text: row.get(3)?,
                translated_text: row.get(4)?,
                timestamp_secs: row.get(5)?,
                confidence: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Clear all historical translation audit records
    pub fn clear_history(&self) -> Result<(), PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM translation_history", [])?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Domain Glossaries
    // -----------------------------------------------------------------------

    /// Add or update term in domain glossary
    pub fn add_glossary_term(
        &self,
        domain: &str,
        term: &GlossaryTerm,
    ) -> Result<(), PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let id = format!(
            "{}:{}",
            domain.to_lowercase(),
            term.source_term.to_lowercase()
        );
        conn.execute(
            "INSERT OR REPLACE INTO domain_glossaries 
             (id, domain, source_term, target_term, case_sensitive)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                domain.to_lowercase(),
                term.source_term,
                term.target_term,
                if term.case_sensitive { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    /// Retrieve all terms registered under a domain glossary
    pub fn get_glossary_terms(&self, domain: &str) -> Result<Vec<GlossaryTerm>, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source_term, target_term, case_sensitive 
             FROM domain_glossaries WHERE domain = ?1 ORDER BY source_term ASC",
        )?;

        let rows = stmt.query_map(params![domain.to_lowercase()], |row| {
            let cs: i32 = row.get(2)?;
            Ok(GlossaryTerm {
                source_term: row.get(0)?,
                target_term: row.get(1)?,
                case_sensitive: cs == 1,
            })
        })?;

        let mut terms = Vec::new();
        for r in rows {
            terms.push(r?);
        }
        Ok(terms)
    }

    /// List all unique glossary domain names
    pub fn list_glossary_domains(&self) -> Result<Vec<String>, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT DISTINCT domain FROM domain_glossaries ORDER BY domain ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;

        let mut domains = Vec::new();
        for r in rows {
            domains.push(r?);
        }
        Ok(domains)
    }

    /// Delete term from domain glossary
    pub fn delete_glossary_term(
        &self,
        domain: &str,
        source_term: &str,
    ) -> Result<bool, PhrasebookError> {
        let conn = self.conn.lock().unwrap();
        let id = format!("{}:{}", domain.to_lowercase(), source_term.to_lowercase());
        let affected = conn.execute("DELETE FROM domain_glossaries WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phrasebook_crud_and_favorites() {
        let store = PhrasebookStore::open_in_memory().unwrap();

        let item = PhrasebookItem {
            id: "phrase-1".to_string(),
            category: PhraseCategory::Dining,
            source_text: "Where is the best restaurant?".to_string(),
            source_lang: LanguageCode::En,
            target_text: "¿Dónde está el mejor restaurante?".to_string(),
            target_lang: LanguageCode::Es,
            notes: Some("Travel phrase".to_string()),
            is_favorite: false,
            created_at_secs: 1729300000,
        };

        store.add_phrase(&item).unwrap();

        let fetched = store.get_phrase("phrase-1").unwrap().unwrap();
        assert_eq!(fetched.source_text, item.source_text);
        assert!(!fetched.is_favorite);

        // Toggle favorite
        let new_fav = store.toggle_favorite("phrase-1").unwrap();
        assert!(new_fav);

        let fav_list = store.list_phrases(None, true).unwrap();
        assert_eq!(fav_list.len(), 1);
        assert_eq!(fav_list[0].id, "phrase-1");

        // Filter by category
        let dining_list = store
            .list_phrases(Some(PhraseCategory::Dining), false)
            .unwrap();
        assert_eq!(dining_list.len(), 1);

        let emergency_list = store
            .list_phrases(Some(PhraseCategory::Emergency), false)
            .unwrap();
        assert_eq!(emergency_list.len(), 0);

        // Delete
        assert!(store.delete_phrase("phrase-1").unwrap());
        assert!(store.get_phrase("phrase-1").unwrap().is_none());
    }

    #[test]
    fn test_translation_history_audit() {
        let store = PhrasebookStore::open_in_memory().unwrap();

        let rec = TranslationHistoryRecord {
            id: "hist-1".to_string(),
            source_lang: LanguageCode::En,
            target_lang: LanguageCode::Fr,
            original_text: "Good morning".to_string(),
            translated_text: "Bonjour".to_string(),
            timestamp_secs: 1729300100,
            confidence: 0.98,
        };

        store.add_history(&rec).unwrap();

        let history = store.get_history(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].original_text, "Good morning");
        assert_eq!(history[0].translated_text, "Bonjour");

        store.clear_history().unwrap();
        assert_eq!(store.get_history(10).unwrap().len(), 0);
    }

    #[test]
    fn test_domain_glossary_management() {
        let store = PhrasebookStore::open_in_memory().unwrap();

        let term1 = GlossaryTerm {
            source_term: "API Gateway".to_string(),
            target_term: "Pasarela de API".to_string(),
            case_sensitive: true,
        };
        let term2 = GlossaryTerm {
            source_term: "Load Balancer".to_string(),
            target_term: "Equilibrador de Carga".to_string(),
            case_sensitive: false,
        };

        store.add_glossary_term("cloud", &term1).unwrap();
        store.add_glossary_term("cloud", &term2).unwrap();

        let domains = store.list_glossary_domains().unwrap();
        assert_eq!(domains, vec!["cloud"]);

        let terms = store.get_glossary_terms("cloud").unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0].source_term, "API Gateway");

        // Delete term
        assert!(store.delete_glossary_term("cloud", "API Gateway").unwrap());
        assert_eq!(store.get_glossary_terms("cloud").unwrap().len(), 1);
    }
}
