#![forbid(unsafe_code)]
//! 8-Bit Quantized Local Embedding Inference & Vector Similarity Index (ADR-0029)
//!
//! Provides ultra-fast, 100% offline natural language semantic search:
//! - 64-dimensional dense semantic vector space
//! - 8-bit integer quantization (`[i8; 64]`) consuming 64 bytes per document
//! - Subword character n-gram hashing and normalized projection
//! - In-memory contiguous vector bank with SIMD-friendly integer dot products
//! - Sub-millisecond cosine similarity search (<50ms SLA, <300MB RAM)

use crate::models::{SemanticSearchResult, VectorEmbedding};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Standard vector dimensionality for the quantized embedding model
pub const VECTOR_DIM: usize = shared_ai_core::EMBEDDING_DIM;

/// In-memory vector index for semantic similarity search with disk persistence
pub struct VectorIndex {
    embeddings: RwLock<HashMap<PathBuf, VectorEmbedding>>,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            embeddings: RwLock::new(HashMap::new()),
        }
    }

    /// Indexes a document's textual content by generating and storing its dense embedding
    pub fn index_document<P: AsRef<Path>>(&self, path: P, text: &str) -> VectorEmbedding {
        let embedding = LocalEmbeddingModel::infer_embedding(path.as_ref(), text);
        if let Ok(mut lock) = self.embeddings.write() {
            lock.insert(path.as_ref().to_path_buf(), embedding.clone());
        }
        embedding
    }

    /// Inserts an already computed vector embedding
    pub fn insert_embedding(&self, embedding: VectorEmbedding) {
        if let Ok(mut lock) = self.embeddings.write() {
            lock.insert(embedding.doc_path.clone(), embedding);
        }
    }

    /// Removes an embedding by path
    pub fn remove_document<P: AsRef<Path>>(&self, path: P) {
        if let Ok(mut lock) = self.embeddings.write() {
            lock.remove(path.as_ref());
        }
    }

    /// Returns the total count of indexed vectors
    pub fn len(&self) -> usize {
        self.embeddings.read().map(|l| l.len()).unwrap_or(0)
    }

    /// Returns whether the vector index is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Save vector index to disk at specified path
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let lock = self
            .embeddings
            .read()
            .map_err(|e| std::io::Error::other(format!("Lock error: {e}")))?;
        let json = serde_json::to_string(&*lock)
            .map_err(|e| std::io::Error::other(format!("Serialization error: {e}")))?;
        std::fs::write(path.as_ref(), json)
    }

    /// Load vector index from disk at specified path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(path.as_ref())?;
        let map: HashMap<PathBuf, VectorEmbedding> = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::other(format!("Deserialization error: {e}")))?;
        Ok(Self {
            embeddings: RwLock::new(map),
        })
    }

    /// Performs nearest-neighbor cosine similarity search for a natural language query
    pub fn search(&self, query: &str, limit: usize) -> Vec<SemanticSearchResult> {
        let query_emb = LocalEmbeddingModel::infer_embedding(Path::new(""), query);
        let lock = match self.embeddings.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        let mut scored: Vec<SemanticSearchResult> = lock
            .values()
            .map(|doc_emb| {
                let score = compute_cosine_similarity(&query_emb, doc_emb);
                let file_name = doc_emb
                    .doc_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                let ext = doc_emb
                    .doc_path
                    .extension()
                    .map(|s| s.to_string_lossy().to_uppercase())
                    .unwrap_or_else(|| "FILE".to_string());

                SemanticSearchResult {
                    path: doc_emb.doc_path.clone(),
                    file_name,
                    score,
                    snippet: format!("Semantic match ({:.1}% confidence)", score * 100.0),
                    format: ext,
                }
            })
            .collect();

        // Sort descending by similarity score
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);
        scored
    }
}

// ---------------------------------------------------------------------------
// Local Quantized Embedding Model
// ---------------------------------------------------------------------------

pub struct LocalEmbeddingModel;

impl LocalEmbeddingModel {
    /// Generates a normalized dense quantized 8-bit embedding vector from text using concept-aware semantic subspaces
    pub fn infer_embedding(doc_path: &Path, text: &str) -> VectorEmbedding {
        let engine = shared_ai_core::ConceptEmbeddingEngine::new_standard();
        let dense = engine.embed_text(text);

        let mut quantized = Vec::with_capacity(VECTOR_DIM);
        for &val in &dense.vector {
            let q = (val * 127.0).round().clamp(-127.0, 127.0) as i8;
            quantized.push(q);
        }

        let mut q_sum_sq = 0.0f32;
        for &q in &quantized {
            q_sum_sq += (q as f32) * (q as f32);
        }
        let magnitude = q_sum_sq.sqrt();

        VectorEmbedding {
            doc_path: doc_path.to_path_buf(),
            vector: quantized,
            dimension: VECTOR_DIM,
            magnitude,
        }
    }
}

/// Computes the cosine similarity between two quantized vector embeddings
#[inline]
pub fn compute_cosine_similarity(a: &VectorEmbedding, b: &VectorEmbedding) -> f32 {
    if a.magnitude < 0.0001 || b.magnitude < 0.0001 {
        return 0.0;
    }

    let mut dot_product = 0i32;
    for (va, vb) in a.vector.iter().zip(b.vector.iter()) {
        dot_product += (*va as i32) * (*vb as i32);
    }

    let sim = (dot_product as f32) / (a.magnitude * b.magnitude);
    // Normalize to 0.0 .. 1.0 range
    ((sim + 1.0) / 2.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dimension_and_magnitude() {
        let emb = LocalEmbeddingModel::infer_embedding(
            Path::new("/test/doc.md"),
            "System architecture high performance offline database",
        );
        assert_eq!(emb.vector.len(), VECTOR_DIM);
        assert!(emb.magnitude > 0.0);
    }

    #[test]
    fn test_concept_synonym_similarity() {
        let doc_k8s = LocalEmbeddingModel::infer_embedding(
            Path::new("/manifest.yaml"),
            "k8s pod container deployment",
        );
        let doc_kube = LocalEmbeddingModel::infer_embedding(
            Path::new("/cluster.tf"),
            "kubernetes docker cluster",
        );
        let doc_db = LocalEmbeddingModel::infer_embedding(
            Path::new("/init.sql"),
            "postgres database query migration",
        );

        let sim_cloud = compute_cosine_similarity(&doc_k8s, &doc_kube);
        let sim_cross = compute_cosine_similarity(&doc_k8s, &doc_db);

        assert!(
            sim_cloud > sim_cross,
            "Cloud concepts (k8s/kubernetes) must have higher similarity: cloud={} cross={}",
            sim_cloud,
            sim_cross
        );
        assert!(sim_cloud > 0.80);
    }

    #[test]
    fn test_vector_index_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("vectors.json");

        let index = VectorIndex::new();
        index.index_document("/docs/k8s.md", "kubernetes pod container deployment");
        index.index_document("/docs/db.md", "postgres sql query database");

        assert_eq!(index.len(), 2);
        index.save_to_path(&index_path).unwrap();
        assert!(index_path.exists());

        let loaded = VectorIndex::load_from_path(&index_path).unwrap();
        assert_eq!(loaded.len(), 2);

        let hits = loaded.search("container deployment", 5);
        assert_eq!(hits[0].path, PathBuf::from("/docs/k8s.md"));
    }

    #[test]
    fn test_semantic_similarity_ranking() {
        let doc1 = LocalEmbeddingModel::infer_embedding(
            Path::new("/server.rs"),
            "database connection pool timeout failure and error handling",
        );
        let doc2 = LocalEmbeddingModel::infer_embedding(
            Path::new("/storage.rs"),
            "sqlite connection timeout error retry logic",
        );
        let doc3 = LocalEmbeddingModel::infer_embedding(
            Path::new("/recipe.txt"),
            "delicious chocolate cake baking recipe with butter and flour",
        );

        let sim_related = compute_cosine_similarity(&doc1, &doc2);
        let sim_unrelated = compute_cosine_similarity(&doc1, &doc3);

        assert!(
            sim_related > sim_unrelated,
            "Related documents should score higher: related={} unrelated={}",
            sim_related,
            sim_unrelated
        );
        assert!(sim_related - sim_unrelated > 0.10);
    }

    #[test]
    fn test_vector_index_search() {
        let index = VectorIndex::new();
        index.index_document(
            "/docs/kubernetes.md",
            "kubernetes pod crash loop backoff container failure",
        );
        index.index_document(
            "/docs/finance.md",
            "annual corporate revenue tax and balance sheet report",
        );

        assert_eq!(index.len(), 2);

        let hits = index.search("container crash failure", 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].path, PathBuf::from("/docs/kubernetes.md"));
        assert!(hits[0].score > hits[1].score);
    }
}
