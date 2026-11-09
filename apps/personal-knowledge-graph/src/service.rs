#![forbid(unsafe_code)]
//! Knowledge Graph Service Orchestrator

use crate::document_parser::DocumentParser;
use crate::graph_store::{GraphError, GraphStore};
use crate::knowledge_index::{IndexError, KnowledgeSearchIndex};
use crate::models::{
    CitationRecord, FolderSyncSummary, GraphMetrics, GraphVisualizationData, KnowledgeEdge,
    KnowledgeNode, NodeType, PageRankEntry, RelationType, SearchResult, WikiLinkSuggestion,
};
use std::collections::{HashMap, HashSet};
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
        Ok(self.store.get_visualization_data_with_pagerank()?)
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

    /// Computes PageRank centrality rankings for all nodes in the knowledge graph
    pub fn compute_pagerank(&self) -> Result<Vec<PageRankEntry>, ServiceError> {
        let all_nodes = self.store.get_all_nodes()?;
        let title_map: HashMap<String, String> =
            all_nodes.into_iter().map(|n| (n.id, n.title)).collect();

        let pr_map = self.store.compute_pagerank(0.85, 40)?;
        let mut entries: Vec<PageRankEntry> = pr_map
            .into_iter()
            .map(|(node_id, score)| {
                let title = title_map
                    .get(&node_id)
                    .cloned()
                    .unwrap_or_else(|| node_id.clone());
                PageRankEntry {
                    node_id,
                    title,
                    score,
                    rank: 0,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.rank = i + 1;
        }

        Ok(entries)
    }

    /// Autocomplete query matching wiki-links `[[Prefix...]]`
    pub fn suggest_wiki_links(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<WikiLinkSuggestion>, ServiceError> {
        Ok(self.store.suggest_wiki_links(prefix, limit)?)
    }

    /// Two-way synchronization between a local directory of `.md` files and the knowledge graph
    pub fn sync_markdown_folder(
        &self,
        folder_path: &Path,
    ) -> Result<FolderSyncSummary, ServiceError> {
        if !folder_path.exists() {
            std::fs::create_dir_all(folder_path)
                .map_err(|e| IndexError::Tantivy(tantivy::TantivyError::IoError(Arc::new(e))))?;
        }

        let mut imported_count = 0usize;
        let mut updated_count = 0usize;
        let mut exported_count = 0usize;

        // 1. Ingest .md files from disk into Knowledge Graph
        let mut disk_files = Vec::new();
        let mut dir_entries = vec![folder_path.to_path_buf()];

        while let Some(current_dir) = dir_entries.pop() {
            if let Ok(read_dir) = std::fs::read_dir(&current_dir) {
                for entry in read_dir.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        dir_entries.push(path);
                    } else if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext.eq_ignore_ascii_case("md")
                                || ext.eq_ignore_ascii_case("markdown")
                            {
                                disk_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        let mut disk_titles = HashSet::new();

        for file_path in disk_files {
            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut title = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(h1) = trimmed.strip_prefix("# ") {
                    let t = h1.trim().to_string();
                    if !t.is_empty() {
                        title = Some(t);
                        break;
                    }
                }
            }
            let title = title.unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled Note".to_string())
            });

            disk_titles.insert(title.to_lowercase());

            if let Some(existing) = self.store.get_node_by_title(&title)? {
                if existing.content != content {
                    self.save_note(
                        Some(&existing.id),
                        &title,
                        &content,
                        Some(existing.node_type),
                        Some(existing.tags),
                    )?;
                    updated_count += 1;
                }
            } else {
                self.save_note(None, &title, &content, Some(NodeType::Note), None)?;
                imported_count += 1;
            }
        }

        // 2. Export nodes from Knowledge Graph to disk if missing
        let all_nodes = self.store.get_all_nodes()?;
        for node in all_nodes {
            if !disk_titles.contains(&node.title.to_lowercase()) {
                let safe_title: String = node
                    .title
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let safe_name = safe_title.trim();
                let file_name = if safe_name.is_empty() {
                    format!("{}.md", node.id)
                } else {
                    format!("{}.md", safe_name)
                };
                let target_file = folder_path.join(file_name);

                let mut out = String::with_capacity(node.content.len() + 64);
                if !node.content.starts_with("# ") {
                    out.push_str(&format!("# {}\n\n", node.title));
                }
                out.push_str(&node.content);

                if std::fs::write(&target_file, out.as_bytes()).is_ok() {
                    exported_count += 1;
                }
            }
        }

        let total_synced_edges = self.store.get_all_edges()?.len();

        Ok(FolderSyncSummary {
            folder_path: folder_path.to_string_lossy().to_string(),
            imported_count,
            exported_count,
            updated_count,
            total_synced_edges,
        })
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

    #[test]
    fn test_sync_markdown_folder_two_way() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("vault");
        std::fs::create_dir_all(&folder).unwrap();

        // Write initial markdown files
        let doc1 = folder.join("Quantum.md");
        std::fs::write(
            &doc1,
            "# Quantum Computing\n\nStudy of [[Qubits]] and entanglement.",
        )
        .unwrap();

        let doc2 = folder.join("Entropy.md");
        std::fs::write(
            &doc2,
            "# Information Entropy\n\nShannon entropy measurements.",
        )
        .unwrap();

        let service = KnowledgeService::open_in_memory().unwrap();

        // Create an existing note in the graph referencing Quantum Computing
        service
            .save_note(
                None,
                "Cryptography",
                "RSA, ECC, and [[Quantum Computing]] resilience.",
                Some(NodeType::Note),
                None,
            )
            .unwrap();

        // Perform two-way sync
        let summary = service.sync_markdown_folder(&folder).unwrap();
        assert_eq!(summary.imported_count, 1); // Entropy.md is fresh imported
        assert_eq!(summary.updated_count, 1); // Quantum.md updates existing stub
        assert_eq!(summary.exported_count, 2); // Cryptography.md and auto-created Qubits.md exported

        // Verify disk files
        assert!(folder.join("Cryptography.md").exists());
        assert!(folder.join("Qubits.md").exists());

        // Verify graph nodes
        let nodes = service.get_graph().unwrap().nodes;
        let titles: Vec<String> = nodes.into_iter().map(|n| n.title).collect();
        assert!(titles.contains(&"Quantum Computing".to_string()));
        assert!(titles.contains(&"Cryptography".to_string()));
        assert!(titles.contains(&"Qubits".to_string()));
    }

    #[test]
    fn test_service_pagerank_and_wiki_suggestions() {
        let service = KnowledgeService::open_in_memory().unwrap();

        service
            .save_note(
                None,
                "Machine Learning",
                "Core overview linking to [[Neural Networks]] and [[Deep Learning]].",
                Some(NodeType::Document),
                None,
            )
            .unwrap();

        let ranking = service.compute_pagerank().unwrap();
        assert!(!ranking.is_empty());
        assert_eq!(ranking[0].rank, 1);

        let suggestions = service.suggest_wiki_links("Neural", 5).unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].title, "Neural Networks");
    }
}
