#![forbid(unsafe_code)]
//! SQLite-backed Graph Storage Engine for Knowledge Nodes, Semantic Edges, and Citations

use crate::models::{
    CitationRecord, GraphMetrics, GraphVisualizationData, KnowledgeEdge, KnowledgeNode, NodeType,
    RelationType,
};
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

/// Thread-safe SQLite Graph Store
pub struct GraphStore {
    conn: Mutex<Connection>,
}

impl GraphStore {
    /// Opens an in-memory SQLite store (ideal for tests and scratch graphs)
    pub fn open_in_memory() -> Result<Self, GraphError> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Opens or creates a persistent SQLite graph database at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GraphError> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), GraphError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                node_type TEXT NOT NULL,
                content TEXT NOT NULL,
                tags_json TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
                target_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
                relation_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                label TEXT,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS citations (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
                authors_json TEXT NOT NULL,
                publication_year INTEGER,
                source_venue TEXT,
                doi TEXT,
                bibtex TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);
            CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);
            CREATE INDEX IF NOT EXISTS idx_nodes_title ON nodes(title);
            CREATE INDEX IF NOT EXISTS idx_citations_node ON citations(node_id);
            ",
        )?;
        Ok(())
    }

    pub fn insert_node(&self, node: &KnowledgeNode) -> Result<(), GraphError> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&node.tags)?;
        let metadata_json = serde_json::to_string(&node.metadata)?;

        conn.execute(
            "INSERT OR REPLACE INTO nodes (id, title, node_type, content, tags_json, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                node.id,
                node.title,
                node.node_type.as_str(),
                node.content,
                tags_json,
                metadata_json,
                node.created_at,
                node.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Result<Option<KnowledgeNode>, GraphError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, node_type, content, tags_json, metadata_json, created_at, updated_at
             FROM nodes WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let node_type_str: String = row.get(2)?;
            let tags_json: String = row.get(4)?;
            let metadata_json: String = row.get(5)?;

            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

            Ok(Some(KnowledgeNode {
                id: row.get(0)?,
                title: row.get(1)?,
                node_type: NodeType::from_str_lenient(&node_type_str),
                content: row.get(3)?,
                tags,
                metadata,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                pagerank: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_node_by_title(&self, title: &str) -> Result<Option<KnowledgeNode>, GraphError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, node_type, content, tags_json, metadata_json, created_at, updated_at
             FROM nodes WHERE LOWER(title) = LOWER(?1) LIMIT 1",
        )?;
        let mut rows = stmt.query(params![title])?;

        if let Some(row) = rows.next()? {
            let node_type_str: String = row.get(2)?;
            let tags_json: String = row.get(4)?;
            let metadata_json: String = row.get(5)?;

            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

            Ok(Some(KnowledgeNode {
                id: row.get(0)?,
                title: row.get(1)?,
                node_type: NodeType::from_str_lenient(&node_type_str),
                content: row.get(3)?,
                tags,
                metadata,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                pagerank: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_node(&self, id: &str) -> Result<bool, GraphError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM nodes WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn insert_edge(&self, edge: &KnowledgeEdge) -> Result<(), GraphError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO edges (id, source_id, target_id, relation_type, weight, label, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                edge.id,
                edge.source_id,
                edge.target_id,
                edge.relation_type.as_str(),
                edge.weight,
                edge.label,
                edge.created_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_edge(&self, id: &str) -> Result<bool, GraphError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM edges WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn get_all_nodes(&self) -> Result<Vec<KnowledgeNode>, GraphError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, node_type, content, tags_json, metadata_json, created_at, updated_at
             FROM nodes ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let node_type_str: String = row.get(2)?;
            let tags_json: String = row.get(4)?;
            let metadata_json: String = row.get(5)?;

            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

            Ok(KnowledgeNode {
                id: row.get(0)?,
                title: row.get(1)?,
                node_type: NodeType::from_str_lenient(&node_type_str),
                content: row.get(3)?,
                tags,
                metadata,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                pagerank: None,
            })
        })?;

        let mut nodes = Vec::new();
        for node in rows {
            nodes.push(node?);
        }
        Ok(nodes)
    }

    pub fn get_all_edges(&self) -> Result<Vec<KnowledgeEdge>, GraphError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, source_id, target_id, relation_type, weight, label, created_at
             FROM edges",
        )?;
        let rows = stmt.query_map([], |row| {
            let rel_str: String = row.get(3)?;
            Ok(KnowledgeEdge {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relation_type: RelationType::from_str_lenient(&rel_str),
                weight: row.get(4)?,
                label: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut edges = Vec::new();
        for edge in rows {
            edges.push(edge?);
        }
        Ok(edges)
    }

    /// Fetches ego-network / neighbors around a target node up to a given traversal depth
    pub fn get_neighbors(
        &self,
        start_id: &str,
        max_depth: usize,
    ) -> Result<GraphVisualizationData, GraphError> {
        let all_edges = self.get_all_edges()?;
        let mut visited_nodes = HashSet::new();
        let mut collected_edges = Vec::new();
        let mut queue = VecDeque::new();

        visited_nodes.insert(start_id.to_string());
        queue.push_back((start_id.to_string(), 0usize));

        while let Some((curr_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in &all_edges {
                if edge.source_id == curr_id {
                    collected_edges.push(edge.clone());
                    if !visited_nodes.contains(&edge.target_id) {
                        visited_nodes.insert(edge.target_id.clone());
                        queue.push_back((edge.target_id.clone(), depth + 1));
                    }
                } else if edge.target_id == curr_id {
                    collected_edges.push(edge.clone());
                    if !visited_nodes.contains(&edge.source_id) {
                        visited_nodes.insert(edge.source_id.clone());
                        queue.push_back((edge.source_id.clone(), depth + 1));
                    }
                }
            }
        }

        let mut nodes = Vec::new();
        for node_id in visited_nodes {
            if let Some(node) = self.get_node(&node_id)? {
                nodes.push(node);
            }
        }

        // Deduplicate edges
        collected_edges.sort_by(|a, b| a.id.cmp(&b.id));
        collected_edges.dedup_by(|a, b| a.id == b.id);

        Ok(GraphVisualizationData {
            nodes,
            edges: collected_edges,
        })
    }

    /// Breadth-First Search (BFS) finding the shortest semantic citation/reference path between two concepts
    pub fn find_shortest_path(
        &self,
        source_id: &str,
        target_id: &str,
    ) -> Result<Vec<String>, GraphError> {
        if source_id == target_id {
            return Ok(vec![source_id.to_string()]);
        }

        let all_edges = self.get_all_edges()?;
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for edge in all_edges {
            adj.entry(edge.source_id.clone())
                .or_default()
                .push(edge.target_id.clone());
            adj.entry(edge.target_id).or_default().push(edge.source_id);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent_map: HashMap<String, String> = HashMap::new();

        visited.insert(source_id.to_string());
        queue.push_back(source_id.to_string());

        let mut found = false;
        while let Some(curr) = queue.pop_front() {
            if curr == target_id {
                found = true;
                break;
            }
            if let Some(neighbors) = adj.get(&curr) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        parent_map.insert(neighbor.clone(), curr.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if !found {
            return Ok(Vec::new());
        }

        let mut path = Vec::new();
        let mut curr = target_id.to_string();
        while curr != source_id {
            path.push(curr.clone());
            if let Some(parent) = parent_map.get(&curr) {
                curr = parent.clone();
            } else {
                break;
            }
        }
        path.push(source_id.to_string());
        path.reverse();
        Ok(path)
    }

    /// Computes summary metrics and degree statistics
    pub fn get_metrics(&self) -> Result<GraphMetrics, GraphError> {
        let nodes = self.get_all_nodes()?;
        let edges = self.get_all_edges()?;

        let mut node_type_counts = HashMap::new();
        let mut tag_counts = HashMap::new();

        for node in &nodes {
            *node_type_counts
                .entry(node.node_type.as_str().to_string())
                .or_insert(0) += 1;
            for tag in &node.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let total_nodes = nodes.len();
        let total_edges = edges.len();

        let density = if total_nodes > 1 {
            (total_edges as f64) / ((total_nodes * (total_nodes - 1)) as f64)
        } else {
            0.0
        };

        Ok(GraphMetrics {
            total_nodes,
            total_edges,
            node_type_counts,
            tag_counts,
            density,
        })
    }

    pub fn insert_citation(&self, citation: &CitationRecord) -> Result<(), GraphError> {
        let conn = self.conn.lock().unwrap();
        let authors_json = serde_json::to_string(&citation.authors)?;
        conn.execute(
            "INSERT OR REPLACE INTO citations (id, node_id, authors_json, publication_year, source_venue, doi, bibtex)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                citation.id,
                citation.node_id,
                authors_json,
                citation.publication_year,
                citation.source_venue,
                citation.doi,
                citation.bibtex
            ],
        )?;
        Ok(())
    }

    pub fn get_citation_for_node(
        &self,
        node_id: &str,
    ) -> Result<Option<CitationRecord>, GraphError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, node_id, authors_json, publication_year, source_venue, doi, bibtex
             FROM citations WHERE node_id = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![node_id])?;

        if let Some(row) = rows.next()? {
            let authors_json: String = row.get(2)?;
            let authors: Vec<String> = serde_json::from_str(&authors_json).unwrap_or_default();
            Ok(Some(CitationRecord {
                id: row.get(0)?,
                node_id: row.get(1)?,
                authors,
                publication_year: row.get(3)?,
                source_venue: row.get(4)?,
                doi: row.get(5)?,
                bibtex: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Computes iterative PageRank centrality over the knowledge graph
    pub fn compute_pagerank(
        &self,
        damping: f64,
        max_iterations: usize,
    ) -> Result<HashMap<String, f64>, GraphError> {
        let all_nodes = self.get_all_nodes()?;
        let n = all_nodes.len();
        if n == 0 {
            return Ok(HashMap::new());
        }

        let all_edges = self.get_all_edges()?;

        let mut id_to_idx = HashMap::with_capacity(n);
        let mut idx_to_id = Vec::with_capacity(n);
        for (i, node) in all_nodes.iter().enumerate() {
            id_to_idx.insert(node.id.clone(), i);
            idx_to_id.push(node.id.clone());
        }

        let mut out_degree = vec![0usize; n];
        let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];

        for edge in all_edges {
            if let (Some(&src_idx), Some(&tgt_idx)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                if src_idx != tgt_idx {
                    out_degree[src_idx] += 1;
                    incoming[tgt_idx].push(src_idx);
                }
            }
        }

        let initial_val = 1.0 / (n as f64);
        let mut pr = vec![initial_val; n];
        let mut next_pr = vec![0.0f64; n];

        for _ in 0..max_iterations {
            let mut dangling_sum = 0.0f64;
            for i in 0..n {
                if out_degree[i] == 0 {
                    dangling_sum += pr[i];
                }
            }

            let base_score = (1.0 - damping) / (n as f64) + damping * (dangling_sum / (n as f64));
            let mut max_delta = 0.0f64;

            for i in 0..n {
                let mut in_sum = 0.0f64;
                for &pred in &incoming[i] {
                    let out_d = out_degree[pred];
                    if out_d > 0 {
                        in_sum += pr[pred] / (out_d as f64);
                    }
                }
                let new_val = base_score + damping * in_sum;
                let delta = (new_val - pr[i]).abs();
                if delta > max_delta {
                    max_delta = delta;
                }
                next_pr[i] = new_val;
            }

            pr.copy_from_slice(&next_pr);
            if max_delta < 1e-6 {
                break;
            }
        }

        let mut result = HashMap::with_capacity(n);
        for (i, id) in idx_to_id.into_iter().enumerate() {
            result.insert(id, pr[i]);
        }

        Ok(result)
    }

    /// Fetches visualization payload with nodes enriched by their computed PageRank score
    pub fn get_visualization_data_with_pagerank(
        &self,
    ) -> Result<GraphVisualizationData, GraphError> {
        let mut nodes = self.get_all_nodes()?;
        let edges = self.get_all_edges()?;
        let pagerank_map = self.compute_pagerank(0.85, 30)?;

        for node in &mut nodes {
            if let Some(&score) = pagerank_map.get(&node.id) {
                node.pagerank = Some(score);
            }
        }

        Ok(GraphVisualizationData { nodes, edges })
    }

    /// Autocomplete query matching wiki-links `[[Prefix...]]`
    pub fn suggest_wiki_links(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::WikiLinkSuggestion>, GraphError> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", prefix.trim());
        let mut stmt = conn.prepare(
            "SELECT title, node_type, content, tags_json FROM nodes WHERE title LIKE ?1 ORDER BY title LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            let title: String = row.get(0)?;
            let node_type: String = row.get(1)?;
            let content: String = row.get(2)?;
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let snippet: String = content.chars().take(80).collect();
            Ok(crate::models::WikiLinkSuggestion {
                title,
                node_type,
                snippet,
                tags,
            })
        })?;

        let mut suggestions = Vec::new();
        for s in rows {
            suggestions.push(s?);
        }
        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_store_crud_and_relationships() {
        let store = GraphStore::open_in_memory().unwrap();

        let mut node_a = KnowledgeNode::new(
            "node-1",
            "Transformer Architecture",
            NodeType::Document,
            "Attention is all you need",
        );
        node_a.tags = vec!["ai".to_string(), "deep-learning".to_string()];
        store.insert_node(&node_a).unwrap();

        let node_b = KnowledgeNode::new(
            "node-2",
            "Self-Attention Mechanism",
            NodeType::Concept,
            "Scaled dot-product attention",
        );
        store.insert_node(&node_b).unwrap();

        let edge = KnowledgeEdge::new("edge-1", "node-1", "node-2", RelationType::ParentOf);
        store.insert_edge(&edge).unwrap();

        let fetched_a = store.get_node("node-1").unwrap().unwrap();
        assert_eq!(fetched_a.title, "Transformer Architecture");
        assert_eq!(fetched_a.tags.len(), 2);

        let fetched_by_title = store
            .get_node_by_title("self-attention mechanism")
            .unwrap()
            .unwrap();
        assert_eq!(fetched_by_title.id, "node-2");

        let neighbors = store.get_neighbors("node-1", 1).unwrap();
        assert_eq!(neighbors.nodes.len(), 2);
        assert_eq!(neighbors.edges.len(), 1);

        let metrics = store.get_metrics().unwrap();
        assert_eq!(metrics.total_nodes, 2);
        assert_eq!(metrics.total_edges, 1);
        assert_eq!(*metrics.node_type_counts.get("document").unwrap(), 1);
    }

    #[test]
    fn test_shortest_path_bfs() {
        let store = GraphStore::open_in_memory().unwrap();

        // Path: A -> B -> C -> D
        let n1 = KnowledgeNode::new("A", "Concept A", NodeType::Concept, "");
        let n2 = KnowledgeNode::new("B", "Concept B", NodeType::Concept, "");
        let n3 = KnowledgeNode::new("C", "Concept C", NodeType::Concept, "");
        let n4 = KnowledgeNode::new("D", "Concept D", NodeType::Concept, "");
        store.insert_node(&n1).unwrap();
        store.insert_node(&n2).unwrap();
        store.insert_node(&n3).unwrap();
        store.insert_node(&n4).unwrap();

        store
            .insert_edge(&KnowledgeEdge::new("e1", "A", "B", RelationType::RelatedTo))
            .unwrap();
        store
            .insert_edge(&KnowledgeEdge::new("e2", "B", "C", RelationType::RelatedTo))
            .unwrap();
        store
            .insert_edge(&KnowledgeEdge::new("e3", "C", "D", RelationType::RelatedTo))
            .unwrap();

        let path = store.find_shortest_path("A", "D").unwrap();
        assert_eq!(path, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_pagerank_centrality() {
        let store = GraphStore::open_in_memory().unwrap();

        // Star graph: Hub receives incoming links from Leaf1, Leaf2, Leaf3
        let hub = KnowledgeNode::new("hub", "Central Concept", NodeType::Concept, "");
        let l1 = KnowledgeNode::new("l1", "Leaf 1", NodeType::Note, "");
        let l2 = KnowledgeNode::new("l2", "Leaf 2", NodeType::Note, "");
        let l3 = KnowledgeNode::new("l3", "Leaf 3", NodeType::Note, "");

        store.insert_node(&hub).unwrap();
        store.insert_node(&l1).unwrap();
        store.insert_node(&l2).unwrap();
        store.insert_node(&l3).unwrap();

        store
            .insert_edge(&KnowledgeEdge::new("e1", "l1", "hub", RelationType::Cites))
            .unwrap();
        store
            .insert_edge(&KnowledgeEdge::new("e2", "l2", "hub", RelationType::Cites))
            .unwrap();
        store
            .insert_edge(&KnowledgeEdge::new("e3", "l3", "hub", RelationType::Cites))
            .unwrap();

        let pr = store.compute_pagerank(0.85, 30).unwrap();
        assert_eq!(pr.len(), 4);

        let hub_score = *pr.get("hub").unwrap();
        let l1_score = *pr.get("l1").unwrap();
        assert!(
            hub_score > l1_score,
            "Hub score ({}) must be greater than leaf ({})",
            hub_score,
            l1_score
        );

        let vis = store.get_visualization_data_with_pagerank().unwrap();
        let hub_node = vis.nodes.iter().find(|n| n.id == "hub").unwrap();
        assert!(hub_node.pagerank.is_some());
    }

    #[test]
    fn test_suggest_wiki_links() {
        let store = GraphStore::open_in_memory().unwrap();

        let n1 = KnowledgeNode::new(
            "1",
            "Machine Learning Fundamentals",
            NodeType::Document,
            "Intro content",
        );
        let n2 = KnowledgeNode::new(
            "2",
            "Markov Decision Process",
            NodeType::Concept,
            "Reinforcement learning",
        );
        let n3 = KnowledgeNode::new("3", "Deep Learning", NodeType::Note, "Neural nets");

        store.insert_node(&n1).unwrap();
        store.insert_node(&n2).unwrap();
        store.insert_node(&n3).unwrap();

        let suggestions = store.suggest_wiki_links("Learn", 10).unwrap();
        assert_eq!(suggestions.len(), 2);
        let titles: Vec<String> = suggestions.into_iter().map(|s| s.title).collect();
        assert!(titles.contains(&"Machine Learning Fundamentals".to_string()));
        assert!(titles.contains(&"Deep Learning".to_string()));
    }
}
