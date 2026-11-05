#![forbid(unsafe_code)]
//! IPC Command Dispatcher for Offline AI Stem Splitter & Audio Mastering Studio

use crate::dsp_engine::AudioDspEngine;
use crate::models::MasteringConfig;
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
            let project =
                if project_id.is_empty() || project_id == "proj-demo" || project_id == "latest" {
                    let list = service.list_projects().map_err(|e| e.to_string())?;
                    if let Some(first) = list.into_iter().next() {
                        first
                    } else {
                        service.seed_demo_project().map_err(|e| e.to_string())?
                    }
                } else {
                    service.get_project(project_id).map_err(|e| e.to_string())?
                };
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

        "remaster_project" => {
            let project_id = payload["project_id"].as_str().unwrap_or("");
            let target_id =
                if project_id.is_empty() || project_id == "proj-demo" || project_id == "latest" {
                    let list = service.list_projects().map_err(|e| e.to_string())?;
                    list.into_iter()
                        .next()
                        .map(|p| p.project_id)
                        .unwrap_or_else(|| "proj-demo".to_string())
                } else {
                    project_id.to_string()
                };

            let config: MasteringConfig = payload
                .get("config")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let project = service
                .remaster_project(&target_id, config)
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

        let remaster_val = dispatch_ipc_command(
            "remaster_project",
            json!({
                "project_id": "proj-demo",
                "config": {
                    "target_lufs": -12.0
                }
            }),
        )
        .expect("remaster");
        assert!(remaster_val.get("mastered_loudness").is_some());

        let lic_val = dispatch_ipc_command("get_license_status", json!({})).expect("licensing");
        assert!(lic_val.get("tier").is_some());
    }
}
