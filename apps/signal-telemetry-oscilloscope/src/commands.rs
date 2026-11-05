#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Offline Scientific Signal & Telemetry Oscilloscope

use crate::dsp_engine::SignalDspEngine;
use crate::models::{TriggerConfig, WaveformType};
use crate::service::OscilloscopeService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<OscilloscopeService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<OscilloscopeService> {
    SERVICE
        .get_or_init(|| {
            let data_dir =
                platform_common::AppPaths::from_app_name("signal-telemetry-oscilloscope")
                    .map(|p| p.data_dir)
                    .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            let s = OscilloscopeService::open(&data_dir).unwrap_or_else(|_| {
                OscilloscopeService::open_in_memory().expect("In-memory oscilloscope service")
            });
            // Auto seed a demo capture session if newly initialized
            if let Ok(captures) = s.list_captures() {
                if captures.is_empty() {
                    let _ = s.seed_demo_capture();
                }
            }
            Arc::new(s)
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("signal-telemetry-oscilloscope")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::SignalTelemetryOscilloscope, &data_dir)
    })
}

/// Dispatches JSON-RPC styled IPC commands from Svelte 5 frontend or desktop/mobile FFI
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_oscilloscope_stats" => {
            let stats = service.get_stats().map_err(|e| e.to_string())?;
            Ok(json!(stats))
        }

        "list_captures" => {
            let captures = service.list_captures().map_err(|e| e.to_string())?;
            Ok(json!(captures))
        }

        "get_capture" => {
            let session_id = payload["session_id"].as_str().unwrap_or("");
            let session =
                if session_id.is_empty() || session_id == "scope-demo" || session_id == "latest" {
                    let list = service.list_captures().map_err(|e| e.to_string())?;
                    if let Some(first) = list.into_iter().next() {
                        first
                    } else {
                        service.seed_demo_capture().map_err(|e| e.to_string())?
                    }
                } else {
                    service.get_capture(session_id).map_err(|e| e.to_string())?
                };
            Ok(json!(session))
        }

        "acquire_capture" => {
            let title = payload["title"]
                .as_str()
                .unwrap_or("Oscilloscope Laboratory Sweep");
            let sample_rate_hz = payload["sample_rate_hz"]
                .as_f64()
                .map(|f| f as f32)
                .unwrap_or(50_000.0);
            let num_samples = payload["num_samples"]
                .as_u64()
                .map(|u| u as usize)
                .unwrap_or(1000);

            let ch1_type: WaveformType = payload
                .get("ch1_type")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or(WaveformType::Sine);
            let ch1_freq = payload["ch1_freq"]
                .as_f64()
                .map(|f| f as f32)
                .unwrap_or(1000.0);
            let ch1_amp = payload["ch1_amp"].as_f64().map(|f| f as f32).unwrap_or(3.3);

            let ch2_type: WaveformType = payload
                .get("ch2_type")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or(WaveformType::Square);
            let ch2_freq = payload["ch2_freq"]
                .as_f64()
                .map(|f| f as f32)
                .unwrap_or(500.0);
            let ch2_amp = payload["ch2_amp"].as_f64().map(|f| f as f32).unwrap_or(3.3);

            let trigger: TriggerConfig = payload
                .get("trigger")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let session = service
                .acquire_capture(
                    title,
                    sample_rate_hz,
                    num_samples,
                    ch1_type,
                    ch1_freq,
                    ch1_amp,
                    ch2_type,
                    ch2_freq,
                    ch2_amp,
                    &trigger,
                )
                .map_err(|e| e.to_string())?;

            Ok(json!(session))
        }

        "decode_protocol" => {
            let sample_rate_hz = payload["sample_rate_hz"]
                .as_f64()
                .map(|f| f as f32)
                .unwrap_or(50_000.0);
            let baud_rate = payload["baud_rate"]
                .as_u64()
                .map(|u| u as u32)
                .unwrap_or(9600);

            let digital_samples: Vec<u8> = payload
                .get("digital_samples")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| {
                    // Fallback to latest session digital samples if not provided
                    service
                        .list_captures()
                        .ok()
                        .and_then(|list| list.into_iter().next())
                        .map(|s| s.digital_samples)
                        .unwrap_or_default()
                });

            let packets = SignalDspEngine::decode_uart(&digital_samples, sample_rate_hz, baud_rate);
            Ok(json!(packets))
        }

        "get_license_status" => {
            let status: LicenseStatusReport = licensing.status_report();
            Ok(json!(status))
        }

        "activate_license" => {
            let key = payload["license_key"]
                .as_str()
                .or_else(|| payload["license_json"].as_str())
                .ok_or_else(|| "Missing required parameter 'license_key'".to_string())?;
            let status = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(status))
        }

        other => Err(format!("Unknown oscilloscope command: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_workflow_end_to_end() {
        let stats_val = dispatch_ipc_command("get_oscilloscope_stats", json!({})).expect("stats");
        assert!(stats_val.get("total_captures").is_some());

        let list_val = dispatch_ipc_command("list_captures", json!({})).expect("list");
        assert!(list_val.as_array().is_some());

        let get_val = dispatch_ipc_command("get_capture", json!({ "session_id": "scope-demo" }))
            .expect("get_capture");
        assert!(get_val.get("session_id").is_some());
        assert!(get_val.get("spectrum").is_some());

        let acquire_val = dispatch_ipc_command(
            "acquire_capture",
            json!({
                "title": "IPC Test Sweep",
                "sample_rate_hz": 48_000.0,
                "num_samples": 512,
                "ch1_freq": 1000.0,
                "ch1_amp": 2.5
            }),
        )
        .expect("acquire");
        assert_eq!(acquire_val["title"], "IPC Test Sweep");

        let lic_val = dispatch_ipc_command("get_license_status", json!({})).expect("licensing");
        assert!(lic_val.get("tier").is_some());
    }
}
