use crate::certificate::CertificateManager;
use crate::models::{BatesConfig, BoundingBox, BurnStyle, SanitizationCertificate};
use crate::service::RedactionService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<RedactionService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<RedactionService> {
    SERVICE
        .get_or_init(|| Arc::new(RedactionService::new()))
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("clean-room-redactor")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::CleanRoomRedactor, &data_dir)
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

        "get_document" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-settlement");
            let doc = service
                .get_document(doc_id)
                .map_err(|e| format!("Failed to get document: {}", e))?;
            Ok(json!(doc))
        }

        "load_document" => {
            let filename = payload
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("document.pdf");

            let raw_bytes = if let Some(text) = payload.get("content_text").and_then(|v| v.as_str())
            {
                text.as_bytes().to_vec()
            } else if let Some(b64) = payload.get("content_base64").and_then(|v| v.as_str()) {
                b64.as_bytes().to_vec()
            } else {
                return Err("Missing 'content_text' or 'content_base64' in payload".to_string());
            };

            let doc = service
                .load_document(filename, &raw_bytes)
                .map_err(|e| format!("Failed to load document: {}", e))?;
            Ok(json!(doc))
        }

        "add_redaction" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-settlement");
            let page_idx = payload
                .get("page_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let bbox = if let Some(b) = payload.get("bbox") {
                serde_json::from_value::<BoundingBox>(b.clone()).unwrap_or_default()
            } else {
                BoundingBox::default()
            };

            let label = payload
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("Manual Redaction")
                .to_string();

            let burn_style = payload
                .get("burn_style")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "solid_white" => BurnStyle::SolidWhite,
                    "pattern_hatch" => BurnStyle::PatternHatch,
                    "exemption_code" => BurnStyle::ExemptionCode,
                    _ => BurnStyle::SolidBlack,
                })
                .unwrap_or(BurnStyle::SolidBlack);

            let exemption_code = payload
                .get("exemption_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mask = service
                .add_redaction(doc_id, page_idx, bbox, label, burn_style, exemption_code)
                .map_err(|e| format!("Failed to add redaction: {}", e))?;

            Ok(json!(mask))
        }

        "remove_redaction" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-settlement");
            let page_idx = payload
                .get("page_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let mask_id = payload
                .get("mask_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing mask_id".to_string())?;

            let removed = service
                .remove_redaction(doc_id, page_idx, mask_id)
                .map_err(|e| format!("Failed to remove redaction: {}", e))?;

            Ok(json!({ "removed": removed }))
        }

        "toggle_pii_approval" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-settlement");
            let page_idx = payload
                .get("page_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let finding_id = payload
                .get("finding_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing finding_id".to_string())?;
            let approved = payload
                .get("approved")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let res = service
                .toggle_pii_approval(doc_id, page_idx, finding_id, approved)
                .map_err(|e| format!("Failed to toggle PII approval: {}", e))?;

            Ok(json!({ "success": res }))
        }

        "apply_clean_room_redaction" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("doc-demo-settlement");

            let bates_cfg = payload
                .get("bates_config")
                .and_then(|v| serde_json::from_value::<BatesConfig>(v.clone()).ok());

            let cert = service
                .apply_clean_room_redaction(doc_id, bates_cfg)
                .map_err(|e| format!("Failed to apply clean-room redactions: {}", e))?;

            Ok(json!(cert))
        }

        "get_certificate_svg" => {
            let cert: SanitizationCertificate = serde_json::from_value(payload)
                .map_err(|e| format!("Invalid SanitizationCertificate payload: {}", e))?;
            let svg = CertificateManager::render_certificate_svg(&cert);
            Ok(json!({ "svg": svg }))
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
            "Unknown clean-room-redactor IPC command: {}",
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
        assert!(stats_val.get("total_documents_sanitized").is_some());

        // 2. Get demo document
        let doc_val = dispatch_ipc_command(
            "get_document",
            json!({ "document_id": "doc-demo-settlement" }),
        )
        .expect("get_document");
        assert_eq!(doc_val["total_pages"], 3);
        let pii_findings = doc_val["pages"][0]["pii_findings"].as_array().unwrap();
        assert!(!pii_findings.is_empty());

        // 3. Add manual redaction
        let mask_val = dispatch_ipc_command(
            "add_redaction",
            json!({
                "document_id": "doc-demo-settlement",
                "page_index": 0,
                "label": "Settlement Amount",
                "burn_style": "exemption_code",
                "exemption_code": "(b)(4)"
            }),
        )
        .expect("add_redaction");
        assert_eq!(mask_val["label"], "Settlement Amount");
        let mask_id = mask_val["id"].as_str().unwrap();

        // 4. Toggle PII approval
        let finding_id = pii_findings[0]["id"].as_str().unwrap();
        let toggle_val = dispatch_ipc_command(
            "toggle_pii_approval",
            json!({
                "document_id": "doc-demo-settlement",
                "page_index": 0,
                "finding_id": finding_id,
                "approved": false
            }),
        )
        .expect("toggle_pii_approval");
        assert_eq!(toggle_val["success"], true);

        // 5. Apply clean-room redaction
        let cert_val = dispatch_ipc_command(
            "apply_clean_room_redaction",
            json!({
                "document_id": "doc-demo-settlement",
                "bates_config": {
                    "prefix": "LIT-PROD-",
                    "suffix": "",
                    "start_number": 1,
                    "digits": 6,
                    "position": "bottom_right",
                    "font_size": 10,
                    "include_date": false
                }
            }),
        )
        .expect("apply_clean_room_redaction");
        assert_eq!(cert_val["verification_status"], "VERIFIED_AUTHENTIC");
        assert!(cert_val["total_redactions_applied"].as_u64().unwrap() >= 1);

        // 6. Get Certificate SVG
        let svg_val = dispatch_ipc_command("get_certificate_svg", cert_val.clone())
            .expect("get_certificate_svg");
        assert!(svg_val["svg"]
            .as_str()
            .unwrap()
            .contains("OFFLINE CRYPTOGRAPHIC SANITIZATION CERTIFICATE"));

        // 7. Remove manual mask
        let rem_val = dispatch_ipc_command(
            "remove_redaction",
            json!({
                "document_id": "doc-demo-settlement",
                "page_index": 0,
                "mask_id": mask_id
            }),
        )
        .expect("remove_redaction");
        assert_eq!(rem_val["removed"], true);

        // 8. License check
        let lic_val =
            dispatch_ipc_command("get_license_status", json!({})).expect("get_license_status");
        assert!(lic_val.get("tier").is_some());
    }
}
