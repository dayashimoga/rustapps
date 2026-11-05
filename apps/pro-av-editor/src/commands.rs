#![deny(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for Professional Audio & Video Editor
//!
//! Provides typed command interfaces invoked by the desktop Svelte/TypeScript studio UI.

use crate::audio_engine::LoudnessMeterData;
use crate::color::VideoScopeData;
use crate::models::{
    AudioSyncResult, BlendMode, ChromaKeySettings, Clip, ColorGradingSettings, FrameRate,
    MasterExportSummary, ProxyGenerationResult, TimelineProject, TrackType, Transform2D,
};
use crate::service::EditorService;
use crate::templates::{ExportPreset, ProjectTemplate};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};

static SERVICE: OnceLock<Arc<Mutex<EditorService>>> = OnceLock::new();

pub fn get_service() -> Arc<Mutex<EditorService>> {
    SERVICE
        .get_or_init(|| Arc::new(Mutex::new(EditorService::with_studio_starter())))
        .clone()
}

/// Retrieve full current timeline project state
pub fn get_project_state_cmd() -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Create a new project optionally from a pre-configured template with custom dimensions and starter demo clips
pub fn create_project_cmd(
    name: String,
    template_id: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f32>,
    include_demo_clips: bool,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let mut svc = service.lock().map_err(|e| e.to_string())?;

    *svc = EditorService::create_project_with_options(
        template_id.as_deref(),
        &name,
        width,
        height,
        fps,
        include_demo_clips,
    )
    .map_err(|e| e.to_string())?;

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

/// Update project resolution and frame rate on an active timeline
pub fn update_project_settings_cmd(
    width: u32,
    height: u32,
    frame_rate: FrameRate,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.update_project_settings(width, height, frame_rate)
        .map_err(|e| e.to_string())
}

/// Import a local media file into the project's media pool and optionally append it onto the timeline
pub fn import_media_file_cmd(
    file_path: String,
    auto_add_to_timeline: bool,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.import_media_file(&file_path, auto_add_to_timeline)
        .map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Add a media source from `media_pool` onto a timeline track
pub fn add_media_to_timeline_cmd(
    source_id: String,
    track_id: Option<String>,
    timeline_in_ms: Option<u64>,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.add_media_to_timeline(&source_id, track_id.as_deref(), timeline_in_ms)
        .map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Add a new track to the timeline
pub fn add_track_cmd(name: String, track_type: TrackType) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.add_track(&name, track_type)
        .map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Move a clip to a new timeline position and optionally a different track
pub fn move_clip_cmd(
    clip_id: String,
    track_id: Option<String>,
    timeline_in_ms: u64,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.move_clip(&clip_id, track_id.as_deref(), timeline_in_ms)
        .map_err(|e| e.to_string())?;
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

/// Update clip color grading parameters
pub fn update_clip_color_grading_cmd(
    clip_id: Option<String>,
    grading: ColorGradingSettings,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.update_clip_color_grading(clip_id.as_deref(), grading)
        .map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Update clip green/blue screen chroma key settings
pub fn update_clip_chroma_key_cmd(
    clip_id: Option<String>,
    chroma_key: ChromaKeySettings,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.update_clip_chroma_key(clip_id.as_deref(), chroma_key)
        .map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Update track mute/solo/lock or mixer volume/pan
pub fn update_track_state_cmd(
    track_id: String,
    muted: Option<bool>,
    solo: Option<bool>,
    locked: Option<bool>,
    volume: Option<f32>,
    pan: Option<f32>,
) -> Result<TimelineProject, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.update_track_state(&track_id, muted, solo, locked, volume, pan)
        .map_err(|e| e.to_string())?;
    svc.get_project().map_err(|e| e.to_string())
}

/// Update 8-band master EQ and return updated EBU R128 loudness readings
pub fn update_master_eq_cmd(gains_db: Vec<f32>) -> Result<LoudnessMeterData, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.update_master_eq(&gains_db).map_err(|e| e.to_string())
}

/// AI Silence Ripple-Cutter returning removed (start_ms, end_ms) intervals
pub fn ai_ripple_cut_silence_cmd(
    threshold_db: f32,
    min_duration_ms: u64,
    pad_ms: u64,
) -> Result<Vec<(u64, u64)>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let config = crate::ai_editor::SilenceCutConfig {
        threshold_dbfs: threshold_db,
        min_silence_duration_ms: min_duration_ms,
        pre_pad_ms: pad_ms,
        post_pad_ms: pad_ms,
    };
    let intervals = svc
        .auto_cut_silence_intervals(&config)
        .map_err(|e| e.to_string())?;
    Ok(intervals
        .into_iter()
        .map(|iv| (iv.start_ms, iv.end_ms))
        .collect())
}

/// AI Beat Detector returning detected beat timestamps in ms
pub fn ai_detect_beats_cmd(threshold: f32) -> Result<Vec<u64>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.detect_beats_list(threshold).map_err(|e| e.to_string())
}

/// AI Scene Cut Detector returning detected scene boundary timestamps in ms
pub fn ai_detect_scene_cuts_cmd(threshold: f32) -> Result<Vec<u64>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.detect_scene_cuts(threshold).map_err(|e| e.to_string())
}

/// AI Auto-Captions Generator returning (start_ms, end_ms, text) cues
pub fn ai_generate_auto_captions_cmd() -> Result<Vec<(u64, u64, String)>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.generate_auto_captions().map_err(|e| e.to_string())
}

/// Render broadcast audio master WAV file and return summary string
pub fn render_audio_master_cmd(output_path: Option<String>) -> Result<String, String> {
    let path = match output_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = std::path::PathBuf::from(&p);
            if pb.is_relative() {
                if let Ok(app_paths) = platform_common::AppPaths::from_app_name("pro-av-editor") {
                    app_paths.data_dir.join(pb)
                } else {
                    std::env::temp_dir().join(pb)
                }
            } else {
                pb
            }
        }
        _ => {
            if let Ok(app_paths) = platform_common::AppPaths::from_app_name("pro-av-editor") {
                app_paths
                    .data_dir
                    .join("exports")
                    .join("master_mixdown.wav")
            } else {
                std::env::temp_dir().join("master_mixdown.wav")
            }
        }
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path_str = path.to_string_lossy().to_string();
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let bytes = svc
        .export_audio_master(&path_str)
        .map_err(|e| e.to_string())?;
    Ok(format!("{} ({} bytes PCM WAV)", path_str, bytes))
}

/// Render broadcast video master and export standard container with audio mixdown
pub fn render_video_master_cmd(
    preset_id: &str,
    output_path: Option<String>,
) -> Result<MasterExportSummary, String> {
    let preset = ExportPreset::all_standard_presets()
        .into_iter()
        .find(|p| p.id == preset_id)
        .unwrap_or_else(|| ExportPreset::all_standard_presets().remove(0));

    let path = match output_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = std::path::PathBuf::from(&p);
            if pb.is_relative() {
                if let Ok(app_paths) = platform_common::AppPaths::from_app_name("pro-av-editor") {
                    app_paths.data_dir.join(pb)
                } else {
                    std::env::temp_dir().join(pb)
                }
            } else {
                pb
            }
        }
        _ => {
            let ext = &preset.container;
            let filename = format!(
                "master_export_{}_{}.{}",
                (preset.name.replace(' ', "_")),
                date_tag(),
                ext
            );
            if let Ok(app_paths) = platform_common::AppPaths::from_app_name("pro-av-editor") {
                app_paths.data_dir.join("exports").join(filename)
            } else {
                std::env::temp_dir().join(filename)
            }
        }
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path_str = path.to_string_lossy().to_string();
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let summary = svc
        .export_video_master(preset_id, &path_str)
        .map_err(|e| e.to_string())?;
    Ok(summary)
}

/// Synchronize target clip to reference clip using waveform cross-correlation
pub fn sync_audio_tracks_cmd(
    reference_clip_id: String,
    target_clip_id: String,
) -> Result<AudioSyncResult, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.sync_clips_by_audio(&reference_clip_id, &target_clip_id)
        .map_err(|e| e.to_string())
}

/// Generate a low-resolution proxy raster cache for high-speed timeline playback
pub fn generate_proxy_cmd(
    source_id: String,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<ProxyGenerationResult, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.generate_media_proxy(&source_id, width.unwrap_or(640), height.unwrap_or(360))
        .map_err(|e| e.to_string())
}

fn date_tag() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Dispatch desktop IPC commands from the Svelte 5 studio frontend
pub fn dispatch_ipc_command(cmd: &str, args: Value) -> Result<Value, String> {
    match cmd {
        "get_project_state_cmd" => {
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "create_project_cmd" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Studio Project")
                .to_string();
            let template_id = args
                .get("templateId")
                .or_else(|| args.get("template_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let width = args.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
            let height = args
                .get("height")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let fps = args.get("fps").and_then(|v| v.as_f64()).map(|v| v as f32);
            let include_demo_clips = args
                .get("includeDemoClips")
                .or_else(|| args.get("include_demo_clips"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let res =
                create_project_cmd(name, template_id, width, height, fps, include_demo_clips)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "save_project_cmd" => {
            let default_path = platform_common::AppPaths::from_app_name("pro-av-editor")
                .map(|p| p.data_dir.join("studio_project.avedproj"))
                .unwrap_or_else(|_| std::env::temp_dir().join("studio_project.avedproj"));
            if let Some(parent) = default_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| default_path.to_string_lossy().to_string());
            save_project_cmd(path.clone())?;
            Ok(json!(path))
        }
        "load_project_cmd" | "open_project_cmd" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing path".to_string())?
                .to_string();
            let res = load_project_cmd(path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_project_settings_cmd" => {
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;
            let fps_val = args
                .get("frameRate")
                .or_else(|| args.get("frame_rate"))
                .and_then(|v| v.as_str())
                .unwrap_or("Fps60");
            let frame_rate = match fps_val {
                "Fps23_976" => FrameRate::Fps23_976,
                "Fps24" => FrameRate::Fps24,
                "Fps25" => FrameRate::Fps25,
                "Fps29_97" => FrameRate::Fps29_97,
                "Fps30" => FrameRate::Fps30,
                "Fps50" => FrameRate::Fps50,
                "Fps59_94" => FrameRate::Fps59_94,
                "Fps120" => FrameRate::Fps120,
                _ => FrameRate::Fps60,
            };
            let res = update_project_settings_cmd(width, height, frame_rate)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "import_media_file_cmd" => {
            let file_path = args
                .get("filePath")
                .or_else(|| args.get("file_path"))
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing filePath".to_string())?
                .to_string();
            let auto_add = args
                .get("autoAddToTimeline")
                .or_else(|| args.get("auto_add_to_timeline"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let res = import_media_file_cmd(file_path, auto_add)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "add_media_to_timeline_cmd" => {
            let source_id = args
                .get("sourceId")
                .or_else(|| args.get("source_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing sourceId".to_string())?
                .to_string();
            let track_id = args
                .get("trackId")
                .or_else(|| args.get("track_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let timeline_in_ms = args
                .get("timelineInMs")
                .or_else(|| args.get("timeline_in_ms"))
                .and_then(|v| v.as_u64());
            let res = add_media_to_timeline_cmd(source_id, track_id, timeline_in_ms)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "add_track_cmd" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let track_type_str = args
                .get("trackType")
                .or_else(|| args.get("track_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("Video");
            let track_type = match track_type_str.to_ascii_lowercase().as_str() {
                "audio" => TrackType::Audio,
                "subtitle" => TrackType::Subtitle,
                "adjustment" => TrackType::Adjustment,
                _ => TrackType::Video,
            };
            let res = add_track_cmd(name, track_type)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "move_clip_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing clipId".to_string())?
                .to_string();
            let track_id = args
                .get("trackId")
                .or_else(|| args.get("track_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let timeline_in_ms = args
                .get("timelineInMs")
                .or_else(|| args.get("timeline_in_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let res = move_clip_cmd(clip_id, track_id, timeline_in_ms)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_clip_properties_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing clipId".to_string())?
                .to_string();
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let timeline_in_ms = args
                .get("timelineInMs")
                .or_else(|| args.get("timeline_in_ms"))
                .and_then(|v| v.as_u64());
            let timeline_out_ms = args
                .get("timelineOutMs")
                .or_else(|| args.get("timeline_out_ms"))
                .and_then(|v| v.as_u64());
            let speed_multiplier = args
                .get("speedMultiplier")
                .or_else(|| args.get("speed_multiplier"))
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            let opacity = args
                .get("opacity")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            let volume = args
                .get("volume")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            let pan = args.get("pan").and_then(|v| v.as_f64()).map(|f| f as f32);
            let blend_mode: Option<BlendMode> = args
                .get("blendMode")
                .or_else(|| args.get("blend_mode"))
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok());
            let transform: Option<Transform2D> = args
                .get("transform")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok());

            let service = get_service();
            let svc = service.lock().map_err(|e| e.to_string())?;
            svc.update_clip_properties(
                &clip_id,
                name,
                timeline_in_ms,
                timeline_out_ms,
                speed_multiplier,
                opacity,
                volume,
                pan,
                blend_mode,
                transform,
            )
            .map_err(|e| e.to_string())?;
            let res = svc.get_project().map_err(|e| e.to_string())?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_media_duration_cmd" => {
            let source_id = args
                .get("sourceId")
                .or_else(|| args.get("source_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing sourceId".to_string())?;
            let duration_ms = args
                .get("durationMs")
                .or_else(|| args.get("duration_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let width = args.get("width").and_then(|v| v.as_u64()).map(|w| w as u32);
            let height = args
                .get("height")
                .and_then(|v| v.as_u64())
                .map(|h| h as u32);
            let service = get_service();
            let svc = service.lock().map_err(|e| e.to_string())?;
            svc.update_media_duration(source_id, duration_ms, width, height)
                .map_err(|e| e.to_string())?;
            let res = svc.get_project().map_err(|e| e.to_string())?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "add_clip_cmd" => {
            let clip_val = args.get("clip").cloned().unwrap_or(args);
            let clip: Clip = serde_json::from_value(clip_val).map_err(|e| e.to_string())?;
            add_clip_cmd(clip)?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "remove_clip_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing clipId".to_string())?
                .to_string();
            remove_clip_cmd(clip_id)?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "split_clip_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing clipId".to_string())?
                .to_string();
            let timestamp_ms = args
                .get("timestampMs")
                .or_else(|| args.get("timestamp_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1000);
            split_clip_cmd(clip_id, timestamp_ms)?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "trim_clip_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing clipId".to_string())?
                .to_string();
            let new_in_ms = args
                .get("newInMs")
                .or_else(|| args.get("new_in_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let new_out_ms = args
                .get("newOutMs")
                .or_else(|| args.get("new_out_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1000);
            trim_clip_cmd(clip_id, new_in_ms, new_out_ms)?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "ripple_delete_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing clipId".to_string())?
                .to_string();
            ripple_delete_cmd(clip_id)?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "undo_cmd" => {
            let _ = undo_cmd()?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "redo_cmd" => {
            let _ = redo_cmd()?;
            let res = get_project_state_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "render_preview_frame_cmd" => {
            let timestamp_ms = args
                .get("timestampMs")
                .or_else(|| args.get("timestamp_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(640) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(360) as u32;
            let b64 = render_preview_frame_cmd(timestamp_ms, width, height)?;
            Ok(json!(b64))
        }
        "get_video_scopes_cmd" => {
            let timestamp_ms = args
                .get("timestampMs")
                .or_else(|| args.get("timestamp_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(160) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(90) as u32;
            let res = get_video_scopes_cmd(timestamp_ms, width, height)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "get_audio_loudness_cmd" => {
            let start_ms = args
                .get("startMs")
                .or_else(|| args.get("start_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let end_ms = args
                .get("endMs")
                .or_else(|| args.get("end_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(2000);
            let res = get_audio_loudness_cmd(start_ms, end_ms)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_clip_color_grading_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let grading_val = args
                .get("grading")
                .cloned()
                .ok_or_else(|| "Missing grading payload".to_string())?;
            let grading: ColorGradingSettings =
                serde_json::from_value(grading_val).map_err(|e| e.to_string())?;
            let res = update_clip_color_grading_cmd(clip_id, grading)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_clip_chroma_key_cmd" => {
            let clip_id = args
                .get("clipId")
                .or_else(|| args.get("clip_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let chroma_val = args
                .get("chromaKey")
                .or_else(|| args.get("chroma_key"))
                .cloned()
                .ok_or_else(|| "Missing chromaKey payload".to_string())?;
            let chroma_key: ChromaKeySettings =
                serde_json::from_value(chroma_val).map_err(|e| e.to_string())?;
            let res = update_clip_chroma_key_cmd(clip_id, chroma_key)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_track_state_cmd" => {
            let track_id = args
                .get("trackId")
                .or_else(|| args.get("track_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing trackId".to_string())?
                .to_string();
            let muted = args.get("muted").and_then(|v| v.as_bool());
            let solo = args.get("solo").and_then(|v| v.as_bool());
            let locked = args.get("locked").and_then(|v| v.as_bool());
            let volume = args
                .get("volume")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            let pan = args.get("pan").and_then(|v| v.as_f64()).map(|f| f as f32);
            let res = update_track_state_cmd(track_id, muted, solo, locked, volume, pan)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "update_master_eq_cmd" => {
            let gains: Vec<f32> = args
                .get("gainsDb")
                .or_else(|| args.get("gains_db"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|x| x.as_f64().unwrap_or(0.0) as f32)
                        .collect()
                })
                .unwrap_or_else(|| vec![0.0; 8]);
            let res = update_master_eq_cmd(gains)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "auto_cut_silence_cmd" | "ai_ripple_cut_silence_cmd" => {
            let threshold_db = args
                .get("thresholdDb")
                .or_else(|| args.get("threshold_dbfs"))
                .and_then(|v| v.as_f64())
                .unwrap_or(-38.0) as f32;
            let min_duration_ms = args
                .get("minDurationMs")
                .or_else(|| args.get("min_silence_duration_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(400);
            let pad_ms = args
                .get("padMs")
                .or_else(|| args.get("pre_pad_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(60);
            let res = ai_ripple_cut_silence_cmd(threshold_db, min_duration_ms, pad_ms)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "detect_beats_cmd" | "ai_detect_beats_cmd" => {
            let threshold = args
                .get("threshold")
                .or_else(|| args.get("sensitivity"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.25) as f32;
            let res = ai_detect_beats_cmd(threshold)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "ai_detect_scene_cuts_cmd" => {
            let threshold = args
                .get("threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(25.0) as f32;
            let res = ai_detect_scene_cuts_cmd(threshold)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "ai_generate_auto_captions_cmd" => {
            let res = ai_generate_auto_captions_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "render_audio_master_cmd" => {
            let output_path = args
                .get("outputPath")
                .or_else(|| args.get("output_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let res = render_audio_master_cmd(output_path)?;
            Ok(json!(res))
        }
        "render_video_master_cmd" => {
            let preset_id = args
                .get("presetId")
                .or_else(|| args.get("preset_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("preset_1080p60_web");
            let output_path = args
                .get("outputPath")
                .or_else(|| args.get("output_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let res = render_video_master_cmd(preset_id, output_path)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "sync_audio_tracks_cmd" => {
            let ref_id = args
                .get("referenceClipId")
                .or_else(|| args.get("reference_clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing referenceClipId".to_string())?
                .to_string();
            let tgt_id = args
                .get("targetClipId")
                .or_else(|| args.get("target_clip_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing targetClipId".to_string())?
                .to_string();
            let res = sync_audio_tracks_cmd(ref_id, tgt_id)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "generate_proxy_cmd" => {
            let source_id = args
                .get("sourceId")
                .or_else(|| args.get("source_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing sourceId".to_string())?
                .to_string();
            let width = args.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
            let height = args
                .get("height")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let res = generate_proxy_cmd(source_id, width, height)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "export_edl_cmd" => {
            let res = export_edl_cmd()?;
            Ok(json!(res))
        }
        "export_fcpxml_cmd" => {
            let res = export_fcpxml_cmd()?;
            Ok(json!(res))
        }
        "get_templates_cmd" => {
            let res = get_templates_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "get_export_presets_cmd" => {
            let res = get_export_presets_cmd()?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        _ => Err(format!("Unknown Pro AV Editor IPC command: {}", cmd)),
    }
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
        assert_eq!(presets.len(), 6);

        let preview_b64 = render_preview_frame_cmd(0, 64, 36).unwrap();
        assert!(!preview_b64.is_empty());

        let scopes = get_video_scopes_cmd(0, 64, 36).unwrap();
        assert_eq!(scopes.histogram_r.len(), 256);

        // Verify real BMP image import, media pool registration, track addition, and clip movement
        let tmp_bmp = std::env::temp_dir().join("pave_test_import_image.bmp");
        let frame = crate::compositor::VideoFrame::new_solid(4, 4, 255, 64, 32, 255, 0);
        let bmp_bytes = frame.encode_to_bmp();
        std::fs::write(&tmp_bmp, &bmp_bytes).unwrap();

        let imported_proj =
            import_media_file_cmd(tmp_bmp.to_string_lossy().to_string(), true).unwrap();
        assert!(!imported_proj.media_pool.is_empty());

        let track_proj = add_track_cmd("Overlay Video 2".to_string(), TrackType::Video).unwrap();
        assert!(track_proj.tracks.len() >= 4);

        let first_clip_id = track_proj.tracks[0].clips[0].id.clone();
        let moved_proj = move_clip_cmd(first_clip_id, None, 250).unwrap();
        assert_eq!(moved_proj.tracks[0].clips[0].timeline_in_ms, 250);

        let _ = std::fs::remove_file(&tmp_bmp);

        // Test video master export command
        let out_avi = std::env::temp_dir().join("test_master_export.avi");
        let export_res = render_video_master_cmd(
            "preset_uncompressed_avi",
            Some(out_avi.to_string_lossy().to_string()),
        )
        .unwrap();
        assert_eq!(export_res.container, "avi");
        assert!(export_res.bytes_written > 1000);
        assert!(out_avi.exists());
        let _ = std::fs::remove_file(&out_avi);

        // Test create project with custom options
        let custom_proj = create_project_cmd(
            "Custom 4K Commercial".to_string(),
            Some("tpl_cinematic_239".to_string()),
            Some(3840),
            Some(1608),
            Some(24.0),
            true,
        )
        .unwrap();
        assert_eq!(custom_proj.name, "Custom 4K Commercial");
        assert_eq!(custom_proj.settings.width, 3840);
        assert_eq!(custom_proj.settings.height, 1608);
        assert!(!custom_proj.tracks.is_empty());

        // Test update project settings command
        let updated_proj = update_project_settings_cmd(1080, 1920, FrameRate::Fps60).unwrap();
        assert_eq!(updated_proj.settings.width, 1080);
        assert_eq!(updated_proj.settings.height, 1920);
        assert_eq!(updated_proj.settings.frame_rate, FrameRate::Fps60);

        // Test proxy generation and IPC dispatch
        // Add dummy source if media_pool is empty
        if updated_proj.media_pool.is_empty() {
            let tmp_bmp2 = std::env::temp_dir().join("pave_test_proxy_src.bmp");
            let f = crate::compositor::VideoFrame::new_solid(4, 4, 10, 20, 30, 255, 0);
            std::fs::write(&tmp_bmp2, f.encode_to_bmp()).unwrap();
            let _ = import_media_file_cmd(tmp_bmp2.to_string_lossy().to_string(), false);
            let _ = std::fs::remove_file(&tmp_bmp2);
        }
        let current_proj = get_project_state_cmd().unwrap();
        let target_source_id = current_proj.media_pool[0].id.clone();
        let proxy_res = generate_proxy_cmd(target_source_id.clone(), Some(320), Some(180)).unwrap();
        assert_eq!(proxy_res.width, 320);
        assert_eq!(proxy_res.height, 180);
        assert!(std::path::Path::new(&proxy_res.proxy_path).exists());
        let _ = std::fs::remove_file(&proxy_res.proxy_path);

        // Test audio sync command across two clips
        let track_a = &current_proj.tracks[0];
        if track_a.clips.len() >= 2 {
            let ref_id = track_a.clips[0].id.clone();
            let tgt_id = track_a.clips[1].id.clone();
            let sync_res = sync_audio_tracks_cmd(ref_id, tgt_id).unwrap();
            assert!(sync_res.correlation_score >= 0.0);
        }

        // Test IPC dispatch for proxy and audio sync
        let proxy_ipc_val = dispatch_ipc_command(
            "generate_proxy_cmd",
            json!({
                "sourceId": target_source_id,
                "width": 160,
                "height": 90
            }),
        )
        .unwrap();
        assert!(proxy_ipc_val.get("proxy_path").is_some());
    }
}
