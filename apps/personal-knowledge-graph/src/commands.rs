#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Personal Knowledge Graph & Research Studio

use crate::models::{NodeType, RelationType};
use crate::service::KnowledgeService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<KnowledgeService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<KnowledgeService> {
    SERVICE
        .get_or_init(|| {
            let data_dir = platform_common::AppPaths::from_app_name("personal-knowledge-graph")
                .map(|p| p.data_dir)
                .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            Arc::new(KnowledgeService::open(data_dir).unwrap_or_else(|_| {
                KnowledgeService::open_in_memory().expect("In-memory service fallback")
            }))
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("personal-knowledge-graph")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::PersonalKnowledgeGraph, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from the Svelte 5 desktop frontend or FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_graph_stats" => {
            let metrics = service.get_metrics().map_err(|e| e.to_string())?;
            Ok(json!(metrics))
        }

        "get_graph_visualization" => {
            let data = service.get_graph().map_err(|e| e.to_string())?;
            Ok(json!(data))
        }

        "get_node" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'id'".to_string())?;
            let node = service.get_node(id).map_err(|e| e.to_string())?;
            Ok(json!(node))
        }

        "save_note" => {
            let id = payload["id"].as_str();
            let title = payload["title"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'title'".to_string())?;
            let content = payload["content"].as_str().unwrap_or("");
            let node_type = payload["node_type"]
                .as_str()
                .map(NodeType::from_str_lenient);
            let tags = payload["tags"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            let saved = service
                .save_note(id, title, content, node_type, tags)
                .map_err(|e| e.to_string())?;
            Ok(json!(saved))
        }

        "delete_node" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'id'".to_string())?;
            let deleted = service.delete_node(id).map_err(|e| e.to_string())?;
            Ok(json!({ "deleted": deleted }))
        }

        "link_nodes" => {
            let source_id = payload["source_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'source_id'".to_string())?;
            let target_id = payload["target_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'target_id'".to_string())?;
            let rel_str = payload["relation_type"].as_str().unwrap_or("related_to");
            let relation_type = RelationType::from_str_lenient(rel_str);
            let label = payload["label"].as_str().map(String::from);

            let edge = service
                .link_nodes(source_id, target_id, relation_type, label)
                .map_err(|e| e.to_string())?;
            Ok(json!(edge))
        }

        "unlink_nodes" => {
            let edge_id = payload["edge_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'edge_id'".to_string())?;
            let deleted = service.unlink_nodes(edge_id).map_err(|e| e.to_string())?;
            Ok(json!({ "deleted": deleted }))
        }

        "get_node_neighbors" => {
            let node_id = payload["node_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'node_id'".to_string())?;
            let depth = payload["depth"].as_u64().unwrap_or(1) as usize;
            let data = service
                .get_neighbors(node_id, depth)
                .map_err(|e| e.to_string())?;
            Ok(json!(data))
        }

        "find_path" => {
            let source_id = payload["source_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'source_id'".to_string())?;
            let target_id = payload["target_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'target_id'".to_string())?;
            let path = service
                .find_path(source_id, target_id)
                .map_err(|e| e.to_string())?;
            Ok(json!(path))
        }

        "search_knowledge" => {
            let query = payload["query"].as_str().unwrap_or("");
            let semantic = payload["semantic"].as_bool().unwrap_or(false);
            let limit = payload["limit"].as_u64().unwrap_or(20) as usize;

            let results = service
                .search(query, semantic, limit)
                .map_err(|e| e.to_string())?;
            Ok(json!(results))
        }

        "record_citation" => {
            let node_id = payload["node_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'node_id'".to_string())?;
            let authors: Vec<String> = payload["authors"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let year = payload["publication_year"].as_u64().map(|y| y as u32);
            let venue = payload["source_venue"].as_str().map(String::from);
            let doi = payload["doi"].as_str().map(String::from);
            let bibtex = payload["bibtex"].as_str().map(String::from);

            let citation = service
                .record_citation(node_id, authors, year, venue, doi, bibtex)
                .map_err(|e| e.to_string())?;
            Ok(json!(citation))
        }

        "get_citation" => {
            let node_id = payload["node_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'node_id'".to_string())?;
            let citation = service.get_citation(node_id).map_err(|e| e.to_string())?;
            Ok(json!(citation))
        }

        "get_license_status" => {
            let status: LicenseStatusReport = licensing.status_report();
            Ok(json!(status))
        }

        "activate_license" => {
            let license_json = payload["license_json"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'license_json'".to_string())?;
            let status = licensing
                .activate(license_json)
                .map_err(|e| e.to_string())?;
            Ok(json!(status))
        }

        _ => Err(format!("Unknown command: '{}'", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_commands_workflow() {
        let stats = dispatch_ipc_command("get_graph_stats", json!({})).unwrap();
        assert!(stats.get("total_nodes").is_some());

        let note = dispatch_ipc_command(
            "save_note",
            json!({
                "title": "Quantum Error Correction",
                "content": "Surface codes and [[Toric Code]] research.",
                "tags": ["quantum", "physics"]
            }),
        )
        .unwrap();
        let note_id = note["id"].as_str().unwrap();

        let fetched = dispatch_ipc_command("get_node", json!({ "id": note_id })).unwrap();
        assert_eq!(fetched["title"], "Quantum Error Correction");

        let search = dispatch_ipc_command(
            "search_knowledge",
            json!({
                "query": "quantum",
                "semantic": false
            }),
        )
        .unwrap();
        assert!(!search.as_array().unwrap().is_empty());

        let lic_status = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(lic_status.get("tier").is_some());
    }
}
