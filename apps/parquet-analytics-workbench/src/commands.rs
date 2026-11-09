#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Local Database & Parquet Analytics Workbench

use crate::models::{ChartRequest, QueryRequest};
use crate::service::AnalyticsService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<AnalyticsService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<AnalyticsService> {
    SERVICE
        .get_or_init(|| {
            let s = AnalyticsService::new().expect("Failed to initialize AnalyticsService");
            // Auto-seed a demonstration dataset so the workbench opens with data ready
            let _ = s.seed_demo_dataset();
            Arc::new(s)
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("parquet-analytics-workbench")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::ParquetAnalyticsWorkbench, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from Svelte 5 frontend or mobile FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_workbench_stats" => {
            let stats = service.get_workbench_stats();
            Ok(json!(stats))
        }

        "list_datasets" => {
            let datasets = service.list_datasets();
            Ok(json!(datasets))
        }

        "get_schema" => {
            let dataset_id = payload["dataset_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'dataset_id'".to_string())?;
            let schema = service.get_schema(dataset_id).map_err(|e| e.to_string())?;
            Ok(json!(schema))
        }

        "open_dataset" => {
            let path = payload["path"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'path'".to_string())?;
            let default_name = std::path::Path::new(path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("Dataset");
            let name = payload["name"].as_str().unwrap_or(default_name);
            let dataset_id = format!("ds-{}", uuid::Uuid::new_v4().simple());

            let schema = service
                .load_dataset(&dataset_id, name, path)
                .map_err(|e| e.to_string())?;
            Ok(json!(schema))
        }

        "seed_demo_dataset" => {
            let schema = service.seed_demo_dataset().map_err(|e| e.to_string())?;
            Ok(json!(schema))
        }

        "query_dataset" => {
            let query_req: QueryRequest = serde_json::from_value(payload["query"].clone())
                .map_err(|e| format!("Invalid query payload: {}", e))?;
            let page = service
                .query_dataset(&query_req)
                .map_err(|e| e.to_string())?;
            Ok(json!(page))
        }

        "compute_chart_data" => {
            let chart_req: ChartRequest = serde_json::from_value(payload["chart"].clone())
                .map_err(|e| format!("Invalid chart payload: {}", e))?;
            let chart = service
                .compute_chart_data(&chart_req)
                .map_err(|e| e.to_string())?;
            Ok(json!(chart))
        }

        "compute_column_stats" => {
            let dataset_id = payload["dataset_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'dataset_id'".to_string())?;
            let column_name = payload["column_name"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'column_name'".to_string())?;
            let stats = service
                .compute_column_stats(dataset_id, column_name)
                .map_err(|e| e.to_string())?;
            Ok(json!(stats))
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

        "execute_sql" | "execute_sql_query" => {
            let sql = payload["query"]
                .as_str()
                .or_else(|| payload["sql"].as_str())
                .ok_or_else(|| "Missing required parameter 'query'".to_string())?;
            let res = service.execute_sql(sql).map_err(|e| e.to_string())?;
            Ok(json!(res))
        }

        "export_dataset" => {
            let dataset_id = payload["dataset_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'dataset_id'".to_string())?;
            let out_path = payload["out_path"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'out_path'".to_string())?;
            let format_str = payload["format"].as_str().unwrap_or("csv");
            let format = match format_str.to_ascii_lowercase().as_str() {
                "jsonl" | "ndjson" => crate::models::DatasetFormat::JsonLines,
                _ => crate::models::DatasetFormat::Csv,
            };
            let summary = service
                .export_dataset(dataset_id, out_path, format)
                .map_err(|e| e.to_string())?;
            Ok(json!(summary))
        }

        _ => Err(format!("Unknown command: '{}'", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_dispatch_workflow() {
        let stats = dispatch_ipc_command("get_workbench_stats", json!({})).unwrap();
        assert!(stats.get("total_datasets").is_some());

        let datasets = dispatch_ipc_command("list_datasets", json!({})).unwrap();
        let arr = datasets.as_array().unwrap();
        assert!(!arr.is_empty());
        let first_id = arr[0]["dataset_id"].as_str().unwrap();

        let query_val = json!({
            "query": {
                "dataset_id": first_id,
                "filters": [],
                "sort": null,
                "search_query": null,
                "limit": 5,
                "offset": 0
            }
        });
        let page = dispatch_ipc_command("query_dataset", query_val).unwrap();
        assert_eq!(page["rows"].as_array().unwrap().len(), 5);

        let lic = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(lic.get("tier").is_some());

        // Test SQL query execution
        let datasets = dispatch_ipc_command("list_datasets", json!({})).unwrap();
        let arr = datasets.as_array().unwrap();
        let first_id = arr[0]["dataset_id"].as_str().unwrap();
        let tbl_name = format!("ds_{}", first_id.replace('-', "_"));

        let sql_res = dispatch_ipc_command(
            "execute_sql",
            json!({
                "query": format!("SELECT datacenter, count(*) as count FROM \"{}\" GROUP BY datacenter", tbl_name)
            }),
        )
        .unwrap();
        assert_eq!(sql_res["columns"], json!(["datacenter", "count"]));
        assert!(sql_res["row_count"].as_u64().unwrap() > 0);

        // Test CSV export
        let temp = tempfile::tempdir().unwrap();
        let csv_out = temp.path().join("export.csv");
        let exp_res = dispatch_ipc_command(
            "export_dataset",
            json!({
                "dataset_id": first_id,
                "out_path": csv_out.to_str().unwrap(),
                "format": "csv"
            }),
        )
        .unwrap();
        assert_eq!(exp_res["format"], "csv");
        assert!(csv_out.exists());

        // Test JSONL export
        let jsonl_out = temp.path().join("export.jsonl");
        let exp_jsonl = dispatch_ipc_command(
            "export_dataset",
            json!({
                "dataset_id": first_id,
                "out_path": jsonl_out.to_str().unwrap(),
                "format": "jsonl"
            }),
        )
        .unwrap();
        assert_eq!(exp_jsonl["format"], "jsonl");
        assert!(jsonl_out.exists());
    }
}
