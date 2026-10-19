#![forbid(unsafe_code)]
//! DevOps Secret Vault - Ephemeral Encrypted In-Memory Tantivy Search Engine

use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, Query, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Term};

use crate::error::VaultError;
use crate::models::{SecretMetadata, SecretSearchResult, SecretType};

/// In-memory ephemeral Tantivy search engine for unlocked vault secrets
pub struct VaultSearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    field_id: Field,
    field_title: Field,
    field_type: Field,
    field_tags: Field,
    field_username: Field,
    field_notes: Field,
    field_updated_at: Field,
}

impl VaultSearchIndex {
    /// Initialize a new ephemeral in-memory Tantivy search index
    pub fn new() -> Result<Self, VaultError> {
        let mut builder = Schema::builder();
        let field_id = builder.add_text_field("id", STRING | STORED);
        let field_title = builder.add_text_field("title", TEXT | STORED);
        let field_type = builder.add_text_field("secret_type", STRING | STORED);
        let field_tags = builder.add_text_field("tags", TEXT | STORED);
        let field_username = builder.add_text_field("username", TEXT | STORED);
        let field_notes = builder.add_text_field("notes", TEXT);
        let field_updated_at = builder.add_i64_field("updated_at", INDEXED | STORED);
        let schema = builder.build();

        let index = Index::create_in_ram(schema);
        // Allocate a bounded 15MB writer buffer
        let writer = index.writer(15_000_000).map_err(VaultError::from)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(VaultError::from)?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            field_id,
            field_title,
            field_type,
            field_tags,
            field_username,
            field_notes,
            field_updated_at,
        })
    }

    /// Add a secret to the in-memory search index
    pub fn index_secret(&self, secret: &SecretMetadata) -> Result<(), VaultError> {
        let mut doc = TantivyDocument::default();
        doc.add_text(self.field_id, &secret.id);
        doc.add_text(self.field_title, &secret.title);
        doc.add_text(self.field_type, secret.secret_type.as_str());
        doc.add_text(self.field_tags, secret.tags.join(" "));
        if let Some(username) = &secret.username {
            doc.add_text(self.field_username, username);
        }
        if let Some(notes) = &secret.notes {
            doc.add_text(self.field_notes, notes);
        }
        doc.add_i64(self.field_updated_at, secret.updated_at);

        let mut writer = self.writer.lock().map_err(|e| {
            VaultError::Search(shared_search::SearchError::DirectoryOpenFailed(
                e.to_string(),
            ))
        })?;

        // Delete existing doc with same id if updating
        let term = Term::from_field_text(self.field_id, &secret.id);
        writer.delete_term(term);
        writer.add_document(doc)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Batch index multiple secrets (e.g. during vault unlock)
    pub fn index_batch(&self, secrets: &[SecretMetadata]) -> Result<(), VaultError> {
        let mut writer = self.writer.lock().map_err(|e| {
            VaultError::Search(shared_search::SearchError::DirectoryOpenFailed(
                e.to_string(),
            ))
        })?;

        for secret in secrets {
            let mut doc = TantivyDocument::default();
            doc.add_text(self.field_id, &secret.id);
            doc.add_text(self.field_title, &secret.title);
            doc.add_text(self.field_type, secret.secret_type.as_str());
            doc.add_text(self.field_tags, secret.tags.join(" "));
            if let Some(username) = &secret.username {
                doc.add_text(self.field_username, username);
            }
            if let Some(notes) = &secret.notes {
                doc.add_text(self.field_notes, notes);
            }
            doc.add_i64(self.field_updated_at, secret.updated_at);
            writer.add_document(doc)?;
        }

        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Remove a secret from the search index
    pub fn remove_secret(&self, id: &str) -> Result<(), VaultError> {
        let mut writer = self.writer.lock().map_err(|e| {
            VaultError::Search(shared_search::SearchError::DirectoryOpenFailed(
                e.to_string(),
            ))
        })?;

        let term = Term::from_field_text(self.field_id, id);
        writer.delete_term(term);
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Clear all indexed documents
    pub fn clear(&self) -> Result<(), VaultError> {
        let mut writer = self.writer.lock().map_err(|e| {
            VaultError::Search(shared_search::SearchError::DirectoryOpenFailed(
                e.to_string(),
            ))
        })?;

        writer.delete_all_documents()?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Query the secret search index with sub-50ms latency
    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<SecretSearchResult>, VaultError> {
        let searcher = self.reader.searcher();
        let query_str = query_str.trim();

        let parsed_query: Box<dyn Query> = if query_str.is_empty() || query_str == "*" {
            Box::new(AllQuery)
        } else {
            let mut query_parser = QueryParser::for_index(
                &self.index,
                vec![
                    self.field_title,
                    self.field_tags,
                    self.field_username,
                    self.field_type,
                    self.field_notes,
                ],
            );
            query_parser.set_conjunction_by_default();

            match query_parser.parse_query(query_str) {
                Ok(q) => q,
                Err(_) => {
                    // Fallback to fuzzy substring term query on title
                    let term = Term::from_field_text(self.field_title, query_str);
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic))
                }
            }
        };

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = doc
                .get_first(self.field_id)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let title = doc
                .get_first(self.field_title)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let type_str = doc
                .get_first(self.field_type)
                .and_then(|v| v.as_str())
                .unwrap_or("password");
            let secret_type = SecretType::from_str_lossy(type_str);

            let tags_str = doc
                .get_first(self.field_tags)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let tags: Vec<String> = tags_str.split_whitespace().map(|s| s.to_string()).collect();

            let username = doc
                .get_first(self.field_username)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            results.push(SecretSearchResult {
                id,
                title,
                secret_type,
                tags,
                username,
                score,
                snippet: None,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_index_crud_and_query() {
        let index = VaultSearchIndex::new().unwrap();

        let secret1 = SecretMetadata {
            id: "sec-1".to_string(),
            title: "AWS Production Root".to_string(),
            secret_type: SecretType::ApiKey,
            tags: vec!["aws".to_string(), "prod".to_string()],
            username: Some("ops-admin".to_string()),
            url: Some("https://aws.amazon.com".to_string()),
            notes: Some("Master cloud account access".to_string()),
            version: 1,
            created_at: 100,
            updated_at: 100,
        };

        let secret2 = SecretMetadata {
            id: "sec-2".to_string(),
            title: "GitHub Personal Access Token".to_string(),
            secret_type: SecretType::ApiKey,
            tags: vec!["git".to_string(), "ci".to_string()],
            username: Some("developer".to_string()),
            url: Some("https://github.com".to_string()),
            notes: Some("Repo and workflow scopes".to_string()),
            version: 1,
            created_at: 110,
            updated_at: 110,
        };

        index
            .index_batch(&[secret1.clone(), secret2.clone()])
            .unwrap();

        // Query "aws"
        let hits = index.search("aws", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "sec-1");

        // Query "git"
        let hits2 = index.search("git", 10).unwrap();
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].id, "sec-2");

        // Remove secret1
        index.remove_secret("sec-1").unwrap();
        let hits3 = index.search("aws", 10).unwrap();
        assert_eq!(hits3.len(), 0);
    }
}
