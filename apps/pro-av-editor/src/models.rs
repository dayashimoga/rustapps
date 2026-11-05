#![deny(unsafe_code)]
//! Core Domain Models for Professional Audio & Video Editor (PAVE)
//!
//! Conforms to non-destructive NLE editing paradigms:
//! - Multi-track timeline DAG (Video, Audio, Subtitle, Adjustment)
//! - Frame-accurate Timecode with SMPTE drop-frame / non-drop-frame formatting
//! - Keyframe automation (Linear, Bezier, EaseIn, EaseOut, Hold)
//! - Transitions (CrossDissolve, DipToBlack, Wipe, Slide, Zoom, LumaFade)
//! - Blend modes, Spatial 2D Transforms, Color Grading Parameters, 8-Band Audio EQ

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Timecode & Frame Rates
// ---------------------------------------------------------------------------

/// Standard SMPTE frame rates supported by the NLE engine
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FrameRate {
    Fps23_976,
    Fps24,
    Fps25,
    Fps29_97,
    Fps30,
    Fps50,
    Fps59_94,
    Fps60,
    Fps120,
}

impl FrameRate {
    /// Nominal or precise floating point frames per second
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Fps23_976 => 24000.0 / 1001.0,
            Self::Fps24 => 24.0,
            Self::Fps25 => 25.0,
            Self::Fps29_97 => 30000.0 / 1001.0,
            Self::Fps30 => 30.0,
            Self::Fps50 => 50.0,
            Self::Fps59_94 => 60000.0 / 1001.0,
            Self::Fps60 => 60.0,
            Self::Fps120 => 120.0,
        }
    }

    /// Whether this frame rate uses SMPTE drop-frame notation
    pub fn is_drop_frame(&self) -> bool {
        matches!(self, Self::Fps29_97 | Self::Fps59_94)
    }
}

/// High-precision SMPTE Timecode representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timecode {
    pub frame: u64,
    pub fps_nominal: u32,
    pub is_drop_frame: bool,
}

impl Timecode {
    /// Create timecode from frame count and frame rate
    pub fn from_frames(frame: u64, rate: FrameRate) -> Self {
        Self {
            frame,
            fps_nominal: rate.as_f64().round() as u32,
            is_drop_frame: rate.is_drop_frame(),
        }
    }

    /// Create timecode from timestamp in milliseconds
    pub fn from_millis(ms: u64, rate: FrameRate) -> Self {
        let fps = rate.as_f64();
        let frame = ((ms as f64 * fps) / 1000.0).round() as u64;
        Self::from_frames(frame, rate)
    }

    /// Convert timecode back to milliseconds
    pub fn to_millis(&self) -> u64 {
        let fps = self.fps_nominal as f64;
        if fps <= 0.0 {
            return 0;
        }
        ((self.frame as f64 * 1000.0) / fps).round() as u64
    }

    /// Format as SMPTE standard `HH:MM:SS:FF` or `HH:MM:SS;FF` (drop-frame)
    pub fn to_smpte(&self) -> String {
        let fps = self.fps_nominal.max(1) as u64;
        let ff = self.frame % fps;
        let total_secs = self.frame / fps;
        let ss = total_secs % 60;
        let total_mins = total_secs / 60;
        let mm = total_mins % 60;
        let hh = total_mins / 60;

        let sep = if self.is_drop_frame { ';' } else { ':' };
        format!("{:02}:{:02}:{:02}{}{:02}", hh, mm, ss, sep, ff)
    }

    /// Parse SMPTE string `HH:MM:SS:FF` into Timecode
    pub fn parse_smpte(smpte: &str, rate: FrameRate) -> Option<Self> {
        let s = smpte.trim();
        let parts: Vec<&str> = s.split([':', ';']).collect();
        if parts.len() != 4 {
            return None;
        }
        let hh: u64 = parts[0].parse().ok()?;
        let mm: u64 = parts[1].parse().ok()?;
        let ss: u64 = parts[2].parse().ok()?;
        let ff: u64 = parts[3].parse().ok()?;

        let fps = rate.as_f64().round() as u64;
        let frame = (hh * 3600 + mm * 60 + ss) * fps + ff;
        Some(Self::from_frames(frame, rate))
    }
}

// ---------------------------------------------------------------------------
// Keyframe Animation Curves
// ---------------------------------------------------------------------------

/// Interpolation curve applied between consecutive keyframes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum KeyframeCurve {
    #[default]
    Linear,
    Bezier {
        p1_x: f32,
        p1_y: f32,
        p2_x: f32,
        p2_y: f32,
    },
    EaseIn,
    EaseOut,
    EaseInOut,
    Hold,
}

/// An individual animated keyframe parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub timestamp_ms: u64,
    pub value: f32,
    pub curve: KeyframeCurve,
}

impl Keyframe {
    pub fn new(timestamp_ms: u64, value: f32, curve: KeyframeCurve) -> Self {
        Self {
            timestamp_ms,
            value,
            curve,
        }
    }
}

/// Property animated across time with keyframes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimatedProperty {
    pub default_value: f32,
    pub keyframes: Vec<Keyframe>,
}

impl Default for AnimatedProperty {
    fn default() -> Self {
        Self::constant(1.0)
    }
}

impl AnimatedProperty {
    pub fn constant(val: f32) -> Self {
        Self {
            default_value: val,
            keyframes: Vec::new(),
        }
    }

    /// Add a keyframe, maintaining timestamp ordering
    pub fn add_keyframe(&mut self, timestamp_ms: u64, value: f32, curve: KeyframeCurve) {
        self.keyframes
            .push(Keyframe::new(timestamp_ms, value, curve));
        self.keyframes.sort_by_key(|k| k.timestamp_ms);
    }

    /// Evaluate property value at specific timestamp with interpolation
    pub fn evaluate_at(&self, timestamp_ms: u64) -> f32 {
        if self.keyframes.is_empty() {
            return self.default_value;
        }

        // Before first keyframe
        if timestamp_ms <= self.keyframes[0].timestamp_ms {
            return self.keyframes[0].value;
        }

        // After last keyframe
        let last = &self.keyframes[self.keyframes.len() - 1];
        if timestamp_ms >= last.timestamp_ms {
            return last.value;
        }

        // Find surrounding keyframes
        for i in 0..self.keyframes.len() - 1 {
            let k1 = &self.keyframes[i];
            let k2 = &self.keyframes[i + 1];

            if timestamp_ms >= k1.timestamp_ms && timestamp_ms <= k2.timestamp_ms {
                let range = (k2.timestamp_ms - k1.timestamp_ms) as f32;
                if range <= 0.0 {
                    return k1.value;
                }
                let t = ((timestamp_ms - k1.timestamp_ms) as f32) / range;

                return match k1.curve {
                    KeyframeCurve::Hold => k1.value,
                    KeyframeCurve::Linear => k1.value + t * (k2.value - k1.value),
                    KeyframeCurve::EaseIn => {
                        let factor = t * t;
                        k1.value + factor * (k2.value - k1.value)
                    }
                    KeyframeCurve::EaseOut => {
                        let factor = t * (2.0 - t);
                        k1.value + factor * (k2.value - k1.value)
                    }
                    KeyframeCurve::EaseInOut => {
                        let factor = if t < 0.5 {
                            2.0 * t * t
                        } else {
                            -1.0 + (4.0 - 2.0 * t) * t
                        };
                        k1.value + factor * (k2.value - k1.value)
                    }
                    KeyframeCurve::Bezier { p1_y, p2_y, .. } => {
                        // Approximate cubic bezier easing
                        let inv_t = 1.0 - t;
                        let factor =
                            3.0 * inv_t * inv_t * t * p1_y + 3.0 * inv_t * t * t * p2_y + t * t * t;
                        k1.value + factor * (k2.value - k1.value)
                    }
                };
            }
        }

        self.default_value
    }
}

// ---------------------------------------------------------------------------
// Spatial 2D Transforms & Blend Modes
// ---------------------------------------------------------------------------

/// Video layer compositing blend modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
    Darken,
    Lighten,
    Difference,
}

/// 2D affine transformation on video clip frame
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform2D {
    pub scale_x: f32,
    pub scale_y: f32,
    pub translate_x: f32,
    pub translate_y: f32,
    pub rotation_degrees: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            rotation_degrees: 0.0,
            anchor_x: 0.5,
            anchor_y: 0.5,
        }
    }
}

/// Keyframe-animated 2D spatial transformations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct KeyframedTransform {
    pub scale_x: Option<AnimatedProperty>,
    pub scale_y: Option<AnimatedProperty>,
    pub translate_x: Option<AnimatedProperty>,
    pub translate_y: Option<AnimatedProperty>,
    pub rotation_degrees: Option<AnimatedProperty>,
}

impl KeyframedTransform {
    /// Evaluate all animated transform channels at a given timestamp against base transform
    pub fn evaluate_at(&self, timestamp_ms: u64, base: &Transform2D) -> Transform2D {
        Transform2D {
            scale_x: self
                .scale_x
                .as_ref()
                .map(|p| p.evaluate_at(timestamp_ms))
                .unwrap_or(base.scale_x),
            scale_y: self
                .scale_y
                .as_ref()
                .map(|p| p.evaluate_at(timestamp_ms))
                .unwrap_or(base.scale_y),
            translate_x: self
                .translate_x
                .as_ref()
                .map(|p| p.evaluate_at(timestamp_ms))
                .unwrap_or(base.translate_x),
            translate_y: self
                .translate_y
                .as_ref()
                .map(|p| p.evaluate_at(timestamp_ms))
                .unwrap_or(base.translate_y),
            rotation_degrees: self
                .rotation_degrees
                .as_ref()
                .map(|p| p.evaluate_at(timestamp_ms))
                .unwrap_or(base.rotation_degrees),
            anchor_x: base.anchor_x,
            anchor_y: base.anchor_y,
        }
    }
}

// ---------------------------------------------------------------------------
// 3-Way Color Grading & Scopes
// ---------------------------------------------------------------------------

/// RGB color wheel triple (e.g. Lift, Gamma, Gain)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorWheel {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub luma: f32,
}

impl Default for ColorWheel {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            luma: 0.0,
        }
    }
}

impl ColorWheel {
    pub fn is_zero(&self) -> bool {
        self.r.abs() < 1e-4 && self.g.abs() < 1e-4 && self.b.abs() < 1e-4 && self.luma.abs() < 1e-4
    }
}

/// Primary color grading node parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorGradingSettings {
    pub lift: ColorWheel,  // Shadows
    pub gamma: ColorWheel, // Midtones
    pub gain: ColorWheel,  // Highlights
    pub exposure: f32,     // EV stops (-5.0 to +5.0)
    pub contrast: f32,     // 0.0 to 2.0 (1.0 default)
    pub saturation: f32,   // 0.0 to 2.0 (1.0 default)
    pub temperature: f32,  // -100.0 (cool) to +100.0 (warm)
    pub tint: f32,         // -100.0 (green) to +100.0 (magenta)
    pub lut_file_path: Option<String>,
}

impl Default for ColorGradingSettings {
    fn default() -> Self {
        Self {
            lift: ColorWheel::default(),
            gamma: ColorWheel::default(),
            gain: ColorWheel::default(),
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            lut_file_path: None,
        }
    }
}

impl ColorGradingSettings {
    pub fn is_identity(&self) -> bool {
        self.exposure.abs() < 1e-4
            && (self.contrast - 1.0).abs() < 1e-4
            && (self.saturation - 1.0).abs() < 1e-4
            && self.temperature.abs() < 1e-4
            && self.tint.abs() < 1e-4
            && self.lift.is_zero()
            && self.gamma.is_zero()
            && self.gain.is_zero()
            && self.lut_file_path.is_none()
    }
}

/// Green/blue screen chroma key extraction configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChromaKeySettings {
    pub enabled: bool,
    pub key_color: (u8, u8, u8), // RGB, default (0, 255, 0)
    pub similarity: f32,         // 0.0 to 1.0 (threshold)
    pub smoothness: f32,         // 0.0 to 1.0 (feather edge)
    pub despill: f32,            // 0.0 to 1.0 (color neutralization)
}

impl Default for ChromaKeySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            key_color: (0, 255, 0),
            similarity: 0.4,
            smoothness: 0.1,
            despill: 0.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Audio DSP Models (8-Band Parametric EQ & Dynamics)
// ---------------------------------------------------------------------------

/// Audio filter biquad filter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Bell,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

/// Single band of parametric equalizer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q_factor: f32,
}

impl EqBand {
    pub fn new(filter_type: FilterType, frequency_hz: f32, gain_db: f32, q_factor: f32) -> Self {
        Self {
            enabled: true,
            filter_type,
            frequency_hz,
            gain_db,
            q_factor,
        }
    }
}

/// 8-Band parametric equalizer configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricEqSettings {
    pub enabled: bool,
    pub bands: Vec<EqBand>,
}

impl Default for ParametricEqSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            bands: vec![
                EqBand::new(FilterType::HighPass, 40.0, 0.0, 0.707),
                EqBand::new(FilterType::LowShelf, 100.0, 0.0, 0.707),
                EqBand::new(FilterType::Bell, 250.0, 0.0, 1.0),
                EqBand::new(FilterType::Bell, 800.0, 0.0, 1.0),
                EqBand::new(FilterType::Bell, 2000.0, 0.0, 1.0),
                EqBand::new(FilterType::Bell, 5000.0, 0.0, 1.0),
                EqBand::new(FilterType::HighShelf, 10000.0, 0.0, 0.707),
                EqBand::new(FilterType::LowPass, 18000.0, 0.0, 0.707),
            ],
        }
    }
}

/// Audio compressor and dynamics processor settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorSettings {
    pub enabled: bool,
    pub threshold_db: f32,   // -60.0 to 0.0 dB
    pub ratio: f32,          // 1.0 to 20.0:1
    pub attack_ms: f32,      // 0.1 to 100.0 ms
    pub release_ms: f32,     // 10.0 to 1000.0 ms
    pub makeup_gain_db: f32, // 0.0 to 24.0 dB
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Timeline DAG: Tracks, Clips, Transitions & Project
// ---------------------------------------------------------------------------

/// Track categories in the timeline DAG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackType {
    Video,
    Audio,
    Subtitle,
    Adjustment,
}

/// Media clip categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipType {
    Video,
    Audio,
    Image,
    Title,
    ColorSolid,
    Adjustment,
}

/// Media source asset metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaSource {
    pub id: String,
    pub file_path: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub sample_rate: u32,
    pub channels: u32,
    pub has_video: bool,
    pub has_audio: bool,
}

/// Timeline clip element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub name: String,
    pub clip_type: ClipType,
    pub track_id: String,
    pub source_id: Option<String>,
    pub source_in_ms: u64,
    pub source_out_ms: u64,
    pub timeline_in_ms: u64,
    pub timeline_out_ms: u64,
    pub speed_multiplier: f32, // 0.1x to 10.0x
    pub volume: AnimatedProperty,
    pub pan: f32, // -1.0 (Left) to +1.0 (Right)
    pub opacity: AnimatedProperty,
    pub blend_mode: BlendMode,
    pub transform: Transform2D,
    #[serde(default)]
    pub keyframed_transform: Option<KeyframedTransform>,
    #[serde(default)]
    pub chroma_key: ChromaKeySettings,
    pub color_grading: ColorGradingSettings,
    pub eq_settings: ParametricEqSettings,
    pub compressor: CompressorSettings,
}

impl Clip {
    /// Return the active 2D transform at a timestamp, evaluating keyframes if configured
    pub fn transform_at(&self, timestamp_ms: u64) -> Transform2D {
        if let Some(ref kt) = self.keyframed_transform {
            kt.evaluate_at(timestamp_ms, &self.transform)
        } else {
            self.transform.clone()
        }
    }

    /// Calculate current clip duration on the timeline in milliseconds
    pub fn duration_ms(&self) -> u64 {
        self.timeline_out_ms.saturating_sub(self.timeline_in_ms)
    }

    /// Check whether clip is active at specific timeline timestamp
    pub fn contains_timestamp(&self, timestamp_ms: u64) -> bool {
        timestamp_ms >= self.timeline_in_ms && timestamp_ms < self.timeline_out_ms
    }

    /// Map timeline timestamp to media source timestamp taking speed into account
    pub fn map_timeline_to_source_ms(&self, timestamp_ms: u64) -> u64 {
        if timestamp_ms < self.timeline_in_ms {
            return self.source_in_ms;
        }
        let offset = timestamp_ms - self.timeline_in_ms;
        let speed = if self.speed_multiplier > 0.0 {
            self.speed_multiplier
        } else {
            1.0
        };
        let src_offset = (offset as f32 * speed) as u64;
        (self.source_in_ms + src_offset).min(self.source_out_ms)
    }
}

/// Transition between two adjacent video or audio clips
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionType {
    CrossDissolve,
    DipToBlack,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
    ZoomIn,
    ZoomOut,
    LumaFade,
}

/// Transition specification connecting two clips
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub transition_type: TransitionType,
    pub duration_ms: u64,
    pub from_clip_id: String,
    pub to_clip_id: String,
    pub center_timestamp_ms: u64,
}

/// Timeline track grouping multiple clips
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub track_type: TrackType,
    pub z_index: i32,
    pub muted: bool,
    pub solo: bool,
    pub locked: bool,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        track_type: TrackType,
        z_index: i32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            track_type,
            z_index,
            muted: false,
            solo: false,
            locked: false,
            clips: Vec::new(),
        }
    }

    /// Find clip containing specified timestamp
    pub fn clip_at(&self, timestamp_ms: u64) -> Option<&Clip> {
        self.clips
            .iter()
            .find(|c| c.contains_timestamp(timestamp_ms))
    }
}

/// Timeline marker for beats, comments, or chapters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineMarker {
    pub id: String,
    pub timestamp_ms: u64,
    pub label: String,
    pub color_hex: String,
}

/// Project-wide master settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub sample_rate: u32,
    pub audio_channels: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            frame_rate: FrameRate::Fps60,
            sample_rate: 48000,
            audio_channels: 2,
        }
    }
}

/// Complete non-destructive project representation (`.avedproj`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineProject {
    pub schema_version: u32,
    pub project_id: String,
    pub name: String,
    pub settings: ProjectSettings,
    pub tracks: Vec<Track>,
    pub transitions: Vec<Transition>,
    pub markers: Vec<TimelineMarker>,
    pub media_pool: Vec<MediaSource>,
    pub created_at_secs: u64,
    pub updated_at_secs: u64,
}

impl TimelineProject {
    pub fn new(id: impl Into<String>, name: impl Into<String>, settings: ProjectSettings) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            schema_version: 1,
            project_id: id.into(),
            name: name.into(),
            settings,
            tracks: Vec::new(),
            transitions: Vec::new(),
            markers: Vec::new(),
            media_pool: Vec::new(),
            created_at_secs: now,
            updated_at_secs: now,
        }
    }

    /// Calculate total timeline duration in milliseconds across all tracks
    pub fn duration_ms(&self) -> u64 {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.timeline_out_ms)
            .max()
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Master Export Output Model
// ---------------------------------------------------------------------------

/// Result metadata produced when rendering an offline master export
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterExportSummary {
    pub path: String,
    pub bytes_written: usize,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub frame_count: usize,
    pub fps: f32,
    pub container: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smpte_timecode_conversions() {
        let tc = Timecode::from_frames(180, FrameRate::Fps60);
        assert_eq!(tc.to_smpte(), "00:00:03:00");
        assert_eq!(tc.to_millis(), 3000);

        let tc2 = Timecode::from_millis(5500, FrameRate::Fps30);
        assert_eq!(tc2.to_smpte(), "00:00:05:15");

        let parsed = Timecode::parse_smpte("01:02:03:12", FrameRate::Fps24).unwrap();
        assert_eq!(parsed.to_smpte(), "01:02:03:12");
    }

    #[test]
    fn test_animated_property_keyframes() {
        let mut prop = AnimatedProperty::constant(1.0);
        prop.keyframes
            .push(Keyframe::new(0, 0.0, KeyframeCurve::Linear));
        prop.keyframes
            .push(Keyframe::new(1000, 10.0, KeyframeCurve::Linear));
        prop.keyframes
            .push(Keyframe::new(2000, 10.0, KeyframeCurve::Hold));
        prop.keyframes
            .push(Keyframe::new(3000, 20.0, KeyframeCurve::Linear));

        assert_eq!(prop.evaluate_at(0), 0.0);
        assert_eq!(prop.evaluate_at(500), 5.0);
        assert_eq!(prop.evaluate_at(1000), 10.0);
        assert_eq!(prop.evaluate_at(1500), 10.0);
        assert_eq!(prop.evaluate_at(2500), 10.0); // Hold curve holds value at k1
        assert_eq!(prop.evaluate_at(3000), 20.0);
    }

    #[test]
    fn test_clip_timestamp_mapping_and_speed() {
        let clip = Clip {
            id: "clip-1".to_string(),
            name: "Footage".to_string(),
            clip_type: ClipType::Video,
            track_id: "track-v1".to_string(),
            source_id: Some("src-1".to_string()),
            source_in_ms: 1000,
            source_out_ms: 6000,
            timeline_in_ms: 2000,
            timeline_out_ms: 4500,
            speed_multiplier: 2.0, // 2x speed
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            keyframed_transform: None,
            chroma_key: ChromaKeySettings::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };

        assert_eq!(clip.duration_ms(), 2500);
        assert!(clip.contains_timestamp(2000));
        assert!(clip.contains_timestamp(3000));
        assert!(!clip.contains_timestamp(4500)); // Exclusive out point

        // At timeline 2500 (offset 500ms * 2.0x = 1000ms source advance)
        assert_eq!(clip.map_timeline_to_source_ms(2500), 2000);
    }

    #[test]
    fn test_keyframe_interpolation_curves() {
        // EaseIn curve: factor = t * t (starts slow, accelerates)
        let mut ease_in_prop = AnimatedProperty::constant(0.0);
        ease_in_prop.add_keyframe(0, 0.0, KeyframeCurve::EaseIn);
        ease_in_prop.add_keyframe(1000, 100.0, KeyframeCurve::EaseIn);

        // At t = 0.5 (500ms): 0.5^2 * 100 = 25.0
        let val_ease_in = ease_in_prop.evaluate_at(500);
        assert!((val_ease_in - 25.0).abs() < 1e-3);

        // EaseOut curve: factor = t * (2 - t) (starts fast, decelerates)
        let mut ease_out_prop = AnimatedProperty::constant(0.0);
        ease_out_prop.add_keyframe(0, 0.0, KeyframeCurve::EaseOut);
        ease_out_prop.add_keyframe(1000, 100.0, KeyframeCurve::EaseOut);

        // At t = 0.5 (500ms): 0.5 * (2 - 0.5) * 100 = 75.0
        let val_ease_out = ease_out_prop.evaluate_at(500);
        assert!((val_ease_out - 75.0).abs() < 1e-3);

        // EaseInOut curve: smoothstep
        let mut ease_in_out = AnimatedProperty::constant(0.0);
        ease_in_out.add_keyframe(0, 0.0, KeyframeCurve::EaseInOut);
        ease_in_out.add_keyframe(1000, 100.0, KeyframeCurve::EaseInOut);

        // At t = 0.5 (500ms): 2 * 0.5^2 * 100 = 50.0
        let val_ease_in_out = ease_in_out.evaluate_at(500);
        assert!((val_ease_in_out - 50.0).abs() < 1e-3);

        // Bezier curve: p1_y=0.1, p2_y=0.9
        let mut bezier_prop = AnimatedProperty::constant(0.0);
        bezier_prop.add_keyframe(
            0,
            0.0,
            KeyframeCurve::Bezier {
                p1_x: 0.25,
                p1_y: 0.1,
                p2_x: 0.25,
                p2_y: 0.9,
            },
        );
        bezier_prop.add_keyframe(1000, 100.0, KeyframeCurve::Linear);
        let val_bezier = bezier_prop.evaluate_at(500);
        assert!(val_bezier > 0.0 && val_bezier < 100.0);
    }

    #[test]
    fn test_keyframed_transform_evaluation() {
        let base = Transform2D {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            rotation_degrees: 0.0,
            anchor_x: 0.5,
            anchor_y: 0.5,
        };

        let mut kt = KeyframedTransform::default();
        let mut scale_x = AnimatedProperty::constant(1.0);
        scale_x.add_keyframe(0, 1.0, KeyframeCurve::Linear);
        scale_x.add_keyframe(1000, 2.0, KeyframeCurve::Linear);
        kt.scale_x = Some(scale_x);

        let mut rot = AnimatedProperty::constant(0.0);
        rot.add_keyframe(0, 0.0, KeyframeCurve::Linear);
        rot.add_keyframe(1000, 90.0, KeyframeCurve::Linear);
        kt.rotation_degrees = Some(rot);

        let evaluated_mid = kt.evaluate_at(500, &base);
        assert!((evaluated_mid.scale_x - 1.5).abs() < 1e-3);
        assert!((evaluated_mid.scale_y - 1.0).abs() < 1e-3); // Unanimated falls back to base
        assert!((evaluated_mid.rotation_degrees - 45.0).abs() < 1e-3);
    }

    #[test]
    fn test_project_duration_and_serialization() {
        let mut project = TimelineProject::new("proj-1", "Test Edit", ProjectSettings::default());
        let mut track = Track::new("t1", "V1", TrackType::Video, 0);

        let clip1 = Clip {
            id: "c1".to_string(),
            name: "Clip 1".to_string(),
            clip_type: ClipType::Video,
            track_id: "t1".to_string(),
            source_id: None,
            source_in_ms: 0,
            source_out_ms: 5000,
            timeline_in_ms: 0,
            timeline_out_ms: 5000,
            speed_multiplier: 1.0,
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            keyframed_transform: None,
            chroma_key: ChromaKeySettings::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };

        track.clips.push(clip1);
        project.tracks.push(track);

        assert_eq!(project.duration_ms(), 5000);

        let json = serde_json::to_string(&project).unwrap();
        let deserialized: TimelineProject = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.project_id, "proj-1");
        assert_eq!(deserialized.tracks.len(), 1);
        assert_eq!(deserialized.tracks[0].clips.len(), 1);
    }
}
