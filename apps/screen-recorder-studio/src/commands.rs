#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Offline AI Screen Recorder & Tutorial Studio

use crate::models::{CursorEvent, ExportConfig, SubtitleCue};
use crate::service::RecorderService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<RecorderService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<RecorderService> {
    SERVICE
        .get_or_init(|| {
            let data_dir = platform_common::AppPaths::from_app_name("screen-recorder-studio")
                .map(|p| p.data_dir)
                .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            let s = RecorderService::open(&data_dir).unwrap_or_else(|_| {
                RecorderService::open_in_memory().expect("In-memory recorder service")
            });
            // Auto seed a demo walkthrough if database is newly initialized
            if let Ok(sessions) = s.list_sessions() {
                if sessions.is_empty() {
                    let _ = s.seed_demo_session();
                }
            }
            Arc::new(s)
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("screen-recorder-studio")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::ScreenRecorderStudio, &data_dir)
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

        "list_recordings" => {
            let sessions = service.list_sessions().map_err(|e| e.to_string())?;
            Ok(json!(sessions))
        }

        "get_recording" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let session =
                if session_id.is_empty() || session_id == "rec-demo" || session_id == "latest" {
                    let list = service.list_sessions().map_err(|e| e.to_string())?;
                    if let Some(first) = list.into_iter().next() {
                        first
                    } else {
                        service.seed_demo_session().map_err(|e| e.to_string())?
                    }
                } else {
                    service.get_session(session_id).map_err(|e| e.to_string())?
                };
            Ok(json!(session))
        }

        "create_recording" => {
            let title = payload["title"].as_str().unwrap_or("Untitled Recording");
            let duration_ms = payload["duration_ms"].as_u64().unwrap_or(10_000);
            let events: Vec<CursorEvent> =
                serde_json::from_value(payload["events"].clone()).unwrap_or_default();
            let subtitles: Vec<SubtitleCue> =
                serde_json::from_value(payload["subtitles"].clone()).unwrap_or_default();

            let session = service
                .create_recording_session(title, duration_ms, events, subtitles)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "adjust_auto_zooms" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id =
                if session_id.is_empty() || session_id == "rec-demo" || session_id == "latest" {
                    let list = service.list_sessions().map_err(|e| e.to_string())?;
                    list.into_iter()
                        .next()
                        .map(|s| s.session_id)
                        .unwrap_or_else(|| "rec-demo".to_string())
                } else {
                    session_id.to_string()
                };
            let scale = payload["scale"].as_f64().unwrap_or(1.45) as f32;
            let session = service
                .adjust_auto_zooms(&target_id, scale)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "export_tutorial" => {
            let session_id = payload["session_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'session_id'".to_string())?;
            let config: ExportConfig =
                serde_json::from_value(payload["config"].clone()).unwrap_or_default();
            let content = service
                .export_tutorial(session_id, &config)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "content": content }))
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
        let stats = dispatch_ipc_command("get_studio_stats", json!({})).unwrap();
        assert!(stats.get("total_recordings").is_some());

        let list = dispatch_ipc_command("list_recordings", json!({})).unwrap();
        let arr = list.as_array().unwrap();
        assert!(!arr.is_empty());
        let session_id = arr[0]["session_id"].as_str().unwrap();

        let rec =
            dispatch_ipc_command("get_recording", json!({ "session_id": session_id })).unwrap();
        assert_eq!(rec["session_id"].as_str().unwrap(), session_id);

        let export = dispatch_ipc_command(
            "export_tutorial",
            json!({
                "session_id": session_id,
                "config": {
                    "format": "InteractiveHtml",
                    "include_auto_zoom": true,
                    "trim_dead_air": true,
                    "burn_subtitles": true,
                    "target_width": 1920,
                    "target_height": 1080
                }
            }),
        )
        .unwrap();
        assert!(export["content"]
            .as_str()
            .unwrap()
            .contains("<!DOCTYPE html>"));

        let lic = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(lic.get("tier").is_some());
    }
}
