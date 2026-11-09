//! IPC Command Dispatcher for Antigravity Offline Enterprise Multi-App Suite Launcher.

use crate::models::AppCategory;
use crate::service::LauncherService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<LauncherService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_service() -> Arc<LauncherService> {
    SERVICE
        .get_or_init(|| Arc::new(LauncherService::new()))
        .clone()
}

pub fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("antigravity-suite-launcher")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::AllAppsSuite, &data_dir)
    })
}

/// Dispatches an IPC command formatted as JSON
pub fn dispatch_ipc_command(action: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match action {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "list_apps" => {
            let cat = payload
                .get("category")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "enterprise_core" => Some(AppCategory::EnterpriseCore),
                    "creator_media" => Some(AppCategory::CreatorMedia),
                    "industrial_specialized" => Some(AppCategory::IndustrialSpecialized),
                    _ => None,
                });

            let list = service.list_apps(cat);
            Ok(json!(list))
        }

        "get_app" => {
            let app_id = payload.get("app_id").and_then(|v| v.as_str()).unwrap_or("");
            let app = service.get_app(app_id);
            Ok(json!(app))
        }

        "run_diagnostic" => {
            let app_id = payload
                .get("app_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing app_id".to_string())?;
            let res = service.run_diagnostic(app_id);
            Ok(json!(res))
        }

        "sign_suite_manifest" => {
            let manifest = service.sign_suite_manifest();
            Ok(json!(manifest))
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

        _ => Err(format!("Unknown action: {}", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_get_stats() {
        let res = dispatch_ipc_command("get_stats", json!({})).unwrap();
        assert_eq!(res["total_apps_registered"].as_u64().unwrap(), 23);
    }

    #[test]
    fn test_ipc_list_apps() {
        let res = dispatch_ipc_command("list_apps", json!({})).unwrap();
        let arr = res.as_array().unwrap();
        assert_eq!(arr.len(), 23);
    }

    #[test]
    fn test_ipc_get_app() {
        let res =
            dispatch_ipc_command("get_app", json!({ "app_id": "devops-secret-vault" })).unwrap();
        assert_eq!(res["id"].as_str().unwrap(), "devops-secret-vault");
        assert_eq!(res["name"].as_str().unwrap(), "DevOps Secret Vault");
    }

    #[test]
    fn test_ipc_run_diagnostic() {
        let res = dispatch_ipc_command(
            "run_diagnostic",
            json!({ "app_id": "universal-log-viewer" }),
        )
        .unwrap();
        assert!(res["passed"].as_bool().unwrap());
        assert_eq!(res["ipc_checks_passed"].as_u64().unwrap(), 7);
    }

    #[test]
    fn test_ipc_sign_suite_manifest() {
        let res = dispatch_ipc_command("sign_suite_manifest", json!({})).unwrap();
        assert_eq!(res["total_apps"].as_u64().unwrap(), 23);
        assert!(!res["ed25519_signature_hex"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_ipc_license_status() {
        let res = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(res["tier"].is_string());
    }
}
