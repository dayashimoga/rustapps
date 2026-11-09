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
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Standard vector dimensionality for the quantized embedding model
pub const VECTOR_DIM: usize = shared_ai_core::EMBEDDING_DIM;

/// Magic header identifier for high-performance binary vector index files (ADR-0029)
pub const AGVEC_MAGIC: &[u8; 8] = b"AGVEC001";

#[derive(Copy, Clone, PartialEq)]
struct ScoredCandidate<'a> {
    score: f32,
    doc_path: &'a PathBuf,
}

impl<'a> Eq for ScoredCandidate<'a> {}

impl<'a> Ord for ScoredCandidate<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering so BinaryHeap acts as a min-heap (smallest score at top)
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl<'a> PartialOrd for ScoredCandidate<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

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

    /// Save vector index to disk at specified path using AGVEC001 high-performance binary format
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let lock = self
            .embeddings
            .read()
            .map_err(|e| std::io::Error::other(format!("Lock error: {e}")))?;

        let mut buffer = Vec::with_capacity(16 + lock.len() * 128);
        buffer.extend_from_slice(AGVEC_MAGIC);
        buffer.extend_from_slice(&(lock.len() as u32).to_le_bytes());

        for (doc_path, emb) in lock.iter() {
            let path_str = doc_path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            buffer.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
            buffer.extend_from_slice(path_bytes);
            buffer.extend_from_slice(&(emb.dimension as u16).to_le_bytes());
            buffer.extend_from_slice(&emb.magnitude.to_le_bytes());
            for &val in &emb.vector {
                buffer.push(val as u8);
            }
        }

        std::fs::write(path.as_ref(), buffer)
    }

    /// Load vector index from disk at specified path. Supports AGVEC001 binary format with JSON fallback.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let data = std::fs::read(path.as_ref())?;

        // Fast-path: AGVEC001 binary format
        if data.starts_with(AGVEC_MAGIC) {
            if data.len() < 12 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Truncated AGVEC001 binary vector file",
                ));
            }
            let total_count = u32::from_le_bytes(
                data[8..12]
                    .try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ) as usize;

            let mut offset = 12;
            let mut map = HashMap::with_capacity(total_count);

            for _ in 0..total_count {
                if offset + 4 > data.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unexpected EOF reading path length",
                    ));
                }
                let path_len = u32::from_le_bytes(
                    data[offset..offset + 4]
                        .try_into()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                ) as usize;
                offset += 4;

                if offset + path_len + 6 > data.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unexpected EOF reading path and metadata",
                    ));
                }
                let path_str = std::str::from_utf8(&data[offset..offset + path_len])
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                offset += path_len;

                let dim = u16::from_le_bytes(
                    data[offset..offset + 2]
                        .try_into()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                ) as usize;
                offset += 2;

                let magnitude = f32::from_le_bytes(
                    data[offset..offset + 4]
                        .try_into()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                );
                offset += 4;

                if offset + dim > data.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unexpected EOF reading vector bytes",
                    ));
                }

                let vector: Vec<i8> = data[offset..offset + dim]
                    .iter()
                    .map(|&b| b as i8)
                    .collect();
                offset += dim;

                let doc_path = PathBuf::from(path_str);
                map.insert(
                    doc_path.clone(),
                    VectorEmbedding {
                        doc_path,
                        vector,
                        dimension: dim,
                        magnitude,
                    },
                );
            }

            return Ok(Self {
                embeddings: RwLock::new(map),
            });
        }

        // Backward-compatible JSON deserialization fallback
        let text = std::str::from_utf8(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let map: HashMap<PathBuf, VectorEmbedding> = serde_json::from_str(text)
            .map_err(|e| std::io::Error::other(format!("Deserialization error: {e}")))?;
        Ok(Self {
            embeddings: RwLock::new(map),
        })
    }

    /// Performs nearest-neighbor cosine similarity search for a natural language query with bounded memory (O(limit))
    pub fn search(&self, query: &str, limit: usize) -> Vec<SemanticSearchResult> {
        if limit == 0 {
            return Vec::new();
        }

        let query_emb = LocalEmbeddingModel::infer_embedding(Path::new(""), query);
        let lock = match self.embeddings.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        // Bounded min-heap to keep only the top-k highest scoring candidates without allocating strings for non-winners
        let mut min_heap: BinaryHeap<ScoredCandidate> = BinaryHeap::with_capacity(limit + 1);

        for doc_emb in lock.values() {
            let score = compute_cosine_similarity(&query_emb, doc_emb);
            if min_heap.len() < limit {
                min_heap.push(ScoredCandidate {
                    score,
                    doc_path: &doc_emb.doc_path,
                });
            } else if let Some(min_top) = min_heap.peek() {
                if score > min_top.score {
                    min_heap.pop();
                    min_heap.push(ScoredCandidate {
                        score,
                        doc_path: &doc_emb.doc_path,
                    });
                }
            }
        }

        // Sort descending by similarity score
        let mut top_candidates = min_heap.into_vec();
        top_candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Construct SemanticSearchResult only for the winning top-k candidates
        top_candidates
            .into_iter()
            .map(|cand| {
                let file_name = cand
                    .doc_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                let ext = cand
                    .doc_path
                    .extension()
                    .map(|s| s.to_string_lossy().to_uppercase())
                    .unwrap_or_else(|| "FILE".to_string());

                SemanticSearchResult {
                    path: cand.doc_path.clone(),
                    file_name,
                    score: cand.score,
                    snippet: format!("Semantic match ({:.1}% confidence)", cand.score * 100.0),
                    format: ext,
                }
            })
            .collect()
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
        let index_path = temp_dir.path().join("vectors.bin");

        let index = VectorIndex::new();
        index.index_document("/docs/k8s.md", "kubernetes pod container deployment");
        index.index_document("/docs/db.md", "postgres sql query database");

        assert_eq!(index.len(), 2);
        index.save_to_path(&index_path).unwrap();
        assert!(index_path.exists());

        // Verify AGVEC001 magic header
        let raw = std::fs::read(&index_path).unwrap();
        assert!(raw.starts_with(AGVEC_MAGIC));

        let loaded = VectorIndex::load_from_path(&index_path).unwrap();
        assert_eq!(loaded.len(), 2);

        let hits = loaded.search("container deployment", 5);
        assert_eq!(hits[0].path, PathBuf::from("/docs/k8s.md"));
    }

    #[test]
    fn test_vector_index_backward_compatible_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("legacy_vectors.json");

        // Create legacy JSON map
        let mut map = HashMap::new();
        let emb = LocalEmbeddingModel::infer_embedding(
            Path::new("/legacy/doc.txt"),
            "legacy search index content in json",
        );
        map.insert(PathBuf::from("/legacy/doc.txt"), emb);

        let json_text = serde_json::to_string(&map).unwrap();
        std::fs::write(&json_path, json_text).unwrap();

        // Load legacy JSON using load_from_path
        let loaded = VectorIndex::load_from_path(&json_path).unwrap();
        assert_eq!(loaded.len(), 1);

        let hits = loaded.search("legacy search index", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, PathBuf::from("/legacy/doc.txt"));
    }

    #[test]
    fn test_bounded_search_heap() {
        let index = VectorIndex::new();
        for i in 0..50 {
            index.index_document(
                format!("/docs/file_{}.txt", i),
                &format!("document number {} about systems engineering and rust performance", i),
            );
        }

        // Search with limit 5 should return exactly 5 items
        let hits = index.search("rust performance systems", 5);
        assert_eq!(hits.len(), 5);

        // Verify descending order
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score);
        }

        // Limit 0 should return empty
        let empty_hits = index.search("rust performance", 0);
        assert!(empty_hits.is_empty());
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
