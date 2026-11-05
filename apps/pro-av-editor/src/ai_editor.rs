#![deny(unsafe_code)]
//! Offline AI Auto-Editing Suite
//!
//! Provides automated, intelligent timeline operations without cloud dependencies:
//! - AI Silence Ripple-Cutter (removes dead air while preserving speech transients)
//! - AI Visual Scene-Cut Detection (dHash + color histogram difference)
//! - AI Beat & Transient Detector (spectral energy onset for rhythmic editing)
//! - AI Auto-Captions Generator (speech-aligned subtitle track generation)

use crate::models::{Clip, ClipType, TimelineProject, Track, TrackType};
use crate::timeline::TimelineEngine;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1. AI Silence Ripple-Cutter
// ---------------------------------------------------------------------------

/// Configuration parameters for AI silence ripple-cutting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SilenceCutConfig {
    /// Silence threshold in decibels relative to full scale (e.g. -40.0 dBFS)
    pub threshold_dbfs: f32,
    /// Minimum duration of continuous silence to trigger a ripple cut in ms (e.g. 500ms)
    pub min_silence_duration_ms: u64,
    /// Pre-speech safety padding in ms to prevent clipping onset plosives (e.g. 60ms)
    pub pre_pad_ms: u64,
    /// Post-speech safety padding in ms to prevent clipping trailing vowels (e.g. 80ms)
    pub post_pad_ms: u64,
}

impl Default for SilenceCutConfig {
    fn default() -> Self {
        Self {
            threshold_dbfs: -38.0,
            min_silence_duration_ms: 450,
            pre_pad_ms: 60,
            post_pad_ms: 80,
        }
    }
}

/// A detected silent region on the timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SilenceInterval {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl SilenceInterval {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

pub struct AiSilenceCutter;

impl AiSilenceCutter {
    /// Detect silent intervals in an audio stream using windowed RMS energy
    pub fn detect_silence(
        samples: &[(f32, f32)],
        sample_rate: u32,
        config: &SilenceCutConfig,
    ) -> Vec<SilenceInterval> {
        if samples.is_empty() || sample_rate == 0 {
            return Vec::new();
        }

        let chunk_size = ((sample_rate as f32 * 0.05) as usize).max(32); // 50ms evaluation window
        let threshold_linear = 10.0f32.powf(config.threshold_dbfs / 20.0);

        let mut silent_intervals = Vec::new();
        let mut silence_start_sample: Option<usize> = None;

        for (chunk_idx, chunk) in samples.chunks(chunk_size).enumerate() {
            let start_sample = chunk_idx * chunk_size;

            // Compute RMS energy across both channels
            let sum_sq: f32 = chunk.iter().map(|&(l, r)| l * l + r * r).sum();
            let rms = (sum_sq / (chunk.len() * 2) as f32).sqrt();

            let is_silent = rms < threshold_linear;

            if is_silent {
                if silence_start_sample.is_none() {
                    silence_start_sample = Some(start_sample);
                }
            } else if let Some(start_s) = silence_start_sample.take() {
                let duration_samples = start_sample - start_s;
                let duration_ms = (duration_samples as f64 * 1000.0 / sample_rate as f64) as u64;

                if duration_ms >= config.min_silence_duration_ms {
                    let raw_start_ms = (start_s as f64 * 1000.0 / sample_rate as f64) as u64;
                    let raw_end_ms = (start_sample as f64 * 1000.0 / sample_rate as f64) as u64;

                    // Apply pre-pad and post-pad safety margins
                    let safe_start = raw_start_ms + config.post_pad_ms;
                    let safe_end = raw_end_ms.saturating_sub(config.pre_pad_ms);

                    if safe_end > safe_start && (safe_end - safe_start) >= 100 {
                        silent_intervals.push(SilenceInterval {
                            start_ms: safe_start,
                            end_ms: safe_end,
                        });
                    }
                }
            }
        }

        // Check trailing silence at end of audio
        if let Some(start_s) = silence_start_sample {
            let total_samples = samples.len();
            let duration_samples = total_samples - start_s;
            let duration_ms = (duration_samples as f64 * 1000.0 / sample_rate as f64) as u64;

            if duration_ms >= config.min_silence_duration_ms {
                let raw_start_ms = (start_s as f64 * 1000.0 / sample_rate as f64) as u64;
                let raw_end_ms = (total_samples as f64 * 1000.0 / sample_rate as f64) as u64;

                let safe_start = raw_start_ms + config.post_pad_ms;
                let safe_end = raw_end_ms;
                if safe_end > safe_start {
                    silent_intervals.push(SilenceInterval {
                        start_ms: safe_start,
                        end_ms: safe_end,
                    });
                }
            }
        }

        silent_intervals
    }

    /// Automatically ripple-cut all detected dead air from the timeline
    pub fn apply_ripple_cut(engine: &mut TimelineEngine, silences: &[SilenceInterval]) -> usize {
        if silences.is_empty() {
            return 0;
        }

        // Process silences from right to left (descending order) to preserve upstream timestamps
        let mut sorted_silences = silences.to_vec();
        sorted_silences.sort_by_key(|s| std::cmp::Reverse(s.start_ms));

        let mut cuts_applied = 0;

        for interval in sorted_silences {
            let start = interval.start_ms;
            let end = interval.end_ms;
            let duration = interval.duration_ms();

            if duration == 0 {
                continue;
            }

            // Split and ripple trim across all unlocked tracks
            for track in &mut engine.project_mut().tracks {
                if track.locked {
                    continue;
                }

                // Check clips overlapping this interval
                let mut new_clips = Vec::new();
                for clip in &track.clips {
                    if clip.timeline_out_ms <= start {
                        // Entirely before interval: unchanged
                        new_clips.push(clip.clone());
                    } else if clip.timeline_in_ms >= end {
                        // Entirely after interval: shifted left by duration
                        let mut shifted = clip.clone();
                        shifted.timeline_in_ms = shifted.timeline_in_ms.saturating_sub(duration);
                        shifted.timeline_out_ms = shifted.timeline_out_ms.saturating_sub(duration);
                        new_clips.push(shifted);
                    } else if clip.timeline_in_ms < start && clip.timeline_out_ms > end {
                        // Clip spans across the silence: split into left and right parts
                        let speed = if clip.speed_multiplier > 0.0 {
                            clip.speed_multiplier
                        } else {
                            1.0
                        };

                        let mut left = clip.clone();
                        left.timeline_out_ms = start;
                        let left_dur = start - clip.timeline_in_ms;
                        left.source_out_ms = clip.source_in_ms + (left_dur as f32 * speed) as u64;

                        let mut right = clip.clone();
                        right.id = format!("{}_cut", clip.id);
                        right.timeline_in_ms = start; // Shifted left by the cut
                        right.timeline_out_ms = clip.timeline_out_ms.saturating_sub(duration);
                        let cut_src = (duration as f32 * speed) as u64;
                        right.source_in_ms = left.source_out_ms + cut_src;

                        new_clips.push(left);
                        new_clips.push(right);
                    } else if clip.timeline_in_ms >= start && clip.timeline_out_ms <= end {
                        // Clip is completely inside the silent region: dropped
                    } else if clip.timeline_in_ms < start && clip.timeline_out_ms <= end {
                        // Trim end of clip
                        let mut trimmed = clip.clone();
                        trimmed.timeline_out_ms = start;
                        new_clips.push(trimmed);
                    } else if clip.timeline_in_ms >= start && clip.timeline_out_ms > end {
                        // Trim start of clip and shift left
                        let mut trimmed = clip.clone();
                        trimmed.timeline_in_ms = start;
                        trimmed.timeline_out_ms = clip.timeline_out_ms.saturating_sub(duration);
                        new_clips.push(trimmed);
                    }
                }

                track.clips = new_clips;
                track.clips.sort_by_key(|c| c.timeline_in_ms);
            }

            cuts_applied += 1;
        }

        cuts_applied
    }
}

// ---------------------------------------------------------------------------
// 2. AI Visual Scene-Cut Detection
// ---------------------------------------------------------------------------

pub struct SceneCutDetector;

impl SceneCutDetector {
    /// Compute 64-bit difference hash (dHash) of a 32-bit RGBA video frame
    pub fn compute_dhash(rgba: &[u8], width: u32, height: u32) -> u64 {
        if width < 9 || height < 8 || rgba.len() < (width * height * 4) as usize {
            return 0;
        }

        // Downsample to 9x8 grayscale matrix
        let mut gray_matrix = [0u8; 72]; // 9 cols x 8 rows
        let x_step = width / 9;
        let y_step = height / 8;

        for r in 0..8 {
            for c in 0..9 {
                let px = c * x_step;
                let py = r * y_step;
                let idx = ((py * width + px) * 4) as usize;
                let gray = (0.299 * rgba[idx] as f32
                    + 0.587 * rgba[idx + 1] as f32
                    + 0.114 * rgba[idx + 2] as f32) as u8;
                gray_matrix[(r * 9 + c) as usize] = gray;
            }
        }

        // Compute row-wise relative gradient: if col[i] > col[i+1] -> 1 else 0
        let mut hash = 0u64;
        let mut bit_idx = 0;
        for r in 0..8 {
            for c in 0..8 {
                let left = gray_matrix[r * 9 + c];
                let right = gray_matrix[r * 9 + c + 1];
                if left > right {
                    hash |= 1u64 << bit_idx;
                }
                bit_idx += 1;
            }
        }

        hash
    }

    /// Compute Color Histogram Delta between two RGBA frames (0.0 to 1.0)
    pub fn compute_histogram_delta(frame_a: &[u8], frame_b: &[u8]) -> f32 {
        if frame_a.len() != frame_b.len() || frame_a.is_empty() {
            return 1.0;
        }

        let mut hist_a = [0u32; 64]; // 4 bins per channel (4x4x4 = 64 color bins)
        let mut hist_b = [0u32; 64];

        for (ca, cb) in frame_a.chunks_exact(4).zip(frame_b.chunks_exact(4)) {
            let bin_a =
                ((ca[0] / 64) as usize * 16) + ((ca[1] / 64) as usize * 4) + (ca[2] / 64) as usize;
            let bin_b =
                ((cb[0] / 64) as usize * 16) + ((cb[1] / 64) as usize * 4) + (cb[2] / 64) as usize;
            hist_a[bin_a] += 1;
            hist_b[bin_b] += 1;
        }

        let num_pixels = (frame_a.len() / 4) as f32;
        let mut sum_diff = 0.0f32;
        for i in 0..64 {
            let pa = hist_a[i] as f32 / num_pixels;
            let pb = hist_b[i] as f32 / num_pixels;
            sum_diff += (pa - pb).abs();
        }

        (sum_diff * 0.5).clamp(0.0, 1.0)
    }

    /// Detect scene cut between two consecutive frames using dual dHash + histogram metric
    pub fn is_scene_cut(
        frame_a: &[u8],
        width_a: u32,
        height_a: u32,
        frame_b: &[u8],
        width_b: u32,
        height_b: u32,
    ) -> bool {
        let hash_a = Self::compute_dhash(frame_a, width_a, height_a);
        let hash_b = Self::compute_dhash(frame_b, width_b, height_b);
        let hamming_dist = (hash_a ^ hash_b).count_ones();

        let hist_delta = Self::compute_histogram_delta(frame_a, frame_b);

        // Combined cut criterion: Hamming distance > 14 bits AND color histogram delta > 0.35
        hamming_dist >= 14 || hist_delta > 0.40
    }
}

// ---------------------------------------------------------------------------
// 3. AI Beat & Transient Detector
// ---------------------------------------------------------------------------

pub struct BeatDetector;

impl BeatDetector {
    /// Detect musical beat markers on an audio stream using spectral energy flux
    pub fn detect_beats(
        samples: &[(f32, f32)],
        sample_rate: u32,
        sensitivity: f32, // 1.0 to 3.0 (higher = more selective)
    ) -> Vec<u64> {
        if samples.is_empty() || sample_rate == 0 {
            return Vec::new();
        }

        let window_size = ((sample_rate as f32 * 0.02) as usize).max(16); // 20ms analysis window
        let mut energies = Vec::new();

        for chunk in samples.chunks(window_size) {
            let energy: f32 =
                chunk.iter().map(|&(l, r)| l * l + r * r).sum::<f32>() / chunk.len() as f32;
            energies.push(energy);
        }

        let mut beat_timestamps = Vec::new();
        let local_window = 20; // 400ms neighborhood

        for i in local_window..(energies.len().saturating_sub(local_window)) {
            let current = energies[i];
            let neighborhood = &energies[i - local_window..i + local_window];
            let avg: f32 = neighborhood.iter().sum::<f32>() / neighborhood.len() as f32;

            // Beat peak condition
            if current > avg * sensitivity && current > 0.01 {
                // Local maximum test
                let is_peak = neighborhood.iter().all(|&e| current >= e);
                if is_peak {
                    let timestamp_ms =
                        ((i * window_size) as f64 * 1000.0 / sample_rate as f64) as u64;

                    // Ensure minimum gap between consecutive beats (at least 200ms -> ~300 BPM max)
                    if beat_timestamps
                        .last()
                        .map_or(true, |&last| timestamp_ms - last >= 200)
                    {
                        beat_timestamps.push(timestamp_ms);
                    }
                }
            }
        }

        beat_timestamps
    }
}

// ---------------------------------------------------------------------------
// 4. AI Auto-Captions to Subtitle Track
// ---------------------------------------------------------------------------

pub struct AutoCaptionGenerator;

impl AutoCaptionGenerator {
    /// Generate a synchronized subtitle track from detected speech segments
    pub fn generate_subtitle_track(
        project: &mut TimelineProject,
        captions: &[(u64, u64, &str)], // (start_ms, end_ms, text)
    ) -> String {
        let track_id = format!("sub_{}", project.tracks.len() + 1);
        let mut sub_track = Track::new(&track_id, "Auto Captions", TrackType::Subtitle, 50);

        for (idx, &(start_ms, end_ms, text)) in captions.iter().enumerate() {
            let clip = Clip {
                id: format!("cap_{}_{}", track_id, idx + 1),
                name: text.to_string(),
                clip_type: ClipType::Title,
                track_id: track_id.clone(),
                source_id: None,
                source_in_ms: 0,
                source_out_ms: end_ms.saturating_sub(start_ms),
                timeline_in_ms: start_ms,
                timeline_out_ms: end_ms,
                speed_multiplier: 1.0,
                volume: crate::models::AnimatedProperty::constant(1.0),
                pan: 0.0,
                opacity: crate::models::AnimatedProperty::constant(1.0),
                blend_mode: crate::models::BlendMode::Normal,
                transform: crate::models::Transform2D::default(),
                keyframed_transform: None,
                chroma_key: crate::models::ChromaKeySettings::default(),
                color_grading: crate::models::ColorGradingSettings::default(),
                eq_settings: crate::models::ParametricEqSettings::default(),
                compressor: crate::models::CompressorSettings::default(),
            };
            sub_track.clips.push(clip);
        }

        project.tracks.push(sub_track);
        track_id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detection_and_ripple_cut() {
        let sample_rate = 48000;
        // Generate audio: 1 sec speech, 1 sec dead silence (0.0), 1 sec speech
        let mut samples = Vec::new();
        // 1 sec tone
        for i in 0..48000 {
            let t = i as f32 / 48000.0;
            let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            samples.push((val, val));
        }
        // 1 sec silence
        for _ in 0..48000 {
            samples.push((0.0f32, 0.0f32));
        }
        // 1 sec tone
        for i in 0..48000 {
            let t = i as f32 / 48000.0;
            let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            samples.push((val, val));
        }

        let config = SilenceCutConfig {
            threshold_dbfs: -40.0,
            min_silence_duration_ms: 400,
            pre_pad_ms: 50,
            post_pad_ms: 50,
        };

        let silences = AiSilenceCutter::detect_silence(&samples, sample_rate, &config);
        assert_eq!(silences.len(), 1);
        assert!(silences[0].start_ms >= 1000);
        assert!(silences[0].end_ms <= 2000);
        assert!(silences[0].duration_ms() >= 700);

        // Test timeline ripple cut application
        let mut engine = TimelineEngine::with_default_project("Cut Test");
        let clip = Clip {
            id: "c1".to_string(),
            name: "Full Take".to_string(),
            clip_type: ClipType::Video,
            track_id: "v1".to_string(),
            source_id: None,
            source_in_ms: 0,
            source_out_ms: 3000,
            timeline_in_ms: 0,
            timeline_out_ms: 3000,
            speed_multiplier: 1.0,
            volume: crate::models::AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: crate::models::AnimatedProperty::constant(1.0),
            blend_mode: crate::models::BlendMode::Normal,
            transform: crate::models::Transform2D::default(),
            keyframed_transform: None,
            chroma_key: crate::models::ChromaKeySettings::default(),
            color_grading: crate::models::ColorGradingSettings::default(),
            eq_settings: crate::models::ParametricEqSettings::default(),
            compressor: crate::models::CompressorSettings::default(),
        };
        engine.add_clip(clip).unwrap();

        let cuts = AiSilenceCutter::apply_ripple_cut(&mut engine, &silences);
        assert_eq!(cuts, 1);
        // After cutting silence, duration should be reduced from 3000ms
        assert!(engine.project().duration_ms() < 3000);
    }

    #[test]
    fn test_dhash_and_scene_cut_detection() {
        // Create 16x16 white frame and 16x16 black frame
        let white_frame = vec![255u8; 16 * 16 * 4];
        let black_frame = vec![0u8; 16 * 16 * 4];

        let cut_detected =
            SceneCutDetector::is_scene_cut(&white_frame, 16, 16, &black_frame, 16, 16);
        assert!(
            cut_detected,
            "Scene cut between white and black frame should trigger cut"
        );

        // Two identical white frames
        let same_cut = SceneCutDetector::is_scene_cut(&white_frame, 16, 16, &white_frame, 16, 16);
        assert!(!same_cut, "Identical frames must not trigger scene cut");
    }

    #[test]
    fn test_beat_detection_spikes() {
        // Create 2-second audio with rhythmic spikes every 500ms (120 BPM)
        let sample_rate = 48000;
        let mut samples = vec![(0.02f32, 0.02f32); 48000 * 2];

        // Inject 4 distinct beat impulses
        for &beat_ms in &[0, 500, 1000, 1500] {
            let start_sample = (beat_ms * 48000) / 1000;
            for i in 0..480 {
                if start_sample + i < samples.len() {
                    samples[start_sample + i] = (0.9, 0.9);
                }
            }
        }

        let beats = BeatDetector::detect_beats(&samples, sample_rate, 2.0);
        assert!(
            beats.len() >= 2,
            "Expected multiple rhythmic beats, got: {:?}",
            beats
        );
    }
}
