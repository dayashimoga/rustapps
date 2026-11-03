#![deny(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for Professional Audio & Video Editor
//!
//! Provides typed command interfaces invoked by the desktop Svelte/TypeScript studio UI.

use crate::audio_engine::LoudnessMeterData;
use crate::color::VideoScopeData;
use crate::models::{Clip, TimelineProject};
use crate::service::EditorService;
use crate::templates::{ExportPreset, ProjectTemplate};
use std::sync::{Arc, Mutex, OnceLock};

static SERVICE: OnceLock<Arc<Mutex<EditorService>>> = OnceLock::new();

pub fn get_service() -> Arc<Mutex<EditorService>> {
    SERVICE
        .get_or_init(|| Arc::new(Mutex::new(EditorService::new())))
        .clone()
}

/// Retrieve full current timeline project state
pub fn get_project_state_cmd() -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Create a new project optionally from a pre-configured template
pub fn create_project_cmd(
    name: String,
    template_id: Option<String>,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let mut svc = service.lock().map_err(|e| e.to_string())?;

    if let Some(tid) = template_id {
        *svc = EditorService::from_template(&tid, &name).map_err(|e| e.to_string())?;
    } else {
        *svc = EditorService::new();
    }

    svc.get_project().map_err(|e| e.to_string())
}

/// Save project to `.avedproj` atomic file
pub fn save_project_cmd(path: String) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.save_project(path).map_err(|e| e.to_string())
}

/// Load project from `.avedproj` atomic file
pub fn load_project_cmd(path: String) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.load_project(path).map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Add clip to timeline
pub fn add_clip_cmd(clip: Clip) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.add_clip(clip).map_err(|e| e.to_string())
}

/// Remove clip from timeline
pub fn remove_clip_cmd(clip_id: String) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.remove_clip(&clip_id).map_err(|e| e.to_string())
}

/// Split clip at specified timestamp
pub fn split_clip_cmd(clip_id: String, timestamp_ms: u64) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.split_clip(&clip_id, timestamp_ms)
        .map_err(|e| e.to_string())
}

/// Trim clip in/out points
pub fn trim_clip_cmd(clip_id: String, new_in_ms: u64, new_out_ms: u64) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.trim_clip(&clip_id, new_in_ms, new_out_ms)
        .map_err(|e| e.to_string())
}

/// Ripple delete clip and shift subsequent clips left
pub fn ripple_delete_cmd(clip_id: String) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.ripple_delete(&clip_id).map_err(|e| e.to_string())
}

/// Undo previous timeline action
pub fn undo_cmd() -> Result<bool, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.undo().map_err(|e| e.to_string())
}

/// Redo previously undone timeline action
pub fn redo_cmd() -> Result<bool, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.redo().map_err(|e| e.to_string())
}

/// Render frame at timestamp and return base64-encoded standard BMP image for preview
pub fn render_preview_frame_cmd(
    timestamp_ms: u64,
    width: u32,
    height: u32,
) -> Result<String, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let frame = svc
        .render_frame(timestamp_ms, width, height)
        .map_err(|e| e.to_string())?;
    let bmp_bytes = frame.encode_to_bmp();

    // Base64 encode
    let mut enc = Vec::new();
    {
        // Custom simple base64 or blake3 format
        // Standard base64 implementation using fast lookup table
        const B64_CHARS: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for chunk in bmp_bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

            enc.push(B64_CHARS[(b0 >> 2) as usize]);
            enc.push(B64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
            if chunk.len() > 1 {
                enc.push(B64_CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
            } else {
                enc.push(b'=');
            }
            if chunk.len() > 2 {
                enc.push(B64_CHARS[(b2 & 0x3f) as usize]);
            } else {
                enc.push(b'=');
            }
        }
    }
    String::from_utf8(enc).map_err(|e| e.to_string())
}

/// Compute broadcast video scopes (Histogram, Waveform, Vectorscope)
pub fn get_video_scopes_cmd(
    timestamp_ms: u64,
    width: u32,
    height: u32,
) -> Result<VideoScopeData, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.get_video_scopes(timestamp_ms, width, height)
        .map_err(|e| e.to_string())
}

/// Measure EBU R128 audio loudness readings
pub fn get_audio_loudness_cmd(start_ms: u64, end_ms: u64) -> Result<LoudnessMeterData, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let (_, readings) = svc
        .render_audio(start_ms, end_ms)
        .map_err(|e| e.to_string())?;
    Ok(readings)
}

/// AI Silence Ripple-Cutter command
pub fn auto_cut_silence_cmd(threshold_dbfs: f32) -> Result<usize, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let config = crate::ai_editor::SilenceCutConfig {
        threshold_dbfs,
        ..Default::default()
    };
    svc.auto_cut_silence(&config).map_err(|e| e.to_string())
}

/// AI Beat & Transient Detector command
pub fn detect_beats_cmd(sensitivity: f32) -> Result<usize, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.detect_and_add_beat_markers(sensitivity)
        .map_err(|e| e.to_string())
}

/// Export timeline to CMX 3600 EDL
pub fn export_edl_cmd() -> Result<String, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.export_edl().map_err(|e| e.to_string())
}

/// Export timeline to Final Cut Pro XML
pub fn export_fcpxml_cmd() -> Result<String, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.export_fcpxml().map_err(|e| e.to_string())
}

/// Retrieve all available project templates
pub fn get_templates_cmd() -> Result<Vec<ProjectTemplate>, String> {
    Ok(ProjectTemplate::all_templates())
}

/// Retrieve all available export presets
pub fn get_export_presets_cmd() -> Result<Vec<ExportPreset>, String> {
    Ok(ExportPreset::all_standard_presets())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands_workflow() {
        let proj = get_project_state_cmd().unwrap();
        assert!(!proj.name.is_empty());

        let tpls = get_templates_cmd().unwrap();
        assert_eq!(tpls.len(), 4);

        let presets = get_export_presets_cmd().unwrap();
        assert_eq!(presets.len(), 4);

        let preview_b64 = render_preview_frame_cmd(0, 64, 36).unwrap();
        assert!(!preview_b64.is_empty());

        let scopes = get_video_scopes_cmd(0, 64, 36).unwrap();
        assert_eq!(scopes.histogram_r.len(), 256);
    }
}
