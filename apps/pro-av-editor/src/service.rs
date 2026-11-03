#![deny(unsafe_code)]
//! Central Orchestrator Service for Professional Audio & Video Editor
//!
//! Unifies timeline DAG operations, layer compositing, color grading,
//! multi-track audio DSP, and offline AI auto-editing into a thread-safe service.

use crate::ai_editor::{AiSilenceCutter, BeatDetector, SilenceCutConfig};
use crate::audio_engine::{AudioEngine, AudioError, LoudnessMeterData};
use crate::color::{VideoScopeData, VideoScopeGenerator};
use crate::compositor::{CompositorError, LayerCompositor, VideoFrame};
use crate::models::{Clip, CompressorSettings, ParametricEqSettings, TimelineProject, Track};
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

        let beats = BeatDetector::detect_beats(&audio_samples, self.sample_rate, sensitivity);
        let mut guard = self
            .engine
            .write()
            .map_err(|_| ServiceError::LockPoisoned)?;
        let count = beats.len();
        for b in beats {
            guard.add_marker(b, "Beat", "#f59e0b");
        }

        Ok(count)
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
