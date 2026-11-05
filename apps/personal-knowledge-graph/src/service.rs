#![forbid(unsafe_code)]
//! Knowledge Graph Service Orchestrator

use crate::document_parser::DocumentParser;
use crate::graph_store::{GraphError, GraphStore};
use crate::knowledge_index::{IndexError, KnowledgeSearchIndex};
use crate::models::{
    CitationRecord, GraphMetrics, GraphVisualizationData, KnowledgeEdge, KnowledgeNode, NodeType,
    RelationType, SearchResult,
};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Graph store error: {0}")]
    Graph(#[from] GraphError),
    #[error("Index error: {0}")]
    Index(#[from] IndexError),
    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

/// Primary orchestrator for personal knowledge graph operations
pub struct KnowledgeService {
    store: Arc<GraphStore>,
    index: Arc<KnowledgeSearchIndex>,
}

impl KnowledgeService {
    /// Opens an in-memory knowledge graph instance (for tests & scratch workflows)
    pub fn open_in_memory() -> Result<Self, ServiceError> {
        let store = Arc::new(GraphStore::open_in_memory()?);
        let index = Arc::new(KnowledgeSearchIndex::open_in_ram()?);
        Ok(Self { store, index })
    }

    /// Opens or creates persistent knowledge database and index directories
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, ServiceError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir)
            .map_err(|e| IndexError::Tantivy(tantivy::TantivyError::IoError(Arc::new(e))))?;

        let db_path = dir.join("knowledge_graph.db");
        let store = Arc::new(GraphStore::open(db_path)?);

        let index_dir = dir.join("tantivy_index");
        let index = Arc::new(KnowledgeSearchIndex::open_or_create(index_dir)?);

        // Populate search index from existing store if index is fresh
        let nodes = store.get_all_nodes()?;
        for node in nodes {
            let _ = index.index_node(&node);
        }

        Ok(Self { store, index })
    }

    /// Create or update a research note, auto-extracting wiki-links, tags, and citations
    pub fn save_note(
        &self,
        id_opt: Option<&str>,
        title: &str,
        content: &str,
        node_type: Option<NodeType>,
        custom_tags: Option<Vec<String>>,
    ) -> Result<KnowledgeNode, ServiceError> {
        let parsed = DocumentParser::parse(content);

        let mut tags = custom_tags.unwrap_or_default();
        for t in parsed.tags {
            if !tags.contains(&t) {
                tags.push(t);
            }
        }

        let id = match id_opt {
            Some(existing_id) => existing_id.to_string(),
            None => Uuid::new_v4().to_string(),
        };

        let node_type = node_type.unwrap_or(NodeType::Note);
        let mut node = KnowledgeNode::new(&id, title, node_type, content);
        node.tags = tags;

        self.store.insert_node(&node)?;
        self.index.index_node(&node)?;

        // Auto-link wiki-links `[[Target Title]]`
        for link in parsed.wiki_links {
            let target_node_id =
                if let Some(target) = self.store.get_node_by_title(&link.target_title)? {
                    target.id
                } else {
                    // Auto-create concept stub
                    let stub_id = Uuid::new_v4().to_string();
                    let stub_node =
                        KnowledgeNode::new(&stub_id, &link.target_title, NodeType::Concept, "");
                    self.store.insert_node(&stub_node)?;
                    self.index.index_node(&stub_node)?;
                    stub_id
                };

            let edge_id = format!("{}-{}-mentions", id, target_node_id);
            let edge = KnowledgeEdge {
                id: edge_id,
                source_id: id.clone(),
                target_id: target_node_id,
                relation_type: RelationType::Mentions,
                weight: 1.0,
                label: link.alias,
                created_at: node.created_at,
            };
            let _ = self.store.insert_edge(&edge);
        }

        Ok(node)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<KnowledgeNode>, ServiceError> {
        Ok(self.store.get_node(id)?)
    }

    pub fn delete_node(&self, id: &str) -> Result<bool, ServiceError> {
        let deleted = self.store.delete_node(id)?;
        if deleted {
            let _ = self.index.delete_node(id);
        }
        Ok(deleted)
    }

    /// Connect two concepts explicitly with a designated relationship
    pub fn link_nodes(
        &self,
        source_id: &str,
        target_id: &str,
        relation_type: RelationType,
        label: Option<String>,
    ) -> Result<KnowledgeEdge, ServiceError> {
        let edge_id = format!("{}-{}-{:?}", source_id, target_id, relation_type);
        let mut edge = KnowledgeEdge::new(&edge_id, source_id, target_id, relation_type);
        edge.label = label;
        self.store.insert_edge(&edge)?;
        Ok(edge)
    }

    pub fn unlink_nodes(&self, edge_id: &str) -> Result<bool, ServiceError> {
        Ok(self.store.delete_edge(edge_id)?)
    }

    pub fn get_graph(&self) -> Result<GraphVisualizationData, ServiceError> {
        let nodes = self.store.get_all_nodes()?;
        let edges = self.store.get_all_edges()?;
        Ok(GraphVisualizationData { nodes, edges })
    }

    pub fn get_neighbors(
        &self,
        node_id: &str,
        depth: usize,
    ) -> Result<GraphVisualizationData, ServiceError> {
        Ok(self.store.get_neighbors(node_id, depth)?)
    }

    pub fn find_path(
        &self,
        source_id: &str,
        target_id: &str,
    ) -> Result<Vec<KnowledgeNode>, ServiceError> {
        let path_ids = self.store.find_shortest_path(source_id, target_id)?;
        let mut nodes = Vec::new();
        for id in path_ids {
            if let Some(node) = self.store.get_node(&id)? {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    pub fn search(
        &self,
        query: &str,
        semantic: bool,
        limit: usize,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        if query.trim().is_empty() {
            let all_nodes = self.store.get_all_nodes()?;
            let results = all_nodes
                .into_iter()
                .take(limit)
                .map(|node| SearchResult {
                    node,
                    score: 1.0,
                    highlights: Vec::new(),
                })
                .collect();
            return Ok(results);
        }

        if semantic {
            let candidates = self.store.get_all_nodes()?;
            let ranked = self.index.semantic_search(query, &candidates, limit);
            let results = ranked
                .into_iter()
                .map(|(node, score)| SearchResult {
                    node,
                    score,
                    highlights: vec![format!("Semantic similarity: {:.2}", score)],
                })
                .collect();
            Ok(results)
        } else {
            let bm25 = self.index.search_text(query, limit)?;
            let mut results = Vec::new();
            for (node_id, score) in bm25 {
                if let Some(node) = self.store.get_node(&node_id)? {
                    results.push(SearchResult {
                        node,
                        score,
                        highlights: vec![format!("BM25 score: {:.2}", score)],
                    });
                }
            }
            Ok(results)
        }
    }

    pub fn get_metrics(&self) -> Result<GraphMetrics, ServiceError> {
        Ok(self.store.get_metrics()?)
    }

    pub fn record_citation(
        &self,
        node_id: &str,
        authors: Vec<String>,
        year: Option<u32>,
        venue: Option<String>,
        doi: Option<String>,
        bibtex: Option<String>,
    ) -> Result<CitationRecord, ServiceError> {
        let citation = CitationRecord {
            id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            authors,
            publication_year: year,
            source_venue: venue,
            doi,
            bibtex,
        };
        self.store.insert_citation(&citation)?;
        Ok(citation)
    }

    pub fn get_citation(&self, node_id: &str) -> Result<Option<CitationRecord>, ServiceError> {
        Ok(self.store.get_citation_for_node(node_id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_workflow_and_wiki_link_autocreation() {
        let service = KnowledgeService::open_in_memory().unwrap();

        // 1. Create a note referencing a concept via wiki-links
        let note = service
            .save_note(
                None,
                "Neural Attention",
                "Review of [[Transformer]] model for #deep-learning tasks.",
                Some(NodeType::Note),
                None,
            )
            .unwrap();

        assert_eq!(note.title, "Neural Attention");
        assert_eq!(note.tags, vec!["deep-learning"]);

        // 2. Verify graph auto-created the concept stub and edge
        let graph = service.get_graph().unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        let target_stub = graph
            .nodes
            .iter()
            .find(|n| n.title == "Transformer")
            .unwrap();
        assert_eq!(target_stub.node_type, NodeType::Concept);

        // 3. Search note via full-text BM25
        let search_res = service.search("deep-learning", false, 5).unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].node.title, "Neural Attention");

        // 4. Record academic citation
        let citation = service
            .record_citation(
                &note.id,
                vec!["Vaswani et al.".to_string()],
                Some(2017),
                Some("NeurIPS".to_string()),
                Some("10.5555/3295222.3295349".to_string()),
                None,
            )
            .unwrap();
        assert_eq!(citation.authors[0], "Vaswani et al.");

        let fetched_citation = service.get_citation(&note.id).unwrap().unwrap();
        assert_eq!(fetched_citation.publication_year, Some(2017));
    }
}
