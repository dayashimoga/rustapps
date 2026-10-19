#![forbid(unsafe_code)]
//! Shared Search Engine Subsystem
//!
//! Provides an embedded, sub-50ms Tantivy full-text and structured query engine.

use std::path::Path;
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};
use thiserror::Error;

/// Search error definitions
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Tantivy core error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("Failed to open index directory: {0}")]
    DirectoryOpenFailed(String),
    #[error("Failed to parse query '{query}': {reason}")]
    QueryParseFailed { query: String, reason: String },
    #[error("Schema error: field '{0}' not found in schema")]
    FieldNotFound(String),
}

/// A search result hit
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub score: f32,
    pub snippet: Option<String>,
}

/// Document to be indexed
#[derive(Debug, Clone)]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub timestamp: i64,
}

/// Reusable embedded search engine
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    field_id: Field,
    field_title: Field,
    field_content: Field,
    field_category: Field,
    field_timestamp: Field,
}

impl SearchEngine {
    /// Create an ephemeral in-memory search index.
    pub fn create_in_memory() -> Result<Self, SearchError> {
        let (schema, fields) = Self::build_schema();
        let index = Index::create_in_ram(schema);
        Self::init_with_index(index, fields)
    }

    /// Create or open a persistent directory-backed search index.
    pub fn create_in_dir<P: AsRef<Path>>(dir: P) -> Result<Self, SearchError> {
        let path = dir.as_ref();
        std::fs::create_dir_all(path).map_err(tantivy::TantivyError::from)?;

        let (schema, fields) = Self::build_schema();
        let mmap_dir = tantivy::directory::MmapDirectory::open(path)
            .map_err(|e| SearchError::DirectoryOpenFailed(e.to_string()))?;
        let exists = Index::exists(&mmap_dir)
            .map_err(|e| SearchError::DirectoryOpenFailed(e.to_string()))?;

        let index = if exists {
            Index::open_in_dir(path)?
        } else {
            Index::create_in_dir(path, schema)?
        };

        Self::init_with_index(index, fields)
    }

    fn build_schema() -> (Schema, (Field, Field, Field, Field, Field)) {
        let mut builder = Schema::builder();
        let field_id = builder.add_text_field("id", STRING | STORED);
        let field_title = builder.add_text_field("title", TEXT | STORED);
        let field_content = builder.add_text_field("content", TEXT);
        let field_category = builder.add_text_field("category", STRING | STORED);
        let field_timestamp = builder.add_i64_field("timestamp", INDEXED | STORED);
        let schema = builder.build();

        (
            schema,
            (
                field_id,
                field_title,
                field_content,
                field_category,
                field_timestamp,
            ),
        )
    }

    fn init_with_index(
        index: Index,
        fields: (Field, Field, Field, Field, Field),
    ) -> Result<Self, SearchError> {
        // Allocate a bounded 15MB index writer buffer
        let writer = index.writer(15_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            field_id: fields.0,
            field_title: fields.1,
            field_content: fields.2,
            field_category: fields.3,
            field_timestamp: fields.4,
        })
    }

    /// Add a single document to the index buffer.
    pub fn add_document(&self, doc: SearchDocument) -> Result<(), SearchError> {
        let writer = self
            .writer
            .lock()
            .map_err(|e| tantivy::TantivyError::InternalError(e.to_string()))?;

        let mut tantivy_doc = TantivyDocument::default();
        tantivy_doc.add_text(self.field_id, doc.id);
        tantivy_doc.add_text(self.field_title, doc.title);
        tantivy_doc.add_text(self.field_content, doc.content);
        tantivy_doc.add_text(self.field_category, doc.category);
        tantivy_doc.add_i64(self.field_timestamp, doc.timestamp);

        writer.add_document(tantivy_doc)?;
        Ok(())
    }

    /// Commit staged additions and flush to searcher.
    pub fn commit(&self) -> Result<(), SearchError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| tantivy::TantivyError::InternalError(e.to_string()))?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Execute a sub-50ms search query across indexed fields.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchHit>, SearchError> {
        let trimmed = query_str.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.field_title, self.field_content, self.field_category],
        );

        let query =
            query_parser
                .parse_query(trimmed)
                .map_err(|e| SearchError::QueryParseFailed {
                    query: trimmed.to_string(),
                    reason: e.to_string(),
                })?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut hits = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(self.field_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = retrieved_doc
                .get_first(self.field_title)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            hits.push(SearchHit {
                id,
                title,
                score,
                snippet: None,
            });
        }

        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_engine_roundtrip() {
        let engine = SearchEngine::create_in_memory().expect("Should create in-memory engine");

        engine
            .add_document(SearchDocument {
                id: "doc_1".to_string(),
                title: "Production Kubernetes Logs".to_string(),
                content: "Error in ingress controller upstream timeout connection reset"
                    .to_string(),
                category: "logs".to_string(),
                timestamp: 1729300000,
            })
            .unwrap();

        engine
            .add_document(SearchDocument {
                id: "doc_2".to_string(),
                title: "PostgreSQL Master Secret".to_string(),
                content: "Database credentials for high-availability cluster".to_string(),
                category: "vault".to_string(),
                timestamp: 1729300100,
            })
            .unwrap();

        engine.commit().unwrap();

        // Exact term search
        let hits = engine.search("Kubernetes", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "doc_1");

        // Content search
        let hits2 = engine.search("credentials", 10).unwrap();
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].id, "doc_2");

        // Empty search returns empty vector
        let empty_hits = engine.search("", 10).unwrap();
        assert!(empty_hits.is_empty());
    }

    #[test]
    fn test_search_invalid_syntax_handled_safely() {
        let engine = SearchEngine::create_in_memory().unwrap();
        // Syntax error in boolean query
        let result = engine.search("AND OR NOT", 10);
        assert!(result.is_err());
    }
}
