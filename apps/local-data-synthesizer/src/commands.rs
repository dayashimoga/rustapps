use crate::service::SynthesizerService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<SynthesizerService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<SynthesizerService> {
    SERVICE
        .get_or_init(|| Arc::new(SynthesizerService::new()))
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("local-data-synthesizer")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::LocalDataSynthesizer, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from Svelte 5 frontend or desktop/mobile FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "get_dataset_config" => {
            let ds_id = payload
                .get("dataset_id")
                .and_then(|v| v.as_str())
                .unwrap_or("ds-fintech-demo");
            let cfg = service
                .get_dataset_config(ds_id)
                .map_err(|e| format!("Failed to get dataset config: {}", e))?;
            Ok(json!(cfg))
        }

        "synthesize_dataset" => {
            let ds_id = payload
                .get("dataset_id")
                .and_then(|v| v.as_str())
                .unwrap_or("ds-fintech-demo");
            let rows_override = payload
                .get("target_rows_override")
                .and_then(|v| v.as_u64())
                .map(|u| u as usize);

            let report = service
                .synthesize_dataset(ds_id, rows_override)
                .map_err(|e| format!("Synthesis failed: {}", e))?;
            Ok(json!(report))
        }

        "get_table_preview" => {
            let ds_id = payload
                .get("dataset_id")
                .and_then(|v| v.as_str())
                .unwrap_or("ds-fintech-demo");
            let table = payload
                .get("table_name")
                .and_then(|v| v.as_str())
                .unwrap_or("customers");
            let limit = payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(25) as usize;

            let rows = service
                .get_table_preview(ds_id, table, limit)
                .map_err(|e| format!("Preview failed: {}", e))?;
            Ok(json!(rows))
        }

        "export_table_csv" => {
            let ds_id = payload
                .get("dataset_id")
                .and_then(|v| v.as_str())
                .unwrap_or("ds-fintech-demo");
            let table = payload
                .get("table_name")
                .and_then(|v| v.as_str())
                .unwrap_or("customers");

            let csv = service
                .export_table_csv(ds_id, table)
                .map_err(|e| format!("CSV export failed: {}", e))?;
            Ok(json!({ "csv": csv }))
        }

        "get_license_status" => {
            let report: LicenseStatusReport = licensing.status_report();
            Ok(json!(report))
        }

        "apply_license_key" => {
            let key = payload
                .get("license_key")
                .or_else(|| payload.get("key"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing license_key".to_string())?;
            let report = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        _ => Err(format!(
            "Unknown local-data-synthesizer command: {}",
            command
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_workflow_end_to_end() {
        // 1. Get stats
        let stats_val = dispatch_ipc_command("get_stats", json!({})).expect("get_stats");
        assert!(stats_val.get("total_schemas_analyzed").is_some());

        // 2. Get dataset config
        let cfg_val =
            dispatch_ipc_command("get_dataset_config", json!({})).expect("get_dataset_config");
        assert_eq!(cfg_val["dataset_id"], "ds-fintech-demo");
        assert_eq!(cfg_val["tables"].as_array().unwrap().len(), 2);

        // 3. Synthesize dataset
        let report_val = dispatch_ipc_command(
            "synthesize_dataset",
            json!({ "dataset_id": "ds-fintech-demo" }),
        )
        .expect("synthesize_dataset");
        assert_eq!(report_val["total_tables"], 2);
        assert_eq!(report_val["total_rows_generated"], 70);

        // 4. Get table preview
        let preview_val = dispatch_ipc_command(
            "get_table_preview",
            json!({ "table_name": "customers", "limit": 10 }),
        )
        .expect("get_table_preview");
        assert_eq!(preview_val.as_array().unwrap().len(), 10);

        // 5. Export CSV
        let csv_val =
            dispatch_ipc_command("export_table_csv", json!({ "table_name": "customers" }))
                .expect("export_table_csv");
        assert!(csv_val["csv"].as_str().unwrap().contains("customer_id"));

        // 6. License status
        let lic_val =
            dispatch_ipc_command("get_license_status", json!({})).expect("get_license_status");
        assert!(lic_val.get("tier").is_some());
    }
}
