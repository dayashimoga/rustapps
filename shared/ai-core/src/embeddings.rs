#![forbid(unsafe_code)]
//! Concept-Aware Dense Semantic Embedding Engine
//!
//! Produces 128-dimensional normalized dense vectors projecting semantic concepts
//! and synonyms into shared vector spaces with offline persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const EMBEDDING_DIM: usize = 128;

/// Dense semantic vector embedding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DenseEmbedding {
    pub vector: Vec<f32>,
}

impl DenseEmbedding {
    pub fn new(vector: Vec<f32>) -> Self {
        let mut emb = Self { vector };
        emb.normalize();
        emb
    }

    /// L2 normalization
    pub fn normalize(&mut self) {
        let norm_sq: f32 = self.vector.iter().map(|&x| x * x).sum();
        if norm_sq > 1e-12 {
            let inv_norm = 1.0 / norm_sq.sqrt();
            for val in &mut self.vector {
                *val *= inv_norm;
            }
        }
    }

    /// Compute cosine similarity between two normalized embeddings
    pub fn cosine_similarity(&self, other: &DenseEmbedding) -> f32 {
        if self.vector.len() != other.vector.len() {
            return 0.0;
        }
        let dot: f32 = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(&a, &b)| a * b)
            .sum();
        dot.clamp(-1.0, 1.0)
    }
}

/// Concept clusters mapping keywords to dedicated vector subspaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptEmbeddingEngine {
    concept_map: HashMap<String, usize>,
    concept_weights: Vec<f32>,
}

impl Default for ConceptEmbeddingEngine {
    fn default() -> Self {
        Self::new_standard()
    }
}

impl ConceptEmbeddingEngine {
    /// Initialize with standardized concept clusters
    pub fn new_standard() -> Self {
        let mut concept_map = HashMap::new();

        // 8 Primary concept domains mapped to distinct 16-dimensional slices of the 128-dim space:
        // Domain 0: Database & Query (indices 0..16)
        let db_words = [
            "sql",
            "postgres",
            "postgresql",
            "mysql",
            "sqlite",
            "database",
            "db",
            "query",
            "table",
            "schema",
            "record",
            "transaction",
            "migration",
            "column",
            "index",
        ];
        for word in db_words {
            concept_map.insert(word.to_string(), 0);
        }

        // Domain 1: Cloud & Containers (indices 16..32)
        let cloud_words = [
            "k8s",
            "kubernetes",
            "docker",
            "container",
            "pod",
            "cloud",
            "aws",
            "azure",
            "gcp",
            "cluster",
            "node",
            "helm",
            "deploy",
            "ingress",
            "service",
        ];
        for word in cloud_words {
            concept_map.insert(word.to_string(), 1);
        }

        // Domain 2: Failure, Error & Crash (indices 32..48)
        let failure_words = [
            "crash",
            "panic",
            "exception",
            "error",
            "failed",
            "failure",
            "timeout",
            "corrupt",
            "fault",
            "bug",
            "segfault",
            "abort",
            "unhandled",
            "fatal",
            "oom",
        ];
        for word in failure_words {
            concept_map.insert(word.to_string(), 2);
        }

        // Domain 3: Security, Keys & Auth (indices 48..64)
        let sec_words = [
            "auth",
            "login",
            "token",
            "credential",
            "secret",
            "password",
            "keyring",
            "cert",
            "tls",
            "ssh",
            "aes",
            "hash",
            "argon2",
            "audit",
            "vault",
        ];
        for word in sec_words {
            concept_map.insert(word.to_string(), 3);
        }

        // Domain 4: Media, Video & Audio Editing (indices 64..80)
        let media_words = [
            "audio",
            "video",
            "clip",
            "frame",
            "timeline",
            "render",
            "color",
            "lut",
            "grade",
            "sound",
            "eq",
            "mix",
            "track",
            "cut",
            "transition",
        ];
        for word in media_words {
            concept_map.insert(word.to_string(), 4);
        }

        // Domain 5: Logs, Metrics & Telemetry (indices 80..96)
        let log_words = [
            "log",
            "trace",
            "debug",
            "info",
            "warn",
            "tail",
            "metric",
            "telemetry",
            "drain",
            "line",
            "parse",
            "stream",
            "histogram",
            "timestamp",
            "event",
        ];
        for word in log_words {
            concept_map.insert(word.to_string(), 5);
        }

        // Domain 6: Language Translation & Speech (indices 96..112)
        let trans_words = [
            "trans",
            "translate",
            "speech",
            "voice",
            "subtitle",
            "tts",
            "asr",
            "whisper",
            "language",
            "lexicon",
            "grammar",
            "dialogue",
            "phrase",
            "turn",
            "script",
        ];
        for word in trans_words {
            concept_map.insert(word.to_string(), 6);
        }

        // Domain 7: Storage, Filesystem & Search (indices 112..128)
        let fs_words = [
            "file",
            "folder",
            "directory",
            "disk",
            "storage",
            "treemap",
            "duplicate",
            "size",
            "path",
            "indexer",
            "search",
            "mmap",
            "atomic",
            "write",
            "read",
        ];
        for word in fs_words {
            concept_map.insert(word.to_string(), 7);
        }

        let concept_weights = vec![1.0; 8];

        Self {
            concept_map,
            concept_weights,
        }
    }

    /// Embed a piece of text into a normalized 128-dimensional dense vector
    pub fn embed_text(&self, text: &str) -> DenseEmbedding {
        let mut vec = vec![0.0f32; EMBEDDING_DIM];

        for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
            let lower = word.to_lowercase();
            if lower.is_empty() {
                continue;
            }

            // 1. Concept projection: if word matches a domain, strongly project into domain slice
            if let Some(&domain_idx) = self.concept_map.get(&lower) {
                let start_idx = domain_idx * 16;
                let weight = self.concept_weights[domain_idx];
                for i in 0..16 {
                    vec[start_idx + i] += weight;
                }
            }

            // 2. Character n-gram hashing projection for general lexical affinity
            let mut h: u64 = 0xcbf29ce484222325;
            for &byte in lower.as_bytes() {
                h ^= byte as u64;
                h = h.wrapping_mul(0x100000001b3);
                let dim_idx = (h % (EMBEDDING_DIM as u64)) as usize;
                let sign = if (h & 0x80) != 0 { 1.0 } else { -1.0 };
                vec[dim_idx] += sign * 0.25;
            }
        }

        DenseEmbedding::new(vec)
    }

    /// Save concept configuration to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(format!("JSON serialize error: {}", e)))?;
        fs::write(path, json)
    }

    /// Load concept configuration from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::other(format!("JSON parse error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_synonym_similarity() {
        let engine = ConceptEmbeddingEngine::new_standard();

        let v_k8s = engine.embed_text("k8s container pod");
        let v_docker = engine.embed_text("docker kubernetes cluster");
        let v_db = engine.embed_text("postgres sql database");

        let sim_cloud = v_k8s.cosine_similarity(&v_docker);
        let sim_cross = v_k8s.cosine_similarity(&v_db);

        // Synonyms in same domain must have significantly higher similarity than cross-domain
        assert!(
            sim_cloud > 0.85,
            "Expected high cloud similarity, got {}",
            sim_cloud
        );
        assert!(
            sim_cross < 0.40,
            "Expected low cross similarity, got {}",
            sim_cross
        );
    }

    #[test]
    fn test_save_and_load_file() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("concept_engine.json");

        let engine = ConceptEmbeddingEngine::new_standard();
        engine.save_to_file(&file_path).unwrap();
        assert!(file_path.exists());

        let loaded = ConceptEmbeddingEngine::load_from_file(&file_path).unwrap();
        assert_eq!(loaded.concept_map.len(), engine.concept_map.len());
    }
}
