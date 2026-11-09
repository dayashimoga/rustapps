use crate::service::SbomService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<SbomService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<SbomService> {
    SERVICE.get_or_init(|| Arc::new(SbomService::new())).clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("sbom-supply-chain-auditor")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::SbomSupplyChainAuditor, &data_dir)
    })
}

/// Dispatches JSON IPC commands from desktop Svelte 5 frontend and test harnesses
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "get_sbom" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-cyclonedx");
            let doc = service
                .get_sbom(doc_id)
                .map_err(|e| format!("Failed to get SBOM: {}", e))?;
            Ok(json!(doc))
        }

        "load_sbom" => {
            let filename = payload
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("bom.cdx.json");

            let raw_bytes = if let Some(text) = payload.get("content_text").and_then(|v| v.as_str())
            {
                text.as_bytes().to_vec()
            } else if let Some(b64) = payload.get("content_base64").and_then(|v| v.as_str()) {
                b64.as_bytes().to_vec()
            } else {
                return Err("Missing 'content_text' or 'content_base64' in payload".to_string());
            };

            let doc = service
                .load_sbom(filename, &raw_bytes)
                .map_err(|e| format!("Failed to load SBOM: {}", e))?;
            Ok(json!(doc))
        }

        "get_audit_report" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-cyclonedx");
            let report = service
                .get_audit_report(doc_id)
                .map_err(|e| format!("Failed to get audit report: {}", e))?;
            Ok(json!(report))
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
            "Unknown sbom-supply-chain-auditor command: {}",
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
        assert!(stats_val.get("total_sboms_audited").is_some());

        // 2. Get demo SBOM
        let sbom_val =
            dispatch_ipc_command("get_sbom", json!({ "document_id": "doc-demo-cyclonedx" }))
                .expect("get_sbom");
        assert_eq!(sbom_val["total_components"], 8);
        assert!(sbom_val["ntia_minimum_elements_compliant"]
            .as_bool()
            .unwrap());

        // 3. Get audit report
        let report_val = dispatch_ipc_command(
            "get_audit_report",
            json!({ "document_id": "doc-demo-cyclonedx" }),
        )
        .expect("get_audit_report");
        assert_eq!(report_val["critical_cves"], 2);
        assert_eq!(report_val["copyleft_violations"], 1);

        // 4. License status
        let lic_val =
            dispatch_ipc_command("get_license_status", json!({})).expect("get_license_status");
        assert!(lic_val.get("tier").is_some());
    }
}
