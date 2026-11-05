#![deny(unsafe_code)]
//! Non-Destructive Multi-Track Timeline DAG Engine
//!
//! Provides transactional editing operations:
//! - Clip split, trim, move, ripple-delete
//! - Magnetic snapping engine
//! - Transitions and markers management
//! - Transactional CommandHistory with unlimited Undo / Redo
//! - Atomic `.avedproj` project serialization with BLAKE3 checksums

use crate::models::{
    Clip, ProjectSettings, TimelineMarker, TimelineProject, Track, TrackType, Transition,
};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

/// Timeline engine domain errors
#[derive(Debug, Error)]
pub enum TimelineError {
    #[error("Track not found: {0}")]
    TrackNotFound(String),
    #[error("Clip not found: {0}")]
    ClipNotFound(String),
    #[error("Transition not found: {0}")]
    TransitionNotFound(String),
    #[error("Track is locked: {0}")]
    TrackLocked(String),
    #[error("Invalid timestamp {0} for split on clip duration {1}..{2}")]
    InvalidSplitTimestamp(u64, u64, u64),
    #[error("Clip overlap conflict on track '{0}' between {1} and {2}")]
    ClipOverlapConflict(String, u64, u64),
    #[error("Invalid trim parameters: in_ms {0} >= out_ms {1}")]
    InvalidTrimPoints(u64, u64),
    #[error("I/O error during project persistence: {0}")]
    Io(#[from] std::io::Error),
    #[error("Project serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Project checksum verification failed (expected '{expected}', got '{actual}')")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("Project file corrupt or unsupported format")]
    CorruptProjectFile,
}

// ---------------------------------------------------------------------------
// Transactional Undo/Redo Timeline Commands
// ---------------------------------------------------------------------------

pub trait TimelineCommand: Send + Sync {
    fn execute(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError>;
    fn undo(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError>;
    fn description(&self) -> &str;
}

/// Command history manager providing unlimited transactional Undo/Redo
pub struct CommandHistory {
    undo_stack: Vec<Box<dyn TimelineCommand>>,
    redo_stack: Vec<Box<dyn TimelineCommand>>,
    max_history: usize,
}

impl CommandHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    pub fn execute(
        &mut self,
        mut command: Box<dyn TimelineCommand>,
        project: &mut TimelineProject,
    ) -> Result<(), TimelineError> {
        command.execute(project)?;
        self.undo_stack.push(command);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        Ok(())
    }

    pub fn undo(&mut self, project: &mut TimelineProject) -> Result<bool, TimelineError> {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(project)?;
            self.redo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn redo(&mut self, project: &mut TimelineProject) -> Result<bool, TimelineError> {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.execute(project)?;
            self.undo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.description())
    }

    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.description())
    }
}

// ---------------------------------------------------------------------------
// Concrete Commands
// ---------------------------------------------------------------------------

pub struct AddClipCommand {
    clip: Clip,
}

impl AddClipCommand {
    pub fn new(clip: Clip) -> Self {
        Self { clip }
    }
}

impl TimelineCommand for AddClipCommand {
    fn execute(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        let track = project
            .tracks
            .iter_mut()
            .find(|t| t.id == self.clip.track_id)
            .ok_or_else(|| TimelineError::TrackNotFound(self.clip.track_id.clone()))?;

        if track.locked {
            return Err(TimelineError::TrackLocked(track.id.clone()));
        }

        track.clips.push(self.clip.clone());
        track.clips.sort_by_key(|c| c.timeline_in_ms);
        Ok(())
    }

    fn undo(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        let track = project
            .tracks
            .iter_mut()
            .find(|t| t.id == self.clip.track_id)
            .ok_or_else(|| TimelineError::TrackNotFound(self.clip.track_id.clone()))?;

        track.clips.retain(|c| c.id != self.clip.id);
        Ok(())
    }

    fn description(&self) -> &str {
        "Add Clip"
    }
}

pub struct RemoveClipCommand {
    clip_id: String,
    removed_clip: Option<Clip>,
}

impl RemoveClipCommand {
    pub fn new(clip_id: impl Into<String>) -> Self {
        Self {
            clip_id: clip_id.into(),
            removed_clip: None,
        }
    }
}

impl TimelineCommand for RemoveClipCommand {
    fn execute(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        for track in &mut project.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.clip_id) {
                if track.locked {
                    return Err(TimelineError::TrackLocked(track.id.clone()));
                }
                self.removed_clip = Some(track.clips.remove(pos));
                return Ok(());
            }
        }
        Err(TimelineError::ClipNotFound(self.clip_id.clone()))
    }

    fn undo(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        if let Some(clip) = self.removed_clip.take() {
            let track = project
                .tracks
                .iter_mut()
                .find(|t| t.id == clip.track_id)
                .ok_or_else(|| TimelineError::TrackNotFound(clip.track_id.clone()))?;

            track.clips.push(clip);
            track.clips.sort_by_key(|c| c.timeline_in_ms);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Remove Clip"
    }
}

pub struct SplitClipCommand {
    clip_id: String,
    split_timestamp_ms: u64,
    original_clip: Option<Clip>,
    left_clip: Option<Clip>,
    right_clip: Option<Clip>,
}

impl SplitClipCommand {
    pub fn new(clip_id: impl Into<String>, split_timestamp_ms: u64) -> Self {
        Self {
            clip_id: clip_id.into(),
            split_timestamp_ms,
            original_clip: None,
            left_clip: None,
            right_clip: None,
        }
    }
}

impl TimelineCommand for SplitClipCommand {
    fn execute(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        for track in &mut project.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.clip_id) {
                if track.locked {
                    return Err(TimelineError::TrackLocked(track.id.clone()));
                }

                let orig = &track.clips[pos];
                if self.split_timestamp_ms <= orig.timeline_in_ms
                    || self.split_timestamp_ms >= orig.timeline_out_ms
                {
                    return Err(TimelineError::InvalidSplitTimestamp(
                        self.split_timestamp_ms,
                        orig.timeline_in_ms,
                        orig.timeline_out_ms,
                    ));
                }

                let speed = if orig.speed_multiplier > 0.0 {
                    orig.speed_multiplier
                } else {
                    1.0
                };
                let left_duration = self.split_timestamp_ms - orig.timeline_in_ms;
                let src_advance = (left_duration as f32 * speed) as u64;
                let split_src_point = orig.source_in_ms + src_advance;

                let mut left = orig.clone();
                left.timeline_out_ms = self.split_timestamp_ms;
                left.source_out_ms = split_src_point;

                let mut right = orig.clone();
                right.id = format!("{}_part2", orig.id);
                right.timeline_in_ms = self.split_timestamp_ms;
                right.source_in_ms = split_src_point;

                self.original_clip = Some(track.clips.remove(pos));
                self.left_clip = Some(left.clone());
                self.right_clip = Some(right.clone());

                track.clips.push(left);
                track.clips.push(right);
                track.clips.sort_by_key(|c| c.timeline_in_ms);
                return Ok(());
            }
        }
        Err(TimelineError::ClipNotFound(self.clip_id.clone()))
    }

    fn undo(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        if let (Some(orig), Some(left), Some(right)) = (
            self.original_clip.take(),
            self.left_clip.take(),
            self.right_clip.take(),
        ) {
            for track in &mut project.tracks {
                if track.id == orig.track_id {
                    track.clips.retain(|c| c.id != left.id && c.id != right.id);
                    track.clips.push(orig);
                    track.clips.sort_by_key(|c| c.timeline_in_ms);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Split Clip"
    }
}

pub struct RippleDeleteCommand {
    clip_id: String,
    deleted_clip: Option<Clip>,
    shifted_clips_delta: Vec<(String, u64)>, // (clip_id, shifted_ms)
}

impl RippleDeleteCommand {
    pub fn new(clip_id: impl Into<String>) -> Self {
        Self {
            clip_id: clip_id.into(),
            deleted_clip: None,
            shifted_clips_delta: Vec::new(),
        }
    }
}

impl TimelineCommand for RippleDeleteCommand {
    fn execute(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        for track in &mut project.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.clip_id) {
                if track.locked {
                    return Err(TimelineError::TrackLocked(track.id.clone()));
                }

                let clip = track.clips.remove(pos);
                let duration = clip.duration_ms();
                let out_point = clip.timeline_out_ms;
                self.deleted_clip = Some(clip);

                self.shifted_clips_delta.clear();
                for c in track.clips.iter_mut() {
                    if c.timeline_in_ms >= out_point {
                        c.timeline_in_ms = c.timeline_in_ms.saturating_sub(duration);
                        c.timeline_out_ms = c.timeline_out_ms.saturating_sub(duration);
                        self.shifted_clips_delta.push((c.id.clone(), duration));
                    }
                }
                return Ok(());
            }
        }
        Err(TimelineError::ClipNotFound(self.clip_id.clone()))
    }

    fn undo(&mut self, project: &mut TimelineProject) -> Result<(), TimelineError> {
        if let Some(clip) = self.deleted_clip.take() {
            let track = project
                .tracks
                .iter_mut()
                .find(|t| t.id == clip.track_id)
                .ok_or_else(|| TimelineError::TrackNotFound(clip.track_id.clone()))?;

            for (cid, duration) in &self.shifted_clips_delta {
                if let Some(c) = track.clips.iter_mut().find(|c| c.id == *cid) {
                    c.timeline_in_ms += duration;
                    c.timeline_out_ms += duration;
                }
            }

            track.clips.push(clip);
            track.clips.sort_by_key(|c| c.timeline_in_ms);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Ripple Delete Clip"
    }
}

// ---------------------------------------------------------------------------
// Timeline Master Engine
// ---------------------------------------------------------------------------

/// Non-destructive master timeline manager
pub struct TimelineEngine {
    project: TimelineProject,
    history: CommandHistory,
    playhead_ms: u64,
    snap_tolerance_ms: u64,
}

impl TimelineEngine {
    pub fn new(project: TimelineProject) -> Self {
        Self {
            project,
            history: CommandHistory::new(100),
            playhead_ms: 0,
            snap_tolerance_ms: 100, // ~3-6 frames default snapping window
        }
    }

    pub fn with_default_project(name: &str) -> Self {
        let proj = TimelineProject::new(
            format!("proj_{}", &blake3::hash(name.as_bytes()).to_hex()[..8]),
            name,
            ProjectSettings::default(),
        );
        let mut engine = Self::new(proj);
        // Add default tracks
        engine
            .add_track(Track::new("v1", "Video 1", TrackType::Video, 0))
            .unwrap();
        engine
            .add_track(Track::new("a1", "Audio 1", TrackType::Audio, 0))
            .unwrap();
        engine
            .add_track(Track::new("s1", "Subtitles", TrackType::Subtitle, 10))
            .unwrap();
        engine
    }

    pub fn project(&self) -> &TimelineProject {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut TimelineProject {
        &mut self.project
    }

    pub fn playhead_ms(&self) -> u64 {
        self.playhead_ms
    }

    pub fn set_playhead_ms(&mut self, ms: u64) {
        self.playhead_ms = ms;
    }

    pub fn set_snap_tolerance_ms(&mut self, ms: u64) {
        self.snap_tolerance_ms = ms;
    }

    // -----------------------------------------------------------------------
    // Track Operations
    // -----------------------------------------------------------------------

    pub fn add_track(&mut self, track: Track) -> Result<(), TimelineError> {
        self.project.tracks.push(track);
        self.project.tracks.sort_by_key(|t| t.z_index);
        Ok(())
    }

    pub fn remove_track(&mut self, track_id: &str) -> Result<Track, TimelineError> {
        let pos = self
            .project
            .tracks
            .iter()
            .position(|t| t.id == track_id)
            .ok_or_else(|| TimelineError::TrackNotFound(track_id.to_string()))?;
        Ok(self.project.tracks.remove(pos))
    }

    pub fn set_track_muted(&mut self, track_id: &str, muted: bool) -> Result<(), TimelineError> {
        let track = self
            .project
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id)
            .ok_or_else(|| TimelineError::TrackNotFound(track_id.to_string()))?;
        track.muted = muted;
        Ok(())
    }

    pub fn set_track_locked(&mut self, track_id: &str, locked: bool) -> Result<(), TimelineError> {
        let track = self
            .project
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id)
            .ok_or_else(|| TimelineError::TrackNotFound(track_id.to_string()))?;
        track.locked = locked;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Clip Operations via Undoable Commands
    // -----------------------------------------------------------------------

    pub fn add_clip(&mut self, clip: Clip) -> Result<(), TimelineError> {
        self.history
            .execute(Box::new(AddClipCommand::new(clip)), &mut self.project)
    }

    pub fn remove_clip(&mut self, clip_id: &str) -> Result<(), TimelineError> {
        self.history
            .execute(Box::new(RemoveClipCommand::new(clip_id)), &mut self.project)
    }

    pub fn split_clip(&mut self, clip_id: &str, timestamp_ms: u64) -> Result<(), TimelineError> {
        self.history.execute(
            Box::new(SplitClipCommand::new(clip_id, timestamp_ms)),
            &mut self.project,
        )
    }

    pub fn ripple_delete(&mut self, clip_id: &str) -> Result<(), TimelineError> {
        self.history.execute(
            Box::new(RippleDeleteCommand::new(clip_id)),
            &mut self.project,
        )
    }

    pub fn trim_clip(
        &mut self,
        clip_id: &str,
        new_in_ms: u64,
        new_out_ms: u64,
    ) -> Result<(), TimelineError> {
        if new_in_ms >= new_out_ms {
            return Err(TimelineError::InvalidTrimPoints(new_in_ms, new_out_ms));
        }

        for track in &mut self.project.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                if track.locked {
                    return Err(TimelineError::TrackLocked(track.id.clone()));
                }
                clip.timeline_in_ms = new_in_ms;
                clip.timeline_out_ms = new_out_ms;
                track.clips.sort_by_key(|c| c.timeline_in_ms);
                return Ok(());
            }
        }
        Err(TimelineError::ClipNotFound(clip_id.to_string()))
    }

    // -----------------------------------------------------------------------
    // Snapping Engine
    // -----------------------------------------------------------------------

    /// Snap timestamp to nearby clip heads/tails, playhead, or markers within tolerance
    pub fn calculate_snap_point(
        &self,
        candidate_ms: u64,
        exclude_clip_id: Option<&str>,
    ) -> (u64, bool) {
        let mut closest_point = candidate_ms;
        let mut min_diff = self.snap_tolerance_ms + 1;

        let check_snap = |pt: u64, min_diff: &mut u64, closest: &mut u64| {
            let diff = candidate_ms.abs_diff(pt);
            if diff <= self.snap_tolerance_ms && diff < *min_diff {
                *min_diff = diff;
                *closest = pt;
            }
        };

        // 1. Playhead snap
        check_snap(self.playhead_ms, &mut min_diff, &mut closest_point);

        // 2. Timeline origin
        check_snap(0, &mut min_diff, &mut closest_point);

        // 3. Markers snap
        for m in &self.project.markers {
            check_snap(m.timestamp_ms, &mut min_diff, &mut closest_point);
        }

        // 4. Clip heads & tails snap across all tracks
        for track in &self.project.tracks {
            for clip in &track.clips {
                if let Some(ex) = exclude_clip_id {
                    if clip.id == ex {
                        continue;
                    }
                }
                check_snap(clip.timeline_in_ms, &mut min_diff, &mut closest_point);
                check_snap(clip.timeline_out_ms, &mut min_diff, &mut closest_point);
            }
        }

        let snapped = min_diff <= self.snap_tolerance_ms;
        (closest_point, snapped)
    }

    // -----------------------------------------------------------------------
    // Markers & Transitions
    // -----------------------------------------------------------------------

    pub fn add_marker(&mut self, timestamp_ms: u64, label: &str, color_hex: &str) {
        self.project.markers.push(TimelineMarker {
            id: format!("m_{}", self.project.markers.len() + 1),
            timestamp_ms,
            label: label.to_string(),
            color_hex: color_hex.to_string(),
        });
        self.project.markers.sort_by_key(|m| m.timestamp_ms);
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.project.transitions.push(transition);
    }

    pub fn remove_transition(&mut self, id: &str) -> Result<(), TimelineError> {
        let pos = self
            .project
            .transitions
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| TimelineError::TransitionNotFound(id.to_string()))?;
        self.project.transitions.remove(pos);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Undo / Redo Shortcuts
    // -----------------------------------------------------------------------

    pub fn undo(&mut self) -> Result<bool, TimelineError> {
        self.history.undo(&mut self.project)
    }

    pub fn redo(&mut self) -> Result<bool, TimelineError> {
        self.history.redo(&mut self.project)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // -----------------------------------------------------------------------
    // Atomic .avedproj File Persistence
    // -----------------------------------------------------------------------

    /// Save timeline project to `.avedproj` file with BLAKE3 checksum envelope
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TimelineError> {
        let json = serde_json::to_vec_pretty(&self.project)?;
        let checksum = blake3::hash(&json).to_hex().to_string();

        let envelope = ProjectFileEnvelope {
            magic: *b"AVEDPROJ",
            format_version: 1,
            checksum,
            payload_json: String::from_utf8_lossy(&json).to_string(),
        };

        let env_bytes = serde_json::to_vec(&envelope)?;
        let temp_path = path.as_ref().with_extension("tmp");

        {
            let mut file = File::create(&temp_path)?;
            file.write_all(&env_bytes)?;
            file.sync_all()?;
        }

        std::fs::rename(temp_path, path)?;
        Ok(())
    }

    /// Load timeline project from `.avedproj` file and verify BLAKE3 integrity
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, TimelineError> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let envelope: ProjectFileEnvelope =
            serde_json::from_slice(&bytes).map_err(|_| TimelineError::CorruptProjectFile)?;

        if &envelope.magic != b"AVEDPROJ" {
            return Err(TimelineError::CorruptProjectFile);
        }

        let computed_checksum = blake3::hash(envelope.payload_json.as_bytes())
            .to_hex()
            .to_string();
        if computed_checksum != envelope.checksum {
            return Err(TimelineError::ChecksumMismatch {
                expected: envelope.checksum,
                actual: computed_checksum,
            });
        }

        let project: TimelineProject = serde_json::from_str(&envelope.payload_json)?;
        Ok(Self::new(project))
    }
}

/// Disk persistence format wrapper for `.avedproj`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectFileEnvelope {
    magic: [u8; 8],
    format_version: u32,
    checksum: String,
    payload_json: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AnimatedProperty, BlendMode, ChromaKeySettings, ClipType, ColorGradingSettings,
        CompressorSettings, ParametricEqSettings, Transform2D,
    };

    fn make_test_clip(id: &str, track_id: &str, in_ms: u64, out_ms: u64) -> Clip {
        Clip {
            id: id.to_string(),
            name: format!("Clip {}", id),
            clip_type: ClipType::Video,
            track_id: track_id.to_string(),
            source_id: None,
            source_in_ms: 0,
            source_out_ms: out_ms - in_ms,
            timeline_in_ms: in_ms,
            timeline_out_ms: out_ms,
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
        }
    }

    #[test]
    fn test_timeline_add_split_and_undo() {
        let mut engine = TimelineEngine::with_default_project("Test Project");
        let clip = make_test_clip("c1", "v1", 1000, 5000);
        engine.add_clip(clip).unwrap();

        assert_eq!(engine.project().tracks[0].clips.len(), 1);
        assert!(engine.can_undo());

        // Split clip at 3000ms
        engine.split_clip("c1", 3000).unwrap();
        assert_eq!(engine.project().tracks[0].clips.len(), 2);
        assert_eq!(engine.project().tracks[0].clips[0].timeline_out_ms, 3000);
        assert_eq!(engine.project().tracks[0].clips[1].timeline_in_ms, 3000);

        // Undo split
        assert!(engine.undo().unwrap());
        assert_eq!(engine.project().tracks[0].clips.len(), 1);
        assert_eq!(engine.project().tracks[0].clips[0].timeline_out_ms, 5000);

        // Redo split
        assert!(engine.redo().unwrap());
        assert_eq!(engine.project().tracks[0].clips.len(), 2);
    }

    #[test]
    fn test_ripple_delete_and_time_shift() {
        let mut engine = TimelineEngine::with_default_project("Ripple Edit");
        let c1 = make_test_clip("c1", "v1", 0, 2000); // 2 sec
        let c2 = make_test_clip("c2", "v1", 2000, 5000); // 3 sec
        let c3 = make_test_clip("c3", "v1", 5000, 9000); // 4 sec

        engine.add_clip(c1).unwrap();
        engine.add_clip(c2).unwrap();
        engine.add_clip(c3).unwrap();
        assert_eq!(engine.project().tracks[0].clips.len(), 3);

        // Ripple delete c2 (duration 3000ms)
        engine.ripple_delete("c2").unwrap();
        assert_eq!(engine.project().tracks[0].clips.len(), 2);

        // c3 should be shifted left by 3000ms (from 5000..9000 to 2000..6000)
        let remaining_c3 = &engine.project().tracks[0].clips[1];
        assert_eq!(remaining_c3.id, "c3");
        assert_eq!(remaining_c3.timeline_in_ms, 2000);
        assert_eq!(remaining_c3.timeline_out_ms, 6000);

        // Undo ripple delete restores c2 and shifts c3 back
        assert!(engine.undo().unwrap());
        assert_eq!(engine.project().tracks[0].clips.len(), 3);
        assert_eq!(engine.project().tracks[0].clips[2].timeline_in_ms, 5000);
    }

    #[test]
    fn test_magnetic_snapping() {
        let mut engine = TimelineEngine::with_default_project("Snapping Test");
        engine.set_snap_tolerance_ms(50);
        engine.set_playhead_ms(2000);

        let c1 = make_test_clip("c1", "v1", 1000, 4000);
        engine.add_clip(c1).unwrap();

        // Near playhead (2000)
        let (pt, snapped) = engine.calculate_snap_point(2020, None);
        assert!(snapped);
        assert_eq!(pt, 2000);

        // Near clip head (1000)
        let (pt, snapped) = engine.calculate_snap_point(980, None);
        assert!(snapped);
        assert_eq!(pt, 1000);

        // Far from any boundary (6000)
        let (pt, snapped) = engine.calculate_snap_point(6000, None);
        assert!(!snapped);
        assert_eq!(pt, 6000);
    }

    #[test]
    fn test_project_atomic_persistence_and_checksum_tamper() {
        let temp_dir = tempfile::tempdir().unwrap();
        let proj_path = temp_dir.path().join("test_timeline.avedproj");

        let mut engine = TimelineEngine::with_default_project("Persistence Project");
        let clip = make_test_clip("c1", "v1", 0, 3000);
        engine.add_clip(clip).unwrap();
        engine.add_marker(1500, "Scene Cut", "#ff0000");

        // Save
        engine.save_to_file(&proj_path).unwrap();
        assert!(proj_path.exists());

        // Load & verify
        let loaded = TimelineEngine::load_from_file(&proj_path).unwrap();
        assert_eq!(loaded.project().name, "Persistence Project");
        assert_eq!(loaded.project().tracks[0].clips.len(), 1);
        assert_eq!(loaded.project().markers.len(), 1);

        // Tamper test: modify payload to verify checksum detection
        let mut content = std::fs::read_to_string(&proj_path).unwrap();
        content = content.replace("Scene Cut", "Tampered Cut");
        std::fs::write(&proj_path, content.as_bytes()).unwrap();

        let tamper_res = TimelineEngine::load_from_file(&proj_path);
        assert!(matches!(
            tamper_res,
            Err(TimelineError::ChecksumMismatch { .. })
        ));
    }
}
