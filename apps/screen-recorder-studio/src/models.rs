#![forbid(unsafe_code)]
//! Core Domain Models for Offline AI Screen Recorder & Interactive Tutorial Builder

use serde::{Deserialize, Serialize};

/// Type of mouse click action registered during recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ClickType {
    #[default]
    None,
    Left,
    Right,
    Double,
}

/// Telemetry event capturing mouse cursor position and clicks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorEvent {
    pub timestamp_ms: u64,
    pub x: f32,
    pub y: f32,
    pub click: ClickType,
}

/// Telemetry event capturing keyboard interaction and shortcut triggers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeystrokeEvent {
    pub timestamp_ms: u64,
    pub key_combination: String,
    pub duration_ms: u64,
}

/// Bounding region identified for privacy masking / PII redaction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactionRegion {
    pub id: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub pattern_type: String, // "api_key", "email", "credit_card", "jwt", "custom"
}

/// Text block with coordinate bounding box for OCR / PII detection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRegion {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Dynamic camera auto-zoom keyframe centered on user focus areas
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoomKeyframe {
    pub start_ms: u64,
    pub end_ms: u64,
    pub focus_x: f32,
    pub focus_y: f32,
    pub scale: f32,
    pub easing: String,
    pub label: String,
}

/// Detected audio silence / dead-air interval eligible for trimming
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSilenceInterval {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub rms_db: f32,
}

/// Timed subtitle caption cue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub speaker: Option<String>,
}

/// Interactive step-by-step tutorial milestone
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TutorialStep {
    pub step_number: u32,
    pub timestamp_ms: u64,
    pub title: String,
    pub instruction: String,
    pub action_type: String,
    pub target_x: f32,
    pub target_y: f32,
}

/// Complete recording session project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingSession {
    pub session_id: String,
    pub title: String,
    pub created_at: i64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_clicks: usize,
    pub auto_zooms_count: usize,
    pub silence_trimmed_ms: u64,
    pub cursor_events: Vec<CursorEvent>,
    #[serde(default)]
    pub keystroke_events: Vec<KeystrokeEvent>,
    pub zoom_keyframes: Vec<ZoomKeyframe>,
    #[serde(default)]
    pub redaction_regions: Vec<RedactionRegion>,
    pub subtitles: Vec<SubtitleCue>,
    pub tutorial_steps: Vec<TutorialStep>,
}

/// Supported export format targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ExportFormat {
    #[default]
    Mp4,
    WebM,
    Gif,
    InteractiveHtml,
}

/// Configuration parameters for tutorial video rendering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_auto_zoom: bool,
    pub trim_dead_air: bool,
    pub burn_subtitles: bool,
    pub target_width: u32,
    pub target_height: u32,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Mp4,
            include_auto_zoom: true,
            trim_dead_air: true,
            burn_subtitles: true,
            target_width: 1920,
            target_height: 1080,
        }
    }
}

/// Global studio engine metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StudioStats {
    pub total_recordings: usize,
    pub total_duration_secs: f64,
    pub total_zooms_generated: usize,
    pub total_silence_saved_secs: f64,
    pub total_tutorial_steps: usize,
}
