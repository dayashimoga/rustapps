#![deny(unsafe_code)]
//! Offline Personal Knowledge Graph & Research Studio
//!
//! Provides zero-network-egress, 100% offline bidirectional citation graph traversal,
//! automatic wiki-link extraction, Tantivy full-text retrieval, and dense semantic vector search.

pub mod commands;
pub mod document_parser;
#[allow(unsafe_code)]
pub mod ffi;
pub mod graph_store;
pub mod knowledge_index;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use document_parser::{DocumentParser, ParsedDocument, WikiLink};
pub use graph_store::{GraphError, GraphStore};
pub use knowledge_index::{IndexError, KnowledgeSearchIndex};
pub use models::{
    CitationRecord, GraphMetrics, GraphVisualizationData, KnowledgeEdge, KnowledgeNode, NodeType,
    RelationType, SearchResult,
};
pub use service::{KnowledgeService, ServiceError};
