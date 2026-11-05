#![forbid(unsafe_code)]
//! IPC Command Handlers for Air-Gapped API & Protocol Studio

use crate::models::{ApiCollection, ApiRequestItem, FuzzingStrategy, KeyValuePair};
use crate::service::ApiStudioService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<ApiStudioService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<ApiStudioService> {
    SERVICE
        .get_or_init(|| {
            let data_dir = platform_common::AppPaths::from_app_name("api-protocol-studio")
                .map(|p| p.data_dir)
                .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            Arc::new(ApiStudioService::open(data_dir).unwrap_or_else(|_| {
                ApiStudioService::open_in_memory().expect("In-memory service fallback")
            }))
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("api-protocol-studio")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::ApiProtocolStudio, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from the Svelte 5 frontend or FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_studio_stats" => {
            let stats = service.get_stats().map_err(|e| e.to_string())?;
            Ok(json!(stats))
        }

        "list_collections" => {
            let colls = service.list_collections().map_err(|e| e.to_string())?;
            Ok(json!(colls))
        }

        "get_collection" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'id'".to_string())?;
            let coll = service.get_collection(id).map_err(|e| e.to_string())?;
            Ok(json!(coll))
        }

        "save_collection" => {
            let coll: ApiCollection = serde_json::from_value(payload["collection"].clone())
                .map_err(|e| format!("Invalid collection JSON: {}", e))?;
            service.save_collection(&coll).map_err(|e| e.to_string())?;
            Ok(json!({ "saved": true, "id": coll.id }))
        }

        "delete_collection" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'id'".to_string())?;
            let deleted = service.delete_collection(id).map_err(|e| e.to_string())?;
            Ok(json!({ "deleted": deleted }))
        }

        "list_requests" => {
            let collection_id = payload["collection_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'collection_id'".to_string())?;
            let requests = service
                .list_requests_in_collection(collection_id)
                .map_err(|e| e.to_string())?;
            Ok(json!(requests))
        }

        "get_request" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'id'".to_string())?;
            let req = service.get_request(id).map_err(|e| e.to_string())?;
            Ok(json!(req))
        }

        "save_request" => {
            let req: ApiRequestItem = serde_json::from_value(payload["request"].clone())
                .map_err(|e| format!("Invalid request item JSON: {}", e))?;
            service.save_request(&req).map_err(|e| e.to_string())?;
            Ok(json!({ "saved": true, "id": req.id }))
        }

        "delete_request" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'id'".to_string())?;
            let deleted = service.delete_request(id).map_err(|e| e.to_string())?;
            Ok(json!({ "deleted": deleted }))
        }

        "execute_request" => {
            let request_id = payload["request_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'request_id'".to_string())?;
            let vars: Option<Vec<KeyValuePair>> = payload["variables"]
                .as_array()
                .map(|arr| serde_json::from_value(Value::Array(arr.clone())).unwrap_or_default());

            let snapshot = service
                .execute_request(request_id, vars)
                .map_err(|e| e.to_string())?;
            Ok(json!(snapshot))
        }

        "fuzz_request" => {
            let request_id = payload["request_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'request_id'".to_string())?;
            let strat_str = payload["strategy"].as_str().unwrap_or("sql_injection");
            let strategy = match strat_str.to_ascii_lowercase().as_str() {
                "command_injection" => FuzzingStrategy::CommandInjection,
                "buffer_overflow" => FuzzingStrategy::BufferOverflow,
                "special_characters" => FuzzingStrategy::SpecialCharacters,
                "boundary_values" => FuzzingStrategy::BoundaryValues,
                _ => FuzzingStrategy::SqlInjection,
            };
            let param_name = payload["param_name"].as_str().unwrap_or("q");

            let results = service
                .fuzz_request(request_id, strategy, param_name)
                .map_err(|e| e.to_string())?;
            Ok(json!(results))
        }

        "get_latest_response" => {
            let request_id = payload["request_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'request_id'".to_string())?;
            let snapshot = service
                .get_latest_response(request_id)
                .map_err(|e| e.to_string())?;
            Ok(json!(snapshot))
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
    fn test_commands_workflow() {
        let stats = dispatch_ipc_command("get_studio_stats", json!({})).unwrap();
        assert!(stats.get("total_collections").is_some());

        let coll_val = json!({
            "id": "c1",
            "name": "Auth API",
            "description": "Authentication endpoints",
            "variables": [{ "key": "host", "value": "127.0.0.1", "enabled": true, "description": null }],
            "created_at": 1700000000,
            "updated_at": 1700000000
        });
        let save_res =
            dispatch_ipc_command("save_collection", json!({ "collection": coll_val })).unwrap();
        assert_eq!(save_res["saved"], true);

        let req_val = json!({
            "id": "r1",
            "collection_id": "c1",
            "name": "Login",
            "protocol": "HttpRest",
            "method": "POST",
            "url": "http://{{host}}:8080/login",
            "headers": [],
            "query_params": [],
            "auth": { "type": "None" },
            "body": { "body_type": "json", "raw_content": "{\"user\":\"admin\"}", "graphql_query": null, "graphql_variables": null },
            "timeout_ms": 5000
        });
        dispatch_ipc_command("save_request", json!({ "request": req_val })).unwrap();

        let reqs = dispatch_ipc_command("list_requests", json!({ "collection_id": "c1" })).unwrap();
        assert_eq!(reqs.as_array().unwrap().len(), 1);

        let lic = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(lic.get("tier").is_some());
    }
}
