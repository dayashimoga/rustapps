#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Offline QR Code Scanner & Generator Studio

use crate::models::{BatchQrItem, QrStyleConfig};
use crate::security_analyzer::SecurityAnalyzer;
use crate::service::QrStudioService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<QrStudioService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<QrStudioService> {
    SERVICE
        .get_or_init(|| {
            let data_dir = platform_common::AppPaths::from_app_name("qr-code-studio")
                .map(|p| p.data_dir)
                .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            let s = QrStudioService::open(&data_dir).unwrap_or_else(|_| {
                QrStudioService::open_in_memory().expect("In-memory QR studio service")
            });
            // Auto seed demo records if database is empty
            if let Ok(history) = s.list_history(None, false, 1) {
                if history.is_empty() {
                    let _ = s.seed_demo_records();
                }
            }
            Arc::new(s)
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("qr-code-studio")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::QrCodeStudio, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from Svelte 5 frontend or desktop/mobile FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_studio_stats" => {
            let stats = service.get_stats().map_err(|e| e.to_string())?;
            Ok(json!(stats))
        }

        "generate_qr" => {
            let raw_text = payload["raw_text"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'raw_text'".to_string())?;
            let label_opt = payload["label"].as_str();

            let style_opt: Option<QrStyleConfig> = if !payload["style"].is_null() {
                serde_json::from_value(payload["style"].clone()).ok()
            } else {
                None
            };

            let passphrase_opt = payload["passphrase"].as_str();
            let sign_ed25519 = payload["sign_ed25519"].as_bool().unwrap_or(false);

            let result = service
                .generate_qr(raw_text, label_opt, style_opt, passphrase_opt, sign_ed25519)
                .map_err(|e| e.to_string())?;
            Ok(json!(result))
        }

        "scan_qr_image" => {
            let png_bytes = if let Some(b64) = payload["png_base64"].as_str() {
                let clean_b64 = b64.strip_prefix("data:image/png;base64,").unwrap_or(b64);
                simple_base64_decode(clean_b64).ok()
            } else if let Some(path_str) = payload["file_path"].as_str() {
                std::fs::read(path_str).ok()
            } else {
                None
            };

            let passphrase_opt = payload["passphrase"].as_str();

            let result = service
                .scan_qr_image(png_bytes.as_deref(), None, passphrase_opt)
                .map_err(|e| e.to_string())?;
            Ok(json!(result))
        }

        "analyze_payload_security" => {
            let text = payload["payload"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'payload'".to_string())?;
            let report = SecurityAnalyzer::inspect_payload(text);
            Ok(json!(report))
        }

        "batch_generate_qr" => {
            let items: Vec<BatchQrItem> = serde_json::from_value(payload["items"].clone())
                .map_err(|e| format!("Invalid batch items: {}", e))?;
            let style: QrStyleConfig = if !payload["style"].is_null() {
                serde_json::from_value(payload["style"].clone()).unwrap_or_default()
            } else {
                QrStyleConfig::default()
            };

            let report = service
                .batch_generate(&items, &style)
                .map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        "list_history" => {
            let search_opt = payload["search_query"].as_str();
            let favs_only = payload["favorites_only"].as_bool().unwrap_or(false);
            let limit = payload["limit"].as_u64().unwrap_or(50) as usize;

            let history = service
                .list_history(search_opt, favs_only, limit)
                .map_err(|e| e.to_string())?;
            Ok(json!(history))
        }

        "toggle_favorite" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing parameter 'id'".to_string())?;
            let is_fav = service.toggle_favorite(id).map_err(|e| e.to_string())?;
            Ok(json!({ "id": id, "is_favorite": is_fav }))
        }

        "delete_history_item" => {
            let id = payload["id"]
                .as_str()
                .ok_or_else(|| "Missing parameter 'id'".to_string())?;
            let deleted = service.delete_history_item(id).map_err(|e| e.to_string())?;
            Ok(json!({ "id": id, "deleted": deleted }))
        }

        "get_license_status" => {
            let report: LicenseStatusReport = licensing.status_report();
            Ok(json!(report))
        }

        "activate_license" => {
            let key = payload["key"]
                .as_str()
                .ok_or_else(|| "Missing license key".to_string())?;
            let report = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        other => Err(format!("Unknown command '{}'", other)),
    }
}

/// Simple Base64 decoder
fn simple_base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean = input.trim().replace(['\r', '\n', ' '], "");
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0;

    for c in clean.chars() {
        if c == '=' {
            break;
        }
        let val = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(format!("Invalid base64 character '{}'", c)),
        };
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_ipc_command_flows() {
        let stats_val = dispatch_ipc_command("get_studio_stats", json!({})).expect("Get stats");
        assert!(stats_val["total_generated"].as_u64().is_some());

        let gen_val = dispatch_ipc_command(
            "generate_qr",
            json!({
                "raw_text": "https://rust-lang.org",
                "label": "Rust Homepage"
            }),
        )
        .expect("Generate QR");
        assert_eq!(gen_val["payload_type"], "Url");
        assert!(gen_val["svg_markup"].as_str().unwrap().starts_with("<svg"));
        assert!(gen_val["png_base64"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));

        let sec_val = dispatch_ipc_command(
            "analyze_payload_security",
            json!({
                "payload": "https://paypal.com@evil.phish/auth"
            }),
        )
        .expect("Analyze security");
        assert_eq!(sec_val["risk_level"], "Critical");
    }
}
