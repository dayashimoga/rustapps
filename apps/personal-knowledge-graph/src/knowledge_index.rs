#![forbid(unsafe_code)]
//! Tantivy Full-Text Indexing & Dense Semantic Retrieval for Knowledge Graph

use crate::models::KnowledgeNode;
use shared_ai_core::ConceptEmbeddingEngine;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("Directory error: {0}")]
    Directory(#[from] tantivy::directory::error::OpenDirectoryError),
    #[error("Query parse error: {0}")]
    QueryParse(String),
}

/// Hybrid Knowledge Search Index combining Tantivy BM25 and Semantic Vectors
pub struct KnowledgeSearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    field_id: Field,
    field_title: Field,
    field_content: Field,
    field_node_type: Field,
    field_tags: Field,
    embedding_engine: ConceptEmbeddingEngine,
}

impl KnowledgeSearchIndex {
    fn build_schema() -> (Schema, (Field, Field, Field, Field, Field)) {
        let mut builder = Schema::builder();
        let field_id = builder.add_text_field("id", STRING | STORED);
        let field_title = builder.add_text_field("title", TEXT | STORED);
        let field_content = builder.add_text_field("content", TEXT);
        let field_node_type = builder.add_text_field("node_type", STRING | STORED);
        let field_tags = builder.add_text_field("tags", TEXT | STORED);
        let schema = builder.build();
        (
            schema,
            (
                field_id,
                field_title,
                field_content,
                field_node_type,
                field_tags,
            ),
        )
    }

    pub fn open_in_ram() -> Result<Self, IndexError> {
        let (schema, fields) = Self::build_schema();
        let index = Index::create_in_ram(schema);
        Self::init(index, fields)
    }

    pub fn open_or_create<P: AsRef<Path>>(dir_path: P) -> Result<Self, IndexError> {
        let path = dir_path.as_ref();
        std::fs::create_dir_all(path)
            .map_err(|e| IndexError::Tantivy(tantivy::TantivyError::IoError(Arc::new(e))))?;

        let (schema, fields) = Self::build_schema();
        let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;
        Self::init(index, fields)
    }

    fn init(index: Index, fields: (Field, Field, Field, Field, Field)) -> Result<Self, IndexError> {
        let (field_id, field_title, field_content, field_node_type, field_tags) = fields;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let writer = Arc::new(Mutex::new(index.writer(15_000_000)?));
        let embedding_engine = ConceptEmbeddingEngine::new_standard();

        Ok(Self {
            index,
            reader,
            writer,
            field_id,
            field_title,
            field_content,
            field_node_type,
            field_tags,
            embedding_engine,
        })
    }

    /// Index or update a knowledge node
    pub fn index_node(&self, node: &KnowledgeNode) -> Result<(), IndexError> {
        let mut writer = self.writer.lock().unwrap();
        // Remove existing document with the same ID
        let term = tantivy::Term::from_field_text(self.field_id, &node.id);
        writer.delete_term(term);

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(self.field_id, &node.id);
        doc.add_text(self.field_title, &node.title);
        doc.add_text(self.field_content, &node.content);
        doc.add_text(self.field_node_type, node.node_type.as_str());
        doc.add_text(self.field_tags, node.tags.join(" "));

        writer.add_document(doc)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Delete a node from the search index
    pub fn delete_node(&self, id: &str) -> Result<(), IndexError> {
        let mut writer = self.writer.lock().unwrap();
        let term = tantivy::Term::from_field_text(self.field_id, id);
        writer.delete_term(term);
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// BM25 full-text search across titles, content, and tags
    pub fn search_text(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<(String, f32)>, IndexError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.field_title, self.field_content, self.field_tags],
        );

        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| IndexError::QueryParse(e.to_string()))?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_val) = retrieved_doc.get_first(self.field_id) {
                if let Some(id_str) = id_val.as_str() {
                    results.push((id_str.to_string(), score));
                }
            }
        }
        Ok(results)
    }

    /// Rank candidates by semantic cosine similarity against the query embedding
    pub fn semantic_search(
        &self,
        query: &str,
        candidates: &[KnowledgeNode],
        limit: usize,
    ) -> Vec<(KnowledgeNode, f32)> {
        let query_embedding = self.embedding_engine.embed_text(query);
        let mut scored: Vec<(KnowledgeNode, f32)> = candidates
            .iter()
            .map(|node| {
                let node_text = format!("{} {} {}", node.title, node.content, node.tags.join(" "));
                let node_embedding = self.embedding_engine.embed_text(&node_text);
                let sim = query_embedding.cosine_similarity(&node_embedding);
                (node.clone(), sim)
            })
            .filter(|(_, sim)| *sim > 0.05)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NodeType;

    #[test]
    fn test_tantivy_index_and_semantic_search() {
        let index = KnowledgeSearchIndex::open_in_ram().unwrap();

        let n1 = KnowledgeNode::new(
            "k-1",
            "Relational Database Migrations",
            NodeType::Note,
            "SQL schema migrations and sqlite transaction management",
        );
        let n2 = KnowledgeNode::new(
            "k-2",
            "Audio Mixing Techniques",
            NodeType::Note,
            "Sound waveform equalization and multitrack master track",
        );

        index.index_node(&n1).unwrap();
        index.index_node(&n2).unwrap();

        // 1. Text BM25 search
        let bm25_res = index.search_text("sqlite", 5).unwrap();
        assert_eq!(bm25_res.len(), 1);
        assert_eq!(bm25_res[0].0, "k-1");

        // 2. Semantic vector search
        let candidates = vec![n1.clone(), n2.clone()];
        let sem_res = index.semantic_search("postgres postgresql database table", &candidates, 5);
        assert!(!sem_res.is_empty());
        assert_eq!(sem_res[0].0.id, "k-1");
    }
}
