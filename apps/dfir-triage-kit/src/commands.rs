#![forbid(unsafe_code)]
//! IPC Command Handlers for Digital Forensics & Incident Response (DFIR) Triage Kit

use crate::collector::ForensicCollector;
use crate::service::DfirService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::path::Path;
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<DfirService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<DfirService> {
    SERVICE
        .get_or_init(|| {
            let data_dir = platform_common::AppPaths::from_app_name("dfir-triage-kit")
                .map(|p| p.data_dir)
                .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            let s = DfirService::open(&data_dir)
                .unwrap_or_else(|_| DfirService::open_in_memory().expect("In-memory DFIR service"));
            // Auto run an initial triage sweep if database is empty
            if let Ok(runs) = s.list_triage_runs() {
                if runs.is_empty() {
                    let _ = s.execute_triage(None);
                }
            }
            Arc::new(s)
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("dfir-triage-kit")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::DfirTriageKit, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from Svelte 5 frontend or mobile FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_dfir_stats" => {
            let stats = service.get_stats().map_err(|e| e.to_string())?;
            Ok(json!(stats))
        }

        "execute_triage" => {
            let dir_opt = payload["target_dir"].as_str().map(Path::new);
            let package = service.execute_triage(dir_opt).map_err(|e| e.to_string())?;
            Ok(json!(package))
        }

        "list_triage_runs" => {
            let runs = service.list_triage_runs().map_err(|e| e.to_string())?;
            Ok(json!(runs))
        }

        "get_triage_package" => {
            let triage_id = payload["triage_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'triage_id'".to_string())?;
            let package = service
                .get_triage_package(triage_id)
                .map_err(|e| e.to_string())?;
            Ok(json!(package))
        }

        "scan_directory" => {
            let dir_str = payload["path"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'path'".to_string())?;
            let max_depth = payload["max_depth"].as_u64().unwrap_or(3) as usize;
            let records = ForensicCollector::scan_directory_integrity(dir_str, max_depth, 100);
            Ok(json!(records))
        }

        "export_signed_bundle" => {
            let triage_id = payload["triage_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'triage_id'".to_string())?;
            let output_path = payload["output_path"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'output_path'".to_string())?;
            let manifest = service
                .export_signed_bundle(triage_id, output_path)
                .map_err(|e| e.to_string())?;
            Ok(json!(manifest))
        }

        "verify_signed_bundle" => {
            let bundle_path = payload["bundle_path"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'bundle_path'".to_string())?;
            let report = service
                .verify_signed_bundle(bundle_path)
                .map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        "get_mitre_mapping" => {
            let triage_id = payload["triage_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'triage_id'".to_string())?;
            let mappings = service
                .get_mitre_mapping(triage_id)
                .map_err(|e| e.to_string())?;
            Ok(json!(mappings))
        }

        "parse_evtx" => {
            let path = payload["path"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'path'".to_string())?;
            let max_records = payload["max_records"].as_u64().unwrap_or(100) as usize;
            let records = service
                .parse_evtx_file(path, max_records)
                .map_err(|e| e.to_string())?;
            Ok(json!(records))
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
    fn test_ipc_workflow() {
        let stats = dispatch_ipc_command("get_dfir_stats", json!({})).unwrap();
        assert!(stats.get("total_triage_runs").is_some());

        let runs = dispatch_ipc_command("list_triage_runs", json!({})).unwrap();
        let arr = runs.as_array().unwrap();
        assert!(!arr.is_empty());
        let triage_id = arr[0]["triage_id"].as_str().unwrap();

        let pkg =
            dispatch_ipc_command("get_triage_package", json!({ "triage_id": triage_id })).unwrap();
        assert!(pkg.get("processes").is_some());

        let lic = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(lic.get("tier").is_some());

        // Test MITRE mapping IPC
        let mitre =
            dispatch_ipc_command("get_mitre_mapping", json!({ "triage_id": triage_id })).unwrap();
        assert!(mitre.as_array().is_some());

        // Test signed bundle export & verify IPC
        let temp_bundle = tempfile::NamedTempFile::new().unwrap();
        let export_res = dispatch_ipc_command(
            "export_signed_bundle",
            json!({
                "triage_id": triage_id,
                "output_path": temp_bundle.path().to_string_lossy()
            }),
        )
        .unwrap();
        assert!(export_res.get("ed25519_signature").is_some());

        let verify_res = dispatch_ipc_command(
            "verify_signed_bundle",
            json!({
                "bundle_path": temp_bundle.path().to_string_lossy()
            }),
        )
        .unwrap();
        assert_eq!(verify_res["is_valid"], true);
        assert_eq!(verify_res["signature_verified"], true);
    }
}
