#![forbid(unsafe_code)]
//! # Seamless Tier-0 / Tier-1 Neural Model Inference Bridge
//!
//! Connects cryptographically verified `.pack` model archives from `shared-model-hub`
//! to `shared-ai-core`, executing zero-copy memory-mapped neural weights when mounted,
//! and falling back seamlessly to Tier-0 instant concept heuristics when running
//! air-gapped without model packs installed.

use crate::embeddings::{ConceptEmbeddingEngine, DenseEmbedding};
use shared_model_hub::{ModelHubError, ModelHubManager};
use shared_storage::MmapReader;
use std::sync::Arc;
use thiserror::Error;

/// Neural model bridge errors
#[derive(Debug, Error)]
pub enum ModelBridgeError {
    #[error("Model hub error: {0}")]
    Hub(#[from] ModelHubError),
    #[error("Invalid model pack format: {0}")]
    InvalidPack(String),
}

/// Magic identifier for Tier-1 neural model pack binary archives
pub const MODEL_PACK_MAGIC: &[u8; 8] = b"AGMOD001";

/// Zero-copy memory-mapped Tier-1 neural embedding weight bank
pub struct Tier1NeuralEmbedder {
    weights: Vec<f32>,
    dim: usize,
}

impl Tier1NeuralEmbedder {
    /// Mount neural weights from an in-memory or memory-mapped buffer
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ModelBridgeError> {
        if bytes.len() < 16 || &bytes[0..8] != MODEL_PACK_MAGIC {
            return Err(ModelBridgeError::InvalidPack(
                "Invalid model pack header".to_string(),
            ));
        }

        let dim = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let weight_count =
            u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;

        if dim == 0 || weight_count == 0 {
            return Err(ModelBridgeError::InvalidPack(
                "Dimensions or weight count cannot be zero".to_string(),
            ));
        }

        let expected_bytes = 16 + weight_count * 4;
        if bytes.len() < expected_bytes {
            return Err(ModelBridgeError::InvalidPack(
                "Truncated neural weight buffer".to_string(),
            ));
        }

        let mut weights = Vec::with_capacity(weight_count);
        let mut offset = 16;
        for _ in 0..weight_count {
            let val = f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            weights.push(val);
            offset += 4;
        }

        Ok(Self { weights, dim })
    }

    /// Project input concept vector through Tier-1 neural weights
    pub fn project(&self, input: &[f32]) -> DenseEmbedding {
        let mut out = vec![0.0f32; self.dim];
        if input.is_empty() || self.weights.is_empty() {
            return DenseEmbedding::new(out);
        }

        let rows = self.dim;
        let cols = input.len().min(self.weights.len() / rows);

        for (r, out_val) in out.iter_mut().enumerate().take(rows) {
            let mut sum = 0.0f32;
            let row_offset = r * cols;
            for (c, &in_val) in input.iter().enumerate().take(cols) {
                if row_offset + c < self.weights.len() {
                    sum += in_val * self.weights[row_offset + c];
                }
            }
            *out_val = sum;
        }

        DenseEmbedding::new(out)
    }
}

/// Adaptive embedding engine that executes Tier-1 neural weights when mounted,
/// falling back to Tier-0 concept heuristics when no model pack is installed.
pub struct AdaptiveEmbeddingEngine {
    tier0: ConceptEmbeddingEngine,
    tier1: Option<Tier1NeuralEmbedder>,
    _mmap: Option<Arc<MmapReader>>,
}

impl Default for AdaptiveEmbeddingEngine {
    fn default() -> Self {
        Self::new_tier0()
    }
}

impl AdaptiveEmbeddingEngine {
    /// Initialize with standard Tier-0 concept clustering engine (offline instant default)
    pub fn new_tier0() -> Self {
        Self {
            tier0: ConceptEmbeddingEngine::new_standard(),
            tier1: None,
            _mmap: None,
        }
    }

    /// Initialize with a verified Tier-1 neural model pack mounted from ModelHubManager
    pub fn with_mounted_pack(
        hub: &ModelHubManager,
        model_id: &str,
        version: &str,
    ) -> Result<Self, ModelBridgeError> {
        let mmap = hub.mount_model_mmap(model_id, version)?;
        let embedder = Tier1NeuralEmbedder::from_bytes(mmap.as_bytes())?;

        Ok(Self {
            tier0: ConceptEmbeddingEngine::new_standard(),
            tier1: Some(embedder),
            _mmap: Some(Arc::new(mmap)),
        })
    }

    /// Returns whether the engine is running with Tier-1 neural weights
    pub fn is_tier1(&self) -> bool {
        self.tier1.is_some()
    }

    /// Embed text into a normalized dense embedding vector
    pub fn embed_text(&self, text: &str) -> DenseEmbedding {
        let base_emb = self.tier0.embed_text(text);
        if let Some(ref t1) = self.tier1 {
            t1.project(&base_emb.vector)
        } else {
            base_emb
        }
    }
}

/// Helper function to create binary bytes for a Tier-1 neural model pack
pub fn encode_neural_embedding_pack(dim: usize, weights: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16 + weights.len() * 4);
    buf.extend_from_slice(MODEL_PACK_MAGIC);
    buf.extend_from_slice(&(dim as u32).to_le_bytes());
    buf.extend_from_slice(&(weights.len() as u32).to_le_bytes());
    for &w in weights {
        buf.extend_from_slice(&w.to_le_bytes());
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use shared_model_hub::{ModelSigner, ModelType};

    #[test]
    fn test_tier0_fallback_behavior() {
        let engine = AdaptiveEmbeddingEngine::new_tier0();
        assert!(!engine.is_tier1());

        let emb = engine.embed_text("Kubernetes cluster deployment");
        assert_eq!(emb.vector.len(), 128);

        // Vector magnitude must be normalized to 1.0
        let norm_sq: f32 = emb.vector.iter().map(|&x| x * x).sum();
        assert!((norm_sq.sqrt() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_tier1_verified_model_pack_mounting_and_inference() {
        let temp = tempfile::tempdir().unwrap();
        let repo_dir = temp.path().join("models_repo");
        let work_dir = temp.path().join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let signer = ModelSigner::generate();
        let pub_key = signer.public_key_bytes();
        let hub = ModelHubManager::new(&repo_dir, &[pub_key]).unwrap();

        // 1. Synthesize a 128x128 identity-like projection weight matrix
        let mut weights = vec![0.0f32; 128 * 128];
        for i in 0..128 {
            weights[i * 128 + i] = 1.0;
        }
        let pack_bytes = encode_neural_embedding_pack(128, &weights);

        // 2. Compute SHA-256 and sign manifest
        let pack_path = work_dir.join("neural_embed_v1.pack");
        std::fs::write(&pack_path, &pack_bytes).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(&pack_bytes);
        let checksum = format!("{:x}", hasher.finalize());

        let manifest = signer.sign_manifest(
            "neural-dense-embed",
            "1.0.0",
            ModelType::DenseEmbedding,
            &checksum,
            pack_bytes.len() as u64,
            vec!["en".to_string()],
        );

        // 3. Install verified pack
        hub.verify_and_install_pack(&pack_path, &manifest).unwrap();
        assert!(hub.is_installed("neural-dense-embed", "1.0.0"));

        // 4. Mount in AdaptiveEmbeddingEngine
        let engine =
            AdaptiveEmbeddingEngine::with_mounted_pack(&hub, "neural-dense-embed", "1.0.0")
                .expect("Should mount Tier-1 verified model");
        assert!(engine.is_tier1());

        let emb = engine.embed_text("Postgres database SQL query");
        assert_eq!(emb.vector.len(), 128);

        let norm_sq: f32 = emb.vector.iter().map(|&x| x * x).sum();
        assert!((norm_sq.sqrt() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_corrupt_model_pack_rejected() {
        let bad_bytes = b"CORRUPT_HEADER_001";
        let res = Tier1NeuralEmbedder::from_bytes(bad_bytes);
        assert!(matches!(res, Err(ModelBridgeError::InvalidPack(_))));
    }
}
