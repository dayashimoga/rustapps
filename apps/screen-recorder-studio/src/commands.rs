#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Offline AI Screen Recorder & Tutorial Studio

use crate::models::{
    CursorEvent, ExportConfig, KeystrokeEvent, RedactionRegion, SubtitleCue, TextRegion,
    ZoomKeyframe,
};
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

fn resolve_target_session_id(
    service: &RecorderService,
    session_id: &str,
) -> Result<String, String> {
    if session_id.is_empty() || session_id == "rec-demo" || session_id == "latest" {
        let list = service.list_sessions().map_err(|e| e.to_string())?;
        if let Some(first) = list.into_iter().next() {
            Ok(first.session_id)
        } else {
            let demo = service.seed_demo_session().map_err(|e| e.to_string())?;
            Ok(demo.session_id)
        }
    } else {
        Ok(session_id.to_string())
    }
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
            let target_id = resolve_target_session_id(&service, session_id)?;
            let session = service.get_session(&target_id).map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "create_recording" => {
            let title = payload["title"].as_str().unwrap_or("Untitled Recording");
            let duration_ms = payload["duration_ms"].as_u64().unwrap_or(10_000);
            let events: Vec<CursorEvent> =
                serde_json::from_value(payload["events"].clone()).unwrap_or_default();
            let keystrokes: Vec<KeystrokeEvent> = serde_json::from_value(
                payload
                    .get("keystroke_events")
                    .or_else(|| payload.get("keystrokes"))
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or_default();
            let subtitles: Vec<SubtitleCue> =
                serde_json::from_value(payload["subtitles"].clone()).unwrap_or_default();

            let session = service
                .create_recording_session(title, duration_ms, events, keystrokes, subtitles)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "adjust_auto_zooms" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id = resolve_target_session_id(&service, session_id)?;
            let scale = payload["scale"].as_f64().unwrap_or(1.45) as f32;
            let session = service
                .adjust_auto_zooms(&target_id, scale)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "add_zoom_keyframe" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id = resolve_target_session_id(&service, session_id)?;
            let keyframe: ZoomKeyframe = serde_json::from_value(payload["keyframe"].clone())
                .map_err(|e| format!("Invalid ZoomKeyframe payload: {}", e))?;
            let session = service
                .add_zoom_keyframe(&target_id, keyframe)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "delete_zoom_keyframe" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id = resolve_target_session_id(&service, session_id)?;
            let start_ms = payload["start_ms"]
                .as_u64()
                .ok_or_else(|| "Missing required parameter 'start_ms'".to_string())?;
            let session = service
                .delete_zoom_keyframe(&target_id, start_ms)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "add_redaction_region" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id = resolve_target_session_id(&service, session_id)?;
            let region: RedactionRegion = serde_json::from_value(payload["region"].clone())
                .map_err(|e| format!("Invalid RedactionRegion payload: {}", e))?;
            let session = service
                .add_redaction_region(&target_id, region)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "delete_redaction_region" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id = resolve_target_session_id(&service, session_id)?;
            let region_id = payload["region_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'region_id'".to_string())?;
            let session = service
                .delete_redaction_region(&target_id, region_id)
                .map_err(|e| e.to_string())?;
            Ok(json!(session))
        }

        "detect_pii_redactions" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let target_id = resolve_target_session_id(&service, session_id)?;
            let text_regions: Vec<TextRegion> =
                serde_json::from_value(payload["text_regions"].clone()).unwrap_or_default();
            let detected = service
                .detect_pii_redactions_for_session(&target_id, &text_regions)
                .map_err(|e| e.to_string())?;
            let session = service.get_session(&target_id).map_err(|e| e.to_string())?;
            Ok(json!({
                "detected": detected,
                "session": session
            }))
        }

        "export_tutorial" => {
            let session_id = payload["session_id"]
                .as_str()
                .ok_or_else(|| "Missing required parameter 'session_id'".to_string())?;
            let target_id = resolve_target_session_id(&service, session_id)?;
            let config: ExportConfig =
                serde_json::from_value(payload["config"].clone()).unwrap_or_default();
            let content = service
                .export_tutorial(&target_id, &config)
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
        assert!(rec.get("keystroke_events").is_some());

        // Test manual zoom keyframe IPC
        let zoom_res = dispatch_ipc_command(
            "add_zoom_keyframe",
            json!({
                "session_id": session_id,
                "keyframe": {
                    "start_ms": 11_000,
                    "end_ms": 11_800,
                    "focus_x": 0.5,
                    "focus_y": 0.5,
                    "scale": 1.7,
                    "easing": "ease-in-out",
                    "label": "Detail Zoom"
                }
            }),
        )
        .unwrap();
        assert!(zoom_res["zoom_keyframes"].as_array().unwrap().len() >= 5);

        let del_zoom = dispatch_ipc_command(
            "delete_zoom_keyframe",
            json!({
                "session_id": session_id,
                "start_ms": 11_000
            }),
        )
        .unwrap();
        assert_eq!(
            del_zoom["zoom_keyframes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|k| k["start_ms"] == 11_000),
            None
        );

        // Test PII redaction detection IPC
        let pii_res = dispatch_ipc_command(
            "detect_pii_redactions",
            json!({
                "session_id": session_id,
                "text_regions": [
                    {
                        "text": "AWS Key: AKIAIOSFODNN7EXAMPLE",
                        "x": 0.15,
                        "y": 0.25,
                        "width": 0.35,
                        "height": 0.05
                    }
                ]
            }),
        )
        .unwrap();
        assert!(!pii_res["detected"].as_array().unwrap().is_empty());

        // Test manual redaction region IPC
        let add_red = dispatch_ipc_command(
            "add_redaction_region",
            json!({
                "session_id": session_id,
                "region": {
                    "id": "redact-ipc-test",
                    "label": "Manual Mask",
                    "x": 0.2,
                    "y": 0.3,
                    "width": 0.1,
                    "height": 0.1,
                    "pattern_type": "custom"
                }
            }),
        )
        .unwrap();
        assert!(add_red["redaction_regions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "redact-ipc-test"));

        let del_red = dispatch_ipc_command(
            "delete_redaction_region",
            json!({
                "session_id": session_id,
                "region_id": "redact-ipc-test"
            }),
        )
        .unwrap();
        assert!(!del_red["redaction_regions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "redact-ipc-test"));

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
