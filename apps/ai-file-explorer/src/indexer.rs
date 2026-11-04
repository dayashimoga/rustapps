#![forbid(unsafe_code)]
//! Tantivy Embedded Metadata Indexer for High-Speed File Search (<50ms SLA)
//!
//! Extended in Phase 6 with full-text content indexing, document metadata,
//! and OCR token indexing for instant content & semantic retrieval.

use crate::models::{ExtractedDocument, FileItem, SemanticSearchResult};
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

/// Embedded Tantivy file metadata and content indexer
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
    field_content_text: Field,
    field_doc_title: Field,
    field_doc_author: Field,
    field_ocr_tokens: Field,
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

    #[allow(clippy::type_complexity)]
    fn build_schema() -> (
        Schema,
        (
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
        ),
    ) {
        let mut builder = Schema::builder();
        let field_path = builder.add_text_field("path", STRING | STORED);
        let field_file_name = builder.add_text_field("file_name", TEXT | STORED);
        let field_extension = builder.add_text_field("extension", STRING | STORED);
        let field_size_bytes = builder.add_u64_field("size_bytes", INDEXED | STORED | FAST);
        let field_modified_timestamp =
            builder.add_u64_field("modified_timestamp", INDEXED | STORED | FAST);
        let field_is_dir = builder.add_u64_field("is_dir", INDEXED | STORED);
        let field_content_text = builder.add_text_field("content_text", TEXT | STORED);
        let field_doc_title = builder.add_text_field("doc_title", TEXT | STORED);
        let field_doc_author = builder.add_text_field("doc_author", STRING | STORED);
        let field_ocr_tokens = builder.add_text_field("ocr_tokens", TEXT);

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
                field_content_text,
                field_doc_title,
                field_doc_author,
                field_ocr_tokens,
            ),
        )
    }

    #[allow(clippy::type_complexity)]
    fn init_with_index(
        index: Index,
        fields: (
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
            Field,
        ),
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
            field_content_text: fields.6,
            field_doc_title: fields.7,
            field_doc_author: fields.8,
            field_ocr_tokens: fields.9,
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

    /// Indexes an extracted document with its content text and metadata
    pub fn index_document(&self, doc: &ExtractedDocument) -> Result<(), IndexerError> {
        let mut writer = self.writer.lock().map_err(|_| IndexerError::LockError)?;

        let mut t_doc = TantivyDocument::default();
        t_doc.add_text(self.field_path, doc.path.to_string_lossy());
        t_doc.add_text(self.field_file_name, &doc.file_name);
        t_doc.add_text(self.field_extension, doc.extension.as_deref().unwrap_or(""));
        t_doc.add_u64(self.field_size_bytes, doc.content_text.len() as u64);
        t_doc.add_u64(self.field_modified_timestamp, doc.extracted_at_secs);
        t_doc.add_u64(self.field_is_dir, 0);

        // Content & metadata fields
        t_doc.add_text(self.field_content_text, &doc.content_text);
        if let Some(ref title) = doc.metadata.title {
            t_doc.add_text(self.field_doc_title, title);
        }
        if let Some(ref author) = doc.metadata.author {
            t_doc.add_text(self.field_doc_author, author);
        }

        writer.add_document(t_doc)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Remove a file path from the Tantivy index
    pub fn delete_file(&self, path: &Path) -> Result<(), IndexerError> {
        let mut writer = self.writer.lock().map_err(|_| IndexerError::LockError)?;
        let term = Term::from_field_text(self.field_path, &path.to_string_lossy());
        writer.delete_term(term);
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Indexes OCR tokens for an image file
    pub fn index_ocr_tokens(&self, path: &Path, tokens: &str) -> Result<(), IndexerError> {
        let mut writer = self.writer.lock().map_err(|_| IndexerError::LockError)?;

        let mut t_doc = TantivyDocument::default();
        t_doc.add_text(self.field_path, path.to_string_lossy());
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        t_doc.add_text(self.field_file_name, &file_name);
        let ext = path
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        t_doc.add_text(self.field_extension, &ext);
        t_doc.add_u64(self.field_size_bytes, 0);
        t_doc.add_u64(self.field_modified_timestamp, 0);
        t_doc.add_u64(self.field_is_dir, 0);
        t_doc.add_text(self.field_ocr_tokens, tokens);

        writer.add_document(t_doc)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Search indexed files by filename, path, or extension with sub-50ms query execution SLA
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<FileItem>, IndexerError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.field_file_name, self.field_path, self.field_extension],
        );

        let query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(e) => {
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

    /// Full-text content search across document bodies, titles, authors, and OCR tokens
    pub fn search_content(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<SemanticSearchResult>, IndexerError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.field_content_text,
                self.field_doc_title,
                self.field_doc_author,
                self.field_ocr_tokens,
                self.field_file_name,
                self.field_extension,
            ],
        );

        let clean_query = if let Some(stripped) = query_str.strip_prefix("*.") {
            stripped
        } else if let Some(stripped) = query_str.strip_prefix('.') {
            if stripped.is_empty() {
                query_str
            } else {
                stripped
            }
        } else {
            query_str
        };

        let query: Box<dyn tantivy::query::Query> =
            if clean_query.trim().is_empty() || clean_query.trim() == "*" {
                Box::new(tantivy::query::AllQuery)
            } else {
                match query_parser.parse_query(clean_query) {
                    Ok(q) => q,
                    Err(_e) => {
                        let escaped = clean_query.replace(':', " ").replace('\\', "/");
                        query_parser
                            .parse_query(&escaped)
                            .unwrap_or_else(|_| Box::new(tantivy::query::AllQuery))
                    }
                }
            };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
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
                .unwrap_or_default()
                .to_uppercase();

            let content_snippet = doc
                .get_first(self.field_content_text)
                .and_then(|v| v.as_str())
                .map(|text| extract_snippet(text, query_str, 120))
                .unwrap_or_else(|| {
                    doc.get_first(self.field_doc_title)
                        .and_then(|v| v.as_str())
                        .unwrap_or("Match found")
                        .to_string()
                });

            results.push(SemanticSearchResult {
                path: PathBuf::from(path_str),
                file_name,
                score,
                snippet: content_snippet,
                format: if ext_str.is_empty() {
                    "DOC".to_string()
                } else {
                    ext_str
                },
            });
        }

        Ok(results)
    }
}

/// Helper function to extract a contextual text snippet around matched keywords
fn extract_snippet(text: &str, query: &str, max_len: usize) -> String {
    let lower_text = text.to_lowercase();
    let lower_q = query.to_lowercase();

    if let Some(pos) = lower_text.find(&lower_q) {
        let start = pos.saturating_sub(max_len / 3);
        let end = (start + max_len).min(text.len());
        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < text.len() { "..." } else { "" };
        format!("{}{}{}", prefix, text[start..end].trim(), suffix)
    } else {
        let end = max_len.min(text.len());
        let suffix = if end < text.len() { "..." } else { "" };
        format!("{}{}", text[..end].trim(), suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DocumentMetadata;

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
        ];

        let indexed = indexer.index_files(&files).unwrap();
        assert_eq!(indexed, 2);

        let hits = indexer.search("main", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_name, "main.rs");
    }

    #[test]
    fn test_indexer_content_search() {
        let indexer = FileSearchIndexer::create_in_memory().unwrap();

        let doc = ExtractedDocument {
            path: PathBuf::from("/docs/security_spec.md"),
            file_name: "security_spec.md".to_string(),
            extension: Some("md".to_string()),
            metadata: DocumentMetadata {
                title: Some("Security Architecture Blueprint".to_string()),
                author: Some("Security Architect".to_string()),
                page_count: None,
                word_count: 50,
                format: "Markdown Document".to_string(),
                creation_date: None,
            },
            content_text: "This document outlines the Argon2id key derivation parameters and AES-256-GCM symmetric cipher.".to_string(),
            language: Some("markdown".to_string()),
            extracted_at_secs: 1700000000,
        };

        indexer.index_document(&doc).unwrap();

        let hits = indexer.search_content("Argon2id", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_name, "security_spec.md");
        assert!(hits[0].snippet.contains("Argon2id"));
    }
}
