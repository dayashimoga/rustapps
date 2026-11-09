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
use shared_storage::MmapReader;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Standard vector dimensionality for the quantized embedding model
pub const VECTOR_DIM: usize = shared_ai_core::EMBEDDING_DIM;

/// Magic header identifier for high-performance binary vector index files (ADR-0029)
pub const AGVEC_MAGIC: &[u8; 8] = b"AGVEC001";

#[derive(Clone, PartialEq)]
struct ScoredCandidate {
    score: f32,
    doc_path: PathBuf,
}

impl Eq for ScoredCandidate {}

impl Ord for ScoredCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering so BinaryHeap acts as a min-heap (smallest score at top)
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for ScoredCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Zero-copy high-throughput vector index for semantic similarity search with MmapReader support
pub struct VectorIndex {
    mem_embeddings: RwLock<HashMap<PathBuf, VectorEmbedding>>,
    removed_paths: RwLock<HashSet<PathBuf>>,
    mmap_reader: Option<MmapReader>,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            mem_embeddings: RwLock::new(HashMap::new()),
            removed_paths: RwLock::new(HashSet::new()),
            mmap_reader: None,
        }
    }

    /// Indexes a document's textual content by generating and storing its dense embedding
    pub fn index_document<P: AsRef<Path>>(&self, path: P, text: &str) -> VectorEmbedding {
        let embedding = LocalEmbeddingModel::infer_embedding(path.as_ref(), text);
        let p = path.as_ref().to_path_buf();
        if let Ok(mut mem) = self.mem_embeddings.write() {
            mem.insert(p.clone(), embedding.clone());
        }
        if let Ok(mut removed) = self.removed_paths.write() {
            removed.remove(&p);
        }
        embedding
    }

    /// Inserts an already computed vector embedding
    pub fn insert_embedding(&self, embedding: VectorEmbedding) {
        let p = embedding.doc_path.clone();
        if let Ok(mut mem) = self.mem_embeddings.write() {
            mem.insert(p.clone(), embedding);
        }
        if let Ok(mut removed) = self.removed_paths.write() {
            removed.remove(&p);
        }
    }

    /// Removes an embedding by path
    pub fn remove_document<P: AsRef<Path>>(&self, path: P) {
        let p = path.as_ref().to_path_buf();
        if let Ok(mut mem) = self.mem_embeddings.write() {
            mem.remove(&p);
        }
        if let Ok(mut removed) = self.removed_paths.write() {
            removed.insert(p);
        }
    }

    /// Returns the total count of indexed vectors
    pub fn len(&self) -> usize {
        let mem_lock = self
            .mem_embeddings
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let removed_lock = self.removed_paths.read().unwrap_or_else(|e| e.into_inner());

        let mut total = mem_lock.len();
        if let Some(ref reader) = self.mmap_reader {
            let data = reader.as_bytes();
            if data.len() >= 12 && &data[0..8] == AGVEC_MAGIC {
                let total_count =
                    u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
                let mut offset = 12;

                for _ in 0..total_count {
                    if offset + 4 > data.len() {
                        break;
                    }
                    let path_len = u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]) as usize;
                    offset += 4;

                    if offset + path_len + 6 > data.len() {
                        break;
                    }
                    let path_bytes = &data[offset..offset + path_len];
                    offset += path_len;

                    let dim = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 6 + dim;

                    if let Ok(path_str) = std::str::from_utf8(path_bytes) {
                        let path = Path::new(path_str);
                        if !mem_lock.contains_key(path) && !removed_lock.contains(path) {
                            total += 1;
                        }
                    }
                }
            }
        }
        total
    }

    /// Returns whether the vector index is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Save vector index to disk at specified path using AGVEC001 high-performance binary format
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mem_lock = self
            .mem_embeddings
            .read()
            .map_err(|e| std::io::Error::other(format!("Lock error: {e}")))?;
        let removed_lock = self
            .removed_paths
            .read()
            .map_err(|e| std::io::Error::other(format!("Lock error: {e}")))?;

        let mut entries: Vec<(PathBuf, usize, f32, Vec<u8>)> = Vec::new();

        // 1. Existing mmap records not shadowed or removed
        if let Some(ref reader) = self.mmap_reader {
            let data = reader.as_bytes();
            if data.len() >= 12 && &data[0..8] == AGVEC_MAGIC {
                let total_count =
                    u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
                let mut offset = 12;

                for _ in 0..total_count {
                    if offset + 4 > data.len() {
                        break;
                    }
                    let path_len = u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]) as usize;
                    offset += 4;

                    if offset + path_len + 6 > data.len() {
                        break;
                    }
                    let path_bytes = &data[offset..offset + path_len];
                    offset += path_len;

                    let dim = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;

                    let magnitude = f32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    offset += 4;

                    if offset + dim > data.len() {
                        break;
                    }
                    let vec_bytes = &data[offset..offset + dim];
                    offset += dim;

                    if let Ok(path_str) = std::str::from_utf8(path_bytes) {
                        let doc_path = PathBuf::from(path_str);
                        if !mem_lock.contains_key(&doc_path) && !removed_lock.contains(&doc_path) {
                            entries.push((doc_path, dim, magnitude, vec_bytes.to_vec()));
                        }
                    }
                }
            }
        }

        // 2. In-memory entries
        for (doc_path, emb) in mem_lock.iter() {
            if !removed_lock.contains(doc_path) {
                let raw_vec: Vec<u8> = emb.vector.iter().map(|&v| v as u8).collect();
                entries.push((doc_path.clone(), emb.dimension, emb.magnitude, raw_vec));
            }
        }

        let mut buffer = Vec::with_capacity(16 + entries.len() * 128);
        buffer.extend_from_slice(AGVEC_MAGIC);
        buffer.extend_from_slice(&(entries.len() as u32).to_le_bytes());

        for (doc_path, dim, magnitude, vec_bytes) in entries {
            let path_str = doc_path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            buffer.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
            buffer.extend_from_slice(path_bytes);
            buffer.extend_from_slice(&(dim as u16).to_le_bytes());
            buffer.extend_from_slice(&magnitude.to_le_bytes());
            buffer.extend_from_slice(&vec_bytes);
        }

        std::fs::write(path.as_ref(), buffer)
    }

    /// Load vector index from disk at specified path. Supports AGVEC001 binary format with zero-copy MmapReader and JSON fallback.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Vector index file not found: {}", p.display()),
            ));
        }

        let reader = MmapReader::open(p)
            .map_err(|e| std::io::Error::other(format!("Mmap open error: {e}")))?;
        let bytes = reader.as_bytes();

        if bytes.starts_with(AGVEC_MAGIC) {
            if bytes.len() < 12 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Truncated AGVEC001 binary vector file",
                ));
            }
            let _total_count = u32::from_le_bytes(
                bytes[8..12]
                    .try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ) as usize;

            return Ok(Self {
                mem_embeddings: RwLock::new(HashMap::new()),
                removed_paths: RwLock::new(HashSet::new()),
                mmap_reader: Some(reader),
            });
        }

        // Backward-compatible JSON deserialization fallback
        let text = std::str::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let map: HashMap<PathBuf, VectorEmbedding> = serde_json::from_str(text)
            .map_err(|e| std::io::Error::other(format!("Deserialization error: {e}")))?;
        Ok(Self {
            mem_embeddings: RwLock::new(map),
            removed_paths: RwLock::new(HashSet::new()),
            mmap_reader: None,
        })
    }

    /// Performs nearest-neighbor cosine similarity search for a natural language query with bounded memory (O(limit))
    pub fn search(&self, query: &str, limit: usize) -> Vec<SemanticSearchResult> {
        if limit == 0 {
            return Vec::new();
        }

        let query_emb = LocalEmbeddingModel::infer_embedding(Path::new(""), query);
        let mem_lock = self
            .mem_embeddings
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let removed_lock = self.removed_paths.read().unwrap_or_else(|e| e.into_inner());

        let mut min_heap: BinaryHeap<ScoredCandidate> = BinaryHeap::with_capacity(limit + 1);

        // 1. Scan in-memory embeddings
        for (doc_path, doc_emb) in mem_lock.iter() {
            if removed_lock.contains(doc_path) {
                continue;
            }
            let score = compute_cosine_similarity(&query_emb, doc_emb);
            if min_heap.len() < limit {
                min_heap.push(ScoredCandidate {
                    score,
                    doc_path: doc_path.clone(),
                });
            } else if let Some(min_top) = min_heap.peek() {
                if score > min_top.score {
                    min_heap.pop();
                    min_heap.push(ScoredCandidate {
                        score,
                        doc_path: doc_path.clone(),
                    });
                }
            }
        }

        // 2. Zero-copy linear scan over memory-mapped AGVEC001 records
        if let Some(ref reader) = self.mmap_reader {
            let data = reader.as_bytes();
            if data.len() >= 12 && &data[0..8] == AGVEC_MAGIC {
                let total_count =
                    u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
                let mut offset = 12;

                for _ in 0..total_count {
                    if offset + 4 > data.len() {
                        break;
                    }
                    let path_len = u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]) as usize;
                    offset += 4;

                    if offset + path_len + 6 > data.len() {
                        break;
                    }
                    let path_bytes = &data[offset..offset + path_len];
                    offset += path_len;

                    let dim = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;

                    let magnitude = f32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    offset += 4;

                    if offset + dim > data.len() {
                        break;
                    }
                    let vec_bytes = &data[offset..offset + dim];
                    offset += dim;

                    // Skip if shadowed by in-memory update or deletion
                    if let Ok(path_str) = std::str::from_utf8(path_bytes) {
                        let path = Path::new(path_str);
                        if mem_lock.contains_key(path) || removed_lock.contains(path) {
                            continue;
                        }

                        let score = compute_cosine_similarity_raw(
                            &query_emb.vector,
                            query_emb.magnitude,
                            vec_bytes,
                            magnitude,
                        );

                        if min_heap.len() < limit {
                            min_heap.push(ScoredCandidate {
                                score,
                                doc_path: path.to_path_buf(),
                            });
                        } else if let Some(min_top) = min_heap.peek() {
                            if score > min_top.score {
                                min_heap.pop();
                                min_heap.push(ScoredCandidate {
                                    score,
                                    doc_path: path.to_path_buf(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort descending by similarity score
        let mut sorted = min_heap.into_vec();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        sorted
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
                    path: cand.doc_path,
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

/// Computes the cosine similarity directly against a raw memory-mapped byte slice without allocation
#[inline]
pub fn compute_cosine_similarity_raw(
    query_vector: &[i8],
    query_magnitude: f32,
    doc_bytes: &[u8],
    doc_magnitude: f32,
) -> f32 {
    if query_magnitude < 0.0001 || doc_magnitude < 0.0001 {
        return 0.0;
    }

    let mut dot_product = 0i32;
    for (va, vb) in query_vector.iter().zip(doc_bytes.iter()) {
        dot_product += (*va as i32) * (*vb as i8 as i32);
    }

    let sim = (dot_product as f32) / (query_magnitude * doc_magnitude);
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
                &format!(
                    "document number {} about systems engineering and rust performance",
                    i
                ),
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
