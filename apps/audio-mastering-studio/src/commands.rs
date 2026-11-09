#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Offline AI Stem Splitter & Audio Mastering Studio

use crate::dsp_engine::AudioDspEngine;
use crate::models::{MasteringConfig, WavExportConfig};
use crate::service::AudioMasteringService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<AudioMasteringService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<AudioMasteringService> {
    SERVICE
        .get_or_init(|| {
            let data_dir = platform_common::AppPaths::from_app_name("audio-mastering-studio")
                .map(|p| p.data_dir)
                .unwrap_or_else(|_| std::path::PathBuf::from("data"));
            let _ = std::fs::create_dir_all(&data_dir);
            let s = AudioMasteringService::open(&data_dir).unwrap_or_else(|_| {
                AudioMasteringService::open_in_memory().expect("In-memory mastering service")
            });
            // Auto seed a demo project if newly initialized
            if let Ok(projects) = s.list_projects() {
                if projects.is_empty() {
                    let _ = s.seed_demo_project();
                }
            }
            Arc::new(s)
        })
        .clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("audio-mastering-studio")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::AudioMasteringStudio, &data_dir)
    })
}

fn resolve_target_project_id(
    service: &AudioMasteringService,
    project_id: &str,
) -> Result<String, String> {
    if project_id.is_empty() || project_id == "proj-demo" || project_id == "latest" {
        let list = service.list_projects().map_err(|e| e.to_string())?;
        if let Some(first) = list.into_iter().next() {
            Ok(first.project_id)
        } else {
            let demo = service.seed_demo_project().map_err(|e| e.to_string())?;
            Ok(demo.project_id)
        }
    } else {
        Ok(project_id.to_string())
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

        "list_projects" => {
            let projects = service.list_projects().map_err(|e| e.to_string())?;
            Ok(json!(projects))
        }

        "get_project" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let project = service.get_project(&target_id).map_err(|e| e.to_string())?;
            Ok(json!(project))
        }

        "create_project" => {
            let title = payload["title"]
                .as_str()
                .unwrap_or("Studio Stem Separation & Mastering Session");
            let sample_rate = payload["sample_rate"]
                .as_u64()
                .map(|u| u as u32)
                .unwrap_or(44100);

            let samples: Vec<f32> = payload
                .get("samples")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| {
                    let num_samples = sample_rate as usize * 2;
                    AudioDspEngine::generate_synthetic_audio(num_samples, sample_rate)
                });

            let config: MasteringConfig = payload
                .get("config")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let project = service
                .create_project(title, samples, sample_rate, config)
                .map_err(|e| e.to_string())?;

            Ok(json!(project))
        }

        "remaster_project" | "update_mastering_config" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let config: MasteringConfig = payload
                .get("config")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let project = service
                .update_mastering_config(&target_id, config)
                .map_err(|e| e.to_string())?;

            Ok(json!(project))
        }

        "update_eq_band" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let band_index = payload["band_index"]
                .as_u64()
                .ok_or_else(|| "Missing required parameter 'band_index'".to_string())?
                as usize;
            let freq_hz = payload["frequency_hz"]
                .as_f64()
                .ok_or_else(|| "Missing parameter 'frequency_hz'".to_string())?
                as f32;
            let gain_db = payload["gain_db"]
                .as_f64()
                .ok_or_else(|| "Missing parameter 'gain_db'".to_string())?
                as f32;
            let q_factor = payload["q_factor"].as_f64().unwrap_or(1.0) as f32;

            let project = service
                .update_eq_band(&target_id, band_index, freq_hz, gain_db, q_factor)
                .map_err(|e| e.to_string())?;
            Ok(json!(project))
        }

        "get_eq_curve" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let num_points = payload["num_points"].as_u64().unwrap_or(64) as usize;

            let points = service
                .get_eq_curve(&target_id, num_points)
                .map_err(|e| e.to_string())?;
            Ok(json!(points))
        }

        "get_loudness_matched_audio" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let matched_samples = service
                .get_loudness_matched_audio(&target_id)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "matched_samples": matched_samples }))
        }

        "export_mastered_wav" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let config: WavExportConfig = payload
                .get("config")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let wav_bytes = service
                .export_mastered_wav(&target_id, &config)
                .map_err(|e| e.to_string())?;

            // Return byte length and sample count
            Ok(json!({
                "byte_length": wav_bytes.len(),
                "bit_depth": format!("{:?}", config.bit_depth),
                "apply_dither": config.apply_dither
            }))
        }

        "update_stem" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id = resolve_target_project_id(&service, project_id)?;
            let stem_index = payload["stem_index"].as_u64().unwrap_or(0) as usize;
            let gain_db = payload["gain_db"].as_f64().unwrap_or(0.0) as f32;
            let is_muted = payload["is_muted"].as_bool().unwrap_or(false);
            let is_solo = payload["is_solo"].as_bool().unwrap_or(false);

            let project = service
                .update_stem_track(&target_id, stem_index, gain_db, is_muted, is_solo)
                .map_err(|e| e.to_string())?;
            Ok(json!(project))
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

        other => Err(format!("Unknown mastering studio command: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_workflow_end_to_end() {
        let stats_val = dispatch_ipc_command("get_studio_stats", json!({})).expect("stats");
        assert!(stats_val.get("total_projects").is_some());

        let list_val = dispatch_ipc_command("list_projects", json!({})).expect("list");
        assert!(list_val.as_array().is_some());

        let get_val = dispatch_ipc_command("get_project", json!({ "project_id": "proj-demo" }))
            .expect("get_project");
        assert!(get_val.get("project_id").is_some());
        assert!(get_val.get("stems").is_some());
        let project_id = get_val["project_id"].as_str().unwrap();

        let remaster_val = dispatch_ipc_command(
            "remaster_project",
            json!({
                "project_id": project_id,
                "config": {
                    "target_lufs": -12.0
                }
            }),
        )
        .expect("remaster");
        assert!(remaster_val.get("mastered_loudness").is_some());

        // Test EQ curve IPC
        let curve_val = dispatch_ipc_command(
            "get_eq_curve",
            json!({
                "project_id": project_id,
                "num_points": 32
            }),
        )
        .expect("eq curve");
        assert_eq!(curve_val.as_array().unwrap().len(), 32);

        // Test EQ band update IPC
        let band_val = dispatch_ipc_command(
            "update_eq_band",
            json!({
                "project_id": project_id,
                "band_index": 2,
                "frequency_hz": 1500.0,
                "gain_db": 2.5,
                "q_factor": 1.4
            }),
        )
        .expect("update eq band");
        assert_eq!(band_val["mastering_config"]["bands"][2]["gain_db"], 2.5);

        // Test loudness matched audio IPC
        let matched_val = dispatch_ipc_command(
            "get_loudness_matched_audio",
            json!({
                "project_id": project_id
            }),
        )
        .expect("matched audio");
        assert!(!matched_val["matched_samples"]
            .as_array()
            .unwrap()
            .is_empty());

        // Test WAV export IPC
        let wav_val = dispatch_ipc_command(
            "export_mastered_wav",
            json!({
                "project_id": project_id,
                "config": {
                    "bit_depth": "Bit24",
                    "apply_dither": true,
                    "normalize_true_peak": true
                }
            }),
        )
        .expect("export wav");
        assert!(wav_val["byte_length"].as_u64().unwrap() > 44);

        let lic_val = dispatch_ipc_command("get_license_status", json!({})).expect("licensing");
        assert!(lic_val.get("tier").is_some());
    }
}
