#![forbid(unsafe_code)]
//! Core Data Models for Offline Personal Knowledge Graph & Research Studio

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of a knowledge node in the personal research graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum NodeType {
    /// Markdown research note or synthetic summary
    #[default]
    Note,
    /// Ingested paper, PDF, book, or source document
    Document,
    /// Conceptual entity, topic, or subject abstraction
    Concept,
    /// Academic or bibliographic citation record
    Citation,
    /// Highlighted passage or excerpt with exact page / offset
    Excerpt,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Note => "note",
            NodeType::Document => "document",
            NodeType::Concept => "concept",
            NodeType::Citation => "citation",
            NodeType::Excerpt => "excerpt",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "document" => NodeType::Document,
            "concept" => NodeType::Concept,
            "citation" => NodeType::Citation,
            "excerpt" => NodeType::Excerpt,
            _ => NodeType::Note,
        }
    }
}

/// A node in the personal knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeNode {
    pub id: String,
    pub title: String,
    pub node_type: NodeType,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl KnowledgeNode {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        node_type: NodeType,
        content: impl Into<String>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: id.into(),
            title: title.into(),
            node_type,
            content: content.into(),
            tags: Vec::new(),
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Semantic relationship type between two knowledge nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RelationType {
    /// Source cites target literature / source
    Cites,
    /// Bidirectional or conceptual association
    #[default]
    RelatedTo,
    /// Hierarchical taxonomy (source is parent of target)
    ParentOf,
    /// Textual or lexical reference
    Mentions,
    /// Derived theory or extracted finding
    DerivesFrom,
    /// Scholarly contradiction or critique
    Contradicts,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::Cites => "cites",
            RelationType::RelatedTo => "related_to",
            RelationType::ParentOf => "parent_of",
            RelationType::Mentions => "mentions",
            RelationType::DerivesFrom => "derives_from",
            RelationType::Contradicts => "contradicts",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "cites" => RelationType::Cites,
            "parent_of" => RelationType::ParentOf,
            "mentions" => RelationType::Mentions,
            "derives_from" => RelationType::DerivesFrom,
            "contradicts" => RelationType::Contradicts,
            _ => RelationType::RelatedTo,
        }
    }
}

/// A directed semantic link between two nodes in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: RelationType,
    pub weight: f64,
    pub label: Option<String>,
    pub created_at: i64,
}

impl KnowledgeEdge {
    pub fn new(
        id: impl Into<String>,
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        relation_type: RelationType,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: id.into(),
            source_id: source_id.into(),
            target_id: target_id.into(),
            relation_type,
            weight: 1.0,
            label: None,
            created_at: now,
        }
    }
}

/// Bibliographic citation metadata linked to a Citation node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CitationRecord {
    pub id: String,
    pub node_id: String,
    pub authors: Vec<String>,
    pub publication_year: Option<u32>,
    pub source_venue: Option<String>,
    pub doi: Option<String>,
    pub bibtex: Option<String>,
}

/// Statistical metrics describing the graph topology
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphMetrics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub node_type_counts: HashMap<String, usize>,
    pub tag_counts: HashMap<String, usize>,
    pub density: f64,
}

/// Graph payload returned to the UI for visual rendering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphVisualizationData {
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
}

/// Full-text or semantic search result item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub node: KnowledgeNode,
    pub score: f32,
    pub highlights: Vec<String>,
}
