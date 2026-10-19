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
pub const VECTOR_DIM: usize = 64;

/// In-memory vector index for semantic similarity search
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

    /// Indexes a document's textual content by generating and storing its 64-dim embedding
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
    /// Generates a 64-dimensional quantized 8-bit embedding vector from text
    pub fn infer_embedding(doc_path: &Path, text: &str) -> VectorEmbedding {
        let mut float_vec = [0.0f32; VECTOR_DIM];

        // Clean & tokenize text into lowercased tokens and character n-grams
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            return VectorEmbedding {
                doc_path: doc_path.to_path_buf(),
                vector: vec![0i8; VECTOR_DIM],
                dimension: VECTOR_DIM,
                magnitude: 0.0,
            };
        }

        // Token and subword n-gram projection with positional weighting
        for (pos, word) in words.iter().enumerate() {
            let pos_decay = 1.0 / (1.0 + (pos as f32 * 0.05));

            // Whole-word hash projection
            let w_hash = hash_token(word);
            let dim1 = (w_hash as usize) % VECTOR_DIM;
            let sign1 = if (w_hash >> 16) & 1 == 1 { 1.0 } else { -1.0 };
            float_vec[dim1] += sign1 * 2.0 * pos_decay;

            // Character trigram hashes for subword capture (e.g. "error", "err", "ror")
            let chars: Vec<char> = word.chars().collect();
            if chars.len() >= 3 {
                for i in 0..=chars.len() - 3 {
                    let trigram: String = chars[i..i + 3].iter().collect();
                    let t_hash = hash_token(&trigram);
                    let dim2 = (t_hash as usize) % VECTOR_DIM;
                    let sign2 = if (t_hash >> 16) & 1 == 1 { 1.0 } else { -1.0 };
                    float_vec[dim2] += sign2 * 0.8 * pos_decay;
                }
            }
        }

        // Compute L2 norm for unit normalization
        let mut sum_sq = 0.0f32;
        for &val in &float_vec {
            sum_sq += val * val;
        }
        let norm = sum_sq.sqrt();

        // Quantize to [-127, 127]
        let mut quantized = Vec::with_capacity(VECTOR_DIM);
        if norm > 0.0001 {
            for &val in &float_vec {
                let unit = val / norm;
                let q = (unit * 127.0).round().clamp(-127.0, 127.0) as i8;
                quantized.push(q);
            }
        } else {
            quantized.resize(VECTOR_DIM, 0);
        }

        // Recompute actual quantized magnitude for exact cosine similarity
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

/// Fast FNV-1a non-cryptographic token hasher
#[inline]
fn hash_token(token: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in token.as_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    hash
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
