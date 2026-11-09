//! IPC Command Dispatcher for Forensic Air-Gapped Media Privacy Sanitizer.

use crate::service::SanitizerService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<SanitizerService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_service() -> Arc<SanitizerService> {
    SERVICE
        .get_or_init(|| Arc::new(SanitizerService::new()))
        .clone()
}

pub fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("privacy-media-sanitizer")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::PrivacyMediaSanitizer, &data_dir)
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

        "list_media" => {
            let list = service.list_media();
            Ok(json!(list))
        }

        "get_media_report" => {
            let file_id = payload
                .get("file_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let report = service.get_media_report(file_id);
            Ok(json!(report))
        }

        "inspect_upload" => {
            let filename = payload
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_file.bin");

            let raw_bytes = if let Some(b64) = payload.get("bytes_base64").and_then(|v| v.as_str())
            {
                hex::decode(b64).unwrap_or_else(|_| b64.as_bytes().to_vec())
            } else if let Some(text) = payload.get("raw_text").and_then(|v| v.as_str()) {
                text.as_bytes().to_vec()
            } else {
                return Err("Missing bytes_base64 or raw_text payload".to_string());
            };

            let report = service.inspect_upload(filename, &raw_bytes);
            Ok(json!(report))
        }

        "sanitize_file" => {
            let file_id = payload
                .get("file_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing file_id".to_string())?;

            let result = service.sanitize_file(file_id)?;
            Ok(json!(result))
        }

        "get_sanitized_result" => {
            let file_id = payload
                .get("file_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = service.get_sanitized_result(file_id);
            Ok(json!(result))
        }

        "generate_privacy_certificate" => {
            let file_id = payload
                .get("file_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let cert = service.sign_privacy_certificate(file_id)?;
            Ok(json!(cert))
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
            "Unknown privacy sanitizer IPC command: '{}'",
            action
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_commands_end_to_end() {
        // 1. Get stats
        let stats_val = dispatch_ipc_command("get_stats", json!({})).expect("get_stats");
        assert!(stats_val.get("total_files_sanitized").is_some());
        assert!(stats_val["total_files_sanitized"].as_u64().unwrap() >= 1);

        // 2. List media
        let list_val = dispatch_ipc_command("list_media", json!({})).expect("list_media");
        assert!(list_val.as_array().unwrap().len() >= 3);

        // 3. Get media report
        let rep_val = dispatch_ipc_command(
            "get_media_report",
            json!({ "file_id": "FILE-IPHONE-PHOTO" }),
        )
        .expect("get_media_report");
        assert_eq!(rep_val["file_id"], "FILE-IPHONE-PHOTO");
        assert!(rep_val["metadata_tags"].as_array().unwrap().len() >= 3);
        assert_eq!(rep_val["has_gps"], true);

        // 4. Sanitize file
        let san_val =
            dispatch_ipc_command("sanitize_file", json!({ "file_id": "FILE-LEGAL-BRIEF" }))
                .expect("sanitize_file");
        assert_eq!(san_val["file_id"], "FILE-LEGAL-BRIEF");
        assert_eq!(san_val["gps_purged"], true);

        // 5. Get sanitized result
        let res_val = dispatch_ipc_command(
            "get_sanitized_result",
            json!({ "file_id": "FILE-LEGAL-BRIEF" }),
        )
        .expect("get_sanitized_result");
        assert_eq!(res_val["file_id"], "FILE-LEGAL-BRIEF");

        // 6. Sign privacy certificate
        let cert_val = dispatch_ipc_command(
            "generate_privacy_certificate",
            json!({ "file_id": "FILE-LEGAL-BRIEF" }),
        )
        .expect("generate_privacy_certificate");
        assert!(cert_val["ed25519_signature_hex"].as_str().is_some());

        // 7. License Status
        let lic_val =
            dispatch_ipc_command("get_license_status", json!({})).expect("get_license_status");
        assert!(lic_val.get("tier").is_some());
    }
}
