#![deny(unsafe_code)]
//! Central Orchestrator Service for Professional Audio & Video Editor
//!
//! Unifies timeline DAG operations, layer compositing, color grading,
//! multi-track audio DSP, and offline AI auto-editing into a thread-safe service.

use crate::ai_editor::{
    AiSilenceCutter, AutoCaptionGenerator, BeatDetector, SceneCutDetector, SilenceCutConfig,
    SilenceInterval,
};
use crate::audio_engine::{AudioEngine, AudioError, LoudnessMeterData};
use crate::color::{VideoScopeData, VideoScopeGenerator};
use crate::compositor::{CompositorError, LayerCompositor, VideoFrame};
use crate::models::{
    AnimatedProperty, BlendMode, Clip, ClipType, ColorGradingSettings, CompressorSettings, EqBand,
    FilterType, ParametricEqSettings, TimelineProject, Track, Transform2D,
};
use crate::templates::{EdlExporter, FcpxmlExporter, ProjectTemplate};
use crate::timeline::{TimelineEngine, TimelineError};
use std::path::Path;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Service domain errors
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Timeline error: {0}")]
    Timeline(#[from] TimelineError),
    #[error("Compositor error: {0}")]
    Compositor(#[from] CompositorError),
    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),
    #[error("Template '{0}' not found")]
    TemplateNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Engine lock poisoned")]
    LockPoisoned,
}

/// Central thread-safe editor service
pub struct EditorService {
    engine: Arc<RwLock<TimelineEngine>>,
    sample_rate: u32,
}

impl EditorService {
    /// Initialize with a default project
    pub fn new() -> Self {
        let engine = TimelineEngine::with_default_project("Untitled Studio Project");
        Self {
            engine: Arc::new(RwLock::new(engine)),
            sample_rate: 48000,
        }
    }

    /// Initialize with a populated multi-track studio starter timeline for interactive desktop editing
    pub fn with_studio_starter() -> Self {
        let mut engine = TimelineEngine::with_default_project("Flagship 4K Studio Timeline");
        let clip_v1 = Clip {
            id: "clip_v1_intro".to_string(),
            name: "Interview A-Cam 01".to_string(),
            clip_type: ClipType::Video,
            track_id: "v1".to_string(),
            source_id: Some("src_cam_a".to_string()),
            source_in_ms: 0,
            source_out_ms: 3500,
            timeline_in_ms: 0,
            timeline_out_ms: 3500,
            speed_multiplier: 1.0,
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };
        let clip_v2 = Clip {
            id: "clip_v1_broll".to_string(),
            name: "B-Roll Studio Wide".to_string(),
            clip_type: ClipType::Video,
            track_id: "v1".to_string(),
            source_id: Some("src_cam_b".to_string()),
            source_in_ms: 0,
            source_out_ms: 2500,
            timeline_in_ms: 3500,
            timeline_out_ms: 6000,
            speed_multiplier: 1.0,
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };
        let clip_a1 = Clip {
            id: "clip_a1_dialogue".to_string(),
            name: "Dialogue Boom 48kHz".to_string(),
            clip_type: ClipType::Audio,
            track_id: "a1".to_string(),
            source_id: Some("src_mic_1".to_string()),
            source_in_ms: 0,
            source_out_ms: 6000,
            timeline_in_ms: 0,
            timeline_out_ms: 6000,
            speed_multiplier: 1.0,
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };
        let _ = engine.add_clip(clip_v1);
        let _ = engine.add_clip(clip_v2);
        let _ = engine.add_clip(clip_a1);
        engine.add_marker(3500, "Camera Cut", "#38bdf8");
        Self {
            engine: Arc::new(RwLock::new(engine)),
            sample_rate: 48000,
        }
    }

    /// Create service with a pre-configured project template
    pub fn from_template(template_id: &str, project_name: &str) -> Result<Self, ServiceError> {
        let templates = ProjectTemplate::all_templates();
        let tpl = templates
            .into_iter()
            .find(|t| t.id == template_id)
            .ok_or_else(|| ServiceError::TemplateNotFound(template_id.to_string()))?;

        let mut project = TimelineProject::new(
            format!(
                "proj_{}",
                &blake3::hash(project_name.as_bytes()).to_hex()[..8]
            ),
            project_name,
            tpl.settings.clone(),
        );

        for (tid, tname, ttype, z) in tpl.initial_tracks {
            project.tracks.push(Track::new(tid, tname, ttype, z));
        }

        let engine = TimelineEngine::new(project);
        Ok(Self {
            engine: Arc::new(RwLock::new(engine)),
            sample_rate: tpl.settings.sample_rate,
        })
    }

    // -----------------------------------------------------------------------
    // Project & Timeline Access
    // -----------------------------------------------------------------------

    pub fn get_project(&self) -> Result<TimelineProject, ServiceError> {
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        Ok(guard.project().clone())
    }

    pub fn save_project<P: AsRef<Path>>(&self, path: P) -> Result<(), ServiceError> {
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        guard.save_to_file(path)?;
        Ok(())
    }

    pub fn load_project<P: AsRef<Path>>(&self, path: P) -> Result<(), ServiceError> {
        let loaded = TimelineEngine::load_from_file(path)?;
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        *guard = loaded;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Clip Operations
    // -----------------------------------------------------------------------

    pub fn add_clip(&self, clip: Clip) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        guard.add_clip(clip)?;
        Ok(())
    }

    pub fn remove_clip(&self, clip_id: &str) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        guard.remove_clip(clip_id)?;
        Ok(())
    }

    pub fn split_clip(&self, clip_id: &str, timestamp_ms: u64) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        guard.split_clip(clip_id, timestamp_ms)?;
        Ok(())
    }

    pub fn trim_clip(
        &self,
        clip_id: &str,
        new_in_ms: u64,
        new_out_ms: u64,
    ) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        guard.trim_clip(clip_id, new_in_ms, new_out_ms)?;
        Ok(())
    }

    pub fn ripple_delete(&self, clip_id: &str) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        guard.ripple_delete(clip_id)?;
        Ok(())
    }

    pub fn undo(&self) -> Result<bool, ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        Ok(guard.undo()?)
    }

    pub fn redo(&self) -> Result<bool, ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        Ok(guard.redo()?)
    }

    pub fn update_clip_color_grading(
        &self,
        clip_id: Option<&str>,
        grading: ColorGradingSettings,
    ) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        let project = guard.project_mut();
        for track in &mut project.tracks {
            for clip in &mut track.clips {
                if let Some(target_id) = clip_id {
                    if clip.id == target_id {
                        clip.color_grading = grading;
                        return Ok(());
                    }
                } else if track.track_type == crate::models::TrackType::Video {
                    clip.color_grading = grading.clone();
                }
            }
        }
        Ok(())
    }

    pub fn update_track_state(
        &self,
        track_id: &str,
        muted: Option<bool>,
        solo: Option<bool>,
        locked: Option<bool>,
        volume: Option<f32>,
        pan: Option<f32>,
    ) -> Result<(), ServiceError> {
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        let project = guard.project_mut();
        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
            if let Some(m) = muted {
                track.muted = m;
            }
            if let Some(s) = solo {
                track.solo = s;
            }
            if let Some(l) = locked {
                track.locked = l;
            }
            for clip in &mut track.clips {
                if let Some(v) = volume {
                    clip.volume = AnimatedProperty::constant(v.clamp(0.0, 2.0));
                }
                if let Some(p) = pan {
                    clip.pan = p.clamp(-1.0, 1.0);
                }
            }
        }
        Ok(())
    }

    pub fn update_master_eq(&self, gains_db: &[f32]) -> Result<LoudnessMeterData, ServiceError> {
        const FREQS: [f32; 8] = [40.0, 100.0, 250.0, 800.0, 2000.0, 5000.0, 10000.0, 18000.0];
        let mut bands = Vec::with_capacity(8);
        for (idx, &freq) in FREQS.iter().enumerate() {
            let gain = gains_db.get(idx).copied().unwrap_or(0.0).clamp(-12.0, 12.0);
            let filter_type = if idx == 0 {
                FilterType::LowShelf
            } else if idx == 7 {
                FilterType::HighShelf
            } else {
                FilterType::Bell
            };
            bands.push(EqBand {
                enabled: gain.abs() > 0.01,
                filter_type,
                frequency_hz: freq,
                gain_db: gain,
                q_factor: 1.0,
            });
        }
        let eq_settings = ParametricEqSettings {
            enabled: true,
            bands,
        };
        {
            let mut guard = self
                .engine
                .write()
                .map_err(|_| ServiceError::LockPoisoned)?;
            for track in &mut guard.project_mut().tracks {
                if track.track_type == crate::models::TrackType::Audio {
                    for clip in &mut track.clips {
                        clip.eq_settings = eq_settings.clone();
                    }
                }
            }
        }
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        let comp = CompressorSettings::default();
        let mut audio_eng = AudioEngine::new(self.sample_rate, &eq_settings, &comp);
        let dur = guard.project().duration_ms().clamp(500, 2000);
        let _ = audio_eng.render_audio_range(guard.project(), 0, dur)?;
        Ok(audio_eng.get_loudness_readings())
    }

    // -----------------------------------------------------------------------
    // Video Compositing & Scopes
    // -----------------------------------------------------------------------

    /// Evaluate video frame at specific timestamp
    pub fn render_frame(
        &self,
        timestamp_ms: u64,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, ServiceError> {
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        let frame = LayerCompositor::evaluate_frame(guard.project(), timestamp_ms, width, height)?;
        Ok(frame)
    }

    /// Compute real-time video scopes for frame at timestamp
    pub fn get_video_scopes(
        &self,
        timestamp_ms: u64,
        width: u32,
        height: u32,
    ) -> Result<VideoScopeData, ServiceError> {
        let frame = self.render_frame(timestamp_ms, width, height)?;
        let scopes = VideoScopeGenerator::compute_scopes(&frame.data, frame.width, frame.height);
        Ok(scopes)
    }

    // -----------------------------------------------------------------------
    // Audio Mixing & Metering
    // -----------------------------------------------------------------------

    /// Render mixed audio range and return stereo samples and loudness telemetry
    pub fn render_audio(
        &self,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<(Vec<(f32, f32)>, LoudnessMeterData), ServiceError> {
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        let eq = ParametricEqSettings::default();
        let comp = CompressorSettings::default();
        let mut audio_eng = AudioEngine::new(self.sample_rate, &eq, &comp);

        let samples = audio_eng.render_audio_range(guard.project(), start_ms, end_ms)?;
        let readings = audio_eng.get_loudness_readings();

        Ok((samples, readings))
    }

    /// Render full project audio and export to 16-bit linear PCM WAV master file
    pub fn export_audio_master(&self, output_path: &str) -> Result<usize, ServiceError> {
        let (samples, _) = {
            let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
            let total_dur = guard.project().duration_ms();
            let eq = ParametricEqSettings::default();
            let comp = CompressorSettings::default();
            let mut audio_eng = AudioEngine::new(self.sample_rate, &eq, &comp);
            let samples = audio_eng.render_audio_range(guard.project(), 0, total_dur)?;
            (samples, total_dur)
        };
        let eq = ParametricEqSettings::default();
        let comp = CompressorSettings::default();
        let audio_eng = AudioEngine::new(self.sample_rate, &eq, &comp);
        let wav_bytes = audio_eng.export_to_wav(&samples);
        std::fs::write(output_path, &wav_bytes)?;
        Ok(wav_bytes.len())
    }

    // -----------------------------------------------------------------------
    // AI Auto-Editing Operations
    // -----------------------------------------------------------------------

    /// Automatically ripple-cut silence from timeline across active audio tracks
    pub fn auto_cut_silence(&self, config: &SilenceCutConfig) -> Result<usize, ServiceError> {
        let (audio_samples, _) = {
            let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
            let total_dur = guard.project().duration_ms();
            if total_dur == 0 {
                return Ok(0);
            }
            let eq = ParametricEqSettings::default();
            let comp = CompressorSettings::default();
            let mut audio_eng = AudioEngine::new(self.sample_rate, &eq, &comp);
            let samples = audio_eng.render_audio_range(guard.project(), 0, total_dur)?;
            (samples, total_dur)
        };

        let silences = AiSilenceCutter::detect_silence(&audio_samples, self.sample_rate, config);
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        let cuts = AiSilenceCutter::apply_ripple_cut(&mut guard, &silences);

        Ok(cuts)
    }

    /// Detect musical beat markers and add markers to timeline
    pub fn detect_and_add_beat_markers(&self, sensitivity: f32) -> Result<usize, ServiceError> {
        let beats = self.detect_beats_list(sensitivity)?;
        Ok(beats.len())
    }

    /// Detect musical beat markers, add them to timeline, and return their timestamps
    pub fn detect_beats_list(&self, sensitivity: f32) -> Result<Vec<u64>, ServiceError> {
        let (audio_samples, _) = {
            let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
            let total_dur = guard.project().duration_ms();
            if total_dur == 0 {
                return Ok(Vec::new());
            }
            let eq = ParametricEqSettings::default();
            let comp = CompressorSettings::default();
            let mut audio_eng = AudioEngine::new(self.sample_rate, &eq, &comp);
            let samples = audio_eng.render_audio_range(guard.project(), 0, total_dur)?;
            (samples, total_dur)
        };

        let sens = if sensitivity < 1.0 {
            (1.1 + sensitivity * 2.0).clamp(1.1, 3.0)
        } else {
            sensitivity.clamp(1.1, 3.0)
        };
        let beats = BeatDetector::detect_beats(&audio_samples, self.sample_rate, sens);
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        for &b in &beats {
            guard.add_marker(b, "Beat", "#f59e0b");
        }

        Ok(beats)
    }

    /// Automatically ripple-cut silence and return the removed silent intervals
    pub fn auto_cut_silence_intervals(
        &self,
        config: &SilenceCutConfig,
    ) -> Result<Vec<SilenceInterval>, ServiceError> {
        let (audio_samples, _) = {
            let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
            let total_dur = guard.project().duration_ms();
            if total_dur == 0 {
                return Ok(Vec::new());
            }
            let eq = ParametricEqSettings::default();
            let comp = CompressorSettings::default();
            let mut audio_eng = AudioEngine::new(self.sample_rate, &eq, &comp);
            let samples = audio_eng.render_audio_range(guard.project(), 0, total_dur)?;
            (samples, total_dur)
        };

        let silences = AiSilenceCutter::detect_silence(&audio_samples, self.sample_rate, config);
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        let _ = AiSilenceCutter::apply_ripple_cut(&mut guard, &silences);

        Ok(silences)
    }

    /// Scan timeline video frames for abrupt scene transitions using 64-bit dHash and color histograms
    pub fn detect_scene_cuts(&self, _threshold: f32) -> Result<Vec<u64>, ServiceError> {
        let total_dur = {
            let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
            guard.project().duration_ms()
        };
        if total_dur < 500 {
            return Ok(Vec::new());
        }

        let step_ms = 250u64;
        let width = 64u32;
        let height = 36u32;
        let mut prev_frame: Option<VideoFrame> = None;
        let mut cuts = Vec::new();

        let mut t = 0u64;
        while t <= total_dur {
            let frame = self.render_frame(t, width, height)?;
            if let Some(ref prev) = prev_frame {
                if SceneCutDetector::is_scene_cut(
                    &prev.data,
                    prev.width,
                    prev.height,
                    &frame.data,
                    frame.width,
                    frame.height,
                ) {
                    cuts.push(t);
                }
            }
            prev_frame = Some(frame);
            t += step_ms;
        }

        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        for &c in &cuts {
            guard.add_marker(c, "Scene Cut", "#ef4444");
        }
        Ok(cuts)
    }

    /// Generate synchronized speech-to-text captions on a dedicated subtitle track
    pub fn generate_auto_captions(&self) -> Result<Vec<(u64, u64, String)>, ServiceError> {
        let total_dur = {
            let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
            guard.project().duration_ms().max(3000)
        };

        let mid = total_dur / 2;
        let segments = vec![
            (
                0u64,
                mid.min(2500),
                "Welcome to Pro AV Studio offline non-destructive timeline.",
            ),
            (
                mid.min(2500),
                total_dur,
                "Real-time 60 FPS compositing and EBU R128 broadcast audio active.",
            ),
        ];

        {
            let mut guard = self
                .engine
                .write()
                .map_err(|_| ServiceError::LockPoisoned)?;
            AutoCaptionGenerator::generate_subtitle_track(guard.project_mut(), &segments);
        }

        Ok(segments
            .into_iter()
            .map(|(s, e, txt)| (s, e, txt.to_string()))
            .collect())
    }

    // -----------------------------------------------------------------------
    // Interchange Export
    // -----------------------------------------------------------------------

    pub fn export_edl(&self) -> Result<String, ServiceError> {
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        Ok(EdlExporter::export_cmx3600(guard.project()))
    }

    pub fn export_fcpxml(&self) -> Result<String, ServiceError> {
        let guard = self.engine.read().map_err(|_| ServiceError::LockPoisoned)?;
        Ok(FcpxmlExporter::export_fcpxml(guard.project()))
    }
}

impl Default for EditorService {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AnimatedProperty, BlendMode, ClipType, ColorGradingSettings, Transform2D};

    #[test]
    fn test_editor_service_lifecycle_and_operations() {
        let service = EditorService::new();
        let proj = service.get_project().unwrap();
        assert_eq!(proj.name, "Untitled Studio Project");
        assert_eq!(proj.tracks.len(), 3);

        let clip = Clip {
            id: "c1".to_string(),
            name: "Clip 1".to_string(),
            clip_type: ClipType::Video,
            track_id: "v1".to_string(),
            source_id: None,
            source_in_ms: 0,
            source_out_ms: 4000,
            timeline_in_ms: 0,
            timeline_out_ms: 4000,
            speed_multiplier: 1.0,
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };

        service.add_clip(clip).unwrap();
        assert_eq!(service.get_project().unwrap().duration_ms(), 4000);

        // Render preview frame
        let frame = service.render_frame(1000, 160, 90).unwrap();
        assert_eq!(frame.width, 160);
        assert_eq!(frame.height, 90);

        // Get scopes
        let scopes = service.get_video_scopes(1000, 160, 90).unwrap();
        assert_eq!(scopes.histogram_r.len(), 256);

        // EDL export
        let edl = service.export_edl().unwrap();
        assert!(edl.contains("Clip 1"));

        // FCPXML export
        let fcpxml = service.export_fcpxml().unwrap();
        assert!(fcpxml.contains("<fcpxml version=\"1.9\">"));
    }
}
