#![forbid(unsafe_code)]
//! Tantivy Embedded Metadata Indexer for High-Speed File Search (<50ms SLA)

use crate::models::FileItem;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("Failed to open directory: {0}")]
    OpenDirectory(#[from] tantivy::directory::error::OpenDirectoryError),
    #[error("Failed to parse query '{query}': {reason}")]
    QueryParseFailed { query: String, reason: String },
    #[error("Index lock error")]
    LockError,
}

/// Embedded Tantivy file metadata indexer
pub struct FileSearchIndexer {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    field_path: Field,
    field_file_name: Field,
    field_extension: Field,
    field_size_bytes: Field,
    field_modified_timestamp: Field,
    field_is_dir: Field,
}

impl FileSearchIndexer {
    /// Initialize an ephemeral in-memory Tantivy index
    pub fn create_in_memory() -> Result<Self, IndexerError> {
        let (schema, fields) = Self::build_schema();
        let index = Index::create_in_ram(schema);
        Self::init_with_index(index, fields)
    }

    /// Open or create an on-disk Tantivy index directory
    pub fn open_or_create<P: AsRef<Path>>(dir_path: P) -> Result<Self, IndexerError> {
        let path = dir_path.as_ref();
        std::fs::create_dir_all(path)
            .map_err(|e| IndexerError::Tantivy(tantivy::TantivyError::IoError(Arc::new(e))))?;

        let (schema, fields) = Self::build_schema();
        let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;
        Self::init_with_index(index, fields)
    }

    fn build_schema() -> (Schema, (Field, Field, Field, Field, Field, Field)) {
        let mut builder = Schema::builder();
        let field_path = builder.add_text_field("path", STRING | STORED);
        let field_file_name = builder.add_text_field("file_name", TEXT | STORED);
        let field_extension = builder.add_text_field("extension", STRING | STORED);
        let field_size_bytes = builder.add_u64_field("size_bytes", INDEXED | STORED | FAST);
        let field_modified_timestamp =
            builder.add_u64_field("modified_timestamp", INDEXED | STORED | FAST);
        let field_is_dir = builder.add_u64_field("is_dir", INDEXED | STORED);

        let schema = builder.build();
        (
            schema,
            (
                field_path,
                field_file_name,
                field_extension,
                field_size_bytes,
                field_modified_timestamp,
                field_is_dir,
            ),
        )
    }

    fn init_with_index(
        index: Index,
        fields: (Field, Field, Field, Field, Field, Field),
    ) -> Result<Self, IndexerError> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Allocate 30MB for indexing buffer (leaves plenty within 300MB overall RAM SLA)
        let writer = index.writer(30_000_000)?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            field_path: fields.0,
            field_file_name: fields.1,
            field_extension: fields.2,
            field_size_bytes: fields.3,
            field_modified_timestamp: fields.4,
            field_is_dir: fields.5,
        })
    }

    /// Add a batch of file records to the index and commit
    pub fn index_files(&self, items: &[FileItem]) -> Result<usize, IndexerError> {
        let mut writer = self.writer.lock().map_err(|_| IndexerError::LockError)?;

        for item in items {
            let mut doc = TantivyDocument::default();
            doc.add_text(self.field_path, item.path.to_string_lossy());
            doc.add_text(self.field_file_name, &item.file_name);
            doc.add_text(
                self.field_extension,
                item.extension.as_deref().unwrap_or(""),
            );
            doc.add_u64(self.field_size_bytes, item.size_bytes);
            doc.add_u64(self.field_modified_timestamp, item.modified_timestamp_secs);
            doc.add_u64(self.field_is_dir, if item.is_dir { 1 } else { 0 });

            writer.add_document(doc)?;
        }

        writer.commit()?;
        self.reader.reload()?;
        Ok(items.len())
    }

    /// Search indexed files with sub-50ms query execution SLA
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<FileItem>, IndexerError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.field_file_name, self.field_path, self.field_extension],
        );

        let query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(e) => {
                // If standard parse fails, fallback to prefix/wildcard match
                let escaped = query_str.replace(':', " ").replace('\\', "/");
                query_parser.parse_query(&escaped).map_err(|err| {
                    IndexerError::QueryParseFailed {
                        query: query_str.to_string(),
                        reason: format!("{} ({})", err, e),
                    }
                })?
            }
        };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::with_capacity(top_docs.len());

        for (_score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let path_str = doc
                .get_first(self.field_path)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let file_name = doc
                .get_first(self.field_file_name)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let ext_str = doc
                .get_first(self.field_extension)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let size_bytes = doc
                .get_first(self.field_size_bytes)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let modified_timestamp_secs = doc
                .get_first(self.field_modified_timestamp)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let is_dir = doc
                .get_first(self.field_is_dir)
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                == 1;

            let extension = if ext_str.is_empty() {
                None
            } else {
                Some(ext_str.to_string())
            };

            results.push(FileItem {
                path: PathBuf::from(path_str),
                file_name,
                extension,
                size_bytes,
                is_dir,
                modified_timestamp_secs,
                is_readonly: false,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer_crud_and_search() {
        let indexer = FileSearchIndexer::create_in_memory().unwrap();

        let files = vec![
            FileItem {
                path: PathBuf::from("H:/projects/app/main.rs"),
                file_name: "main.rs".to_string(),
                extension: Some("rs".to_string()),
                size_bytes: 4096,
                is_dir: false,
                modified_timestamp_secs: 1700000000,
                is_readonly: false,
            },
            FileItem {
                path: PathBuf::from("H:/projects/app/Cargo.toml"),
                file_name: "Cargo.toml".to_string(),
                extension: Some("toml".to_string()),
                size_bytes: 1024,
                is_dir: false,
                modified_timestamp_secs: 1700000001,
                is_readonly: false,
            },
            FileItem {
                path: PathBuf::from("H:/projects/app/docs"),
                file_name: "docs".to_string(),
                extension: None,
                size_bytes: 0,
                is_dir: true,
                modified_timestamp_secs: 1700000002,
                is_readonly: false,
            },
        ];

        let indexed = indexer.index_files(&files).unwrap();
        assert_eq!(indexed, 3);

        let hits = indexer.search("main", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_name, "main.rs");

        let toml_hits = indexer.search("Cargo", 10).unwrap();
        assert_eq!(toml_hits.len(), 1);
        assert_eq!(toml_hits[0].file_name, "Cargo.toml");
    }

    #[test]
    fn test_indexer_empty_search_returns_empty() {
        let indexer = FileSearchIndexer::create_in_memory().unwrap();
        let hits = indexer.search("nonexistent_file_pattern_xyz", 10).unwrap();
        assert!(hits.is_empty());
    }
}
