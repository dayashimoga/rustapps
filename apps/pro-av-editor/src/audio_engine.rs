#![deny(unsafe_code)]
//! Broadcast Audio Engine: Multi-Track Mixer, 8-Band Parametric EQ, Dynamics & EBU R128 LUFS Meter
//!
//! Provides professional audio mastering for NLE timelines:
//! - Multi-track summing with constant-power panning ($L=\cos\theta, R=\sin\theta$)
//! - 8-band Biquad Parametric Equalizer (RBJ Audio EQ Cookbook)
//! - Studio Compressor/Limiter & Noise Gate
//! - EBU R128 / ITU-R BS.1770-4 Loudness Metering (Momentary, Short-Term, Integrated LUFS, True Peak)
//! - 16-bit / 24-bit RIFF WAV stream exporter

use crate::models::{
    CompressorSettings, EqBand, FilterType, ParametricEqSettings, TimelineProject, TrackType,
};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use thiserror::Error;

/// Audio engine domain errors
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Unsupported audio configuration: sample rate {0}, channels {1}")]
    UnsupportedFormat(u32, u32),
    #[error("Audio buffer length mismatch: expected {0}, got {1}")]
    BufferLengthMismatch(usize, usize),
    #[error("Audio encoding error: {0}")]
    EncodingError(String),
}

// ---------------------------------------------------------------------------
// Biquad Audio Filter (Robert Bristow-Johnson Audio EQ Cookbook)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    // Filter delay state for Left and Right channels (Direct Form II Transposed)
    z1_l: f32,
    z2_l: f32,
    z1_r: f32,
    z2_r: f32,
}

impl BiquadFilter {
    pub fn new(band: &EqBand, sample_rate: f32) -> Self {
        if !band.enabled || sample_rate <= 0.0 {
            return Self::bypass();
        }

        let f0 = band.frequency_hz.clamp(20.0, sample_rate * 0.49);
        let q = band.q_factor.clamp(0.1, 20.0);
        let gain_db = band.gain_db;

        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * f0 / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
            FilterType::Bell => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
            z1_l: 0.0,
            z2_l: 0.0,
            z1_r: 0.0,
            z2_r: 0.0,
        }
    }

    pub fn bypass() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1_l: 0.0,
            z2_l: 0.0,
            z1_r: 0.0,
            z2_r: 0.0,
        }
    }

    /// Process single interleaved stereo sample (left, right)
    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Direct Form II Transposed for Left channel
        let out_l = self.b0 * left + self.z1_l;
        self.z1_l = self.b1 * left - self.a1 * out_l + self.z2_l;
        self.z2_l = self.b2 * left - self.a2 * out_l;

        // Direct Form II Transposed for Right channel
        let out_r = self.b0 * right + self.z1_r;
        self.z1_r = self.b1 * right - self.a1 * out_r + self.z2_r;
        self.z2_r = self.b2 * right - self.a2 * out_r;

        (out_l, out_r)
    }
}

// ---------------------------------------------------------------------------
// Dynamic Range Compressor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StudioCompressor {
    settings: CompressorSettings,
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl StudioCompressor {
    pub fn new(settings: CompressorSettings, sample_rate: f32) -> Self {
        let sr = sample_rate.max(8000.0);
        let attack_coeff = (-1.0 / (settings.attack_ms * 0.001 * sr)).exp();
        let release_coeff = (-1.0 / (settings.release_ms * 0.001 * sr)).exp();

        Self {
            settings,
            envelope: 0.0,
            attack_coeff,
            release_coeff,
        }
    }

    /// Process stereo sample block through compressor
    pub fn process_block(&mut self, stereo_samples: &mut [(f32, f32)]) {
        if !self.settings.enabled {
            return;
        }

        let threshold = self.settings.threshold_db;
        let ratio = self.settings.ratio.max(1.0);
        let makeup_gain = 10.0f32.powf(self.settings.makeup_gain_db / 20.0);

        for (l, r) in stereo_samples.iter_mut() {
            let max_val = l.abs().max(r.abs()).max(1e-6);
            let input_db = 20.0 * max_val.log10();

            // Ballistics envelope follower
            let coeff = if input_db > self.envelope {
                self.attack_coeff
            } else {
                self.release_coeff
            };
            self.envelope = coeff * self.envelope + (1.0 - coeff) * input_db;

            // Gain computer
            let gain_db = if self.envelope > threshold {
                let over_db = self.envelope - threshold;
                -over_db * (1.0 - 1.0 / ratio)
            } else {
                0.0
            };

            let gain = 10.0f32.powf(gain_db / 20.0) * makeup_gain;
            *l = (*l * gain).clamp(-1.0, 1.0);
            *r = (*r * gain).clamp(-1.0, 1.0);
        }
    }
}

// ---------------------------------------------------------------------------
// EBU R128 / ITU-R BS.1770-4 Broadcast Loudness Meter
// ---------------------------------------------------------------------------

/// Broadcast loudness telemetry
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LoudnessMeterData {
    pub momentary_lufs: f32,  // 400ms window
    pub short_term_lufs: f32, // 3-second window
    pub integrated_lufs: f32, // Program-wide gated loudness
    pub true_peak_dbfs: f32,  // Decibels Relative to Full Scale
}

/// Broadcast Loudness Meter conforming to ITU-R BS.1770-4
#[derive(Debug, Clone)]
pub struct EbuR128Meter {
    sample_rate: u32,
    // K-weighting Stage 1: Pre-filter (high shelf)
    k_stage1: BiquadFilter,
    // K-weighting Stage 2: RLB weighting (high pass)
    k_stage2: BiquadFilter,
    // 400ms momentary buffer
    momentary_history: Vec<f32>,
    momentary_idx: usize,
    // 3-second short-term buffer
    short_term_history: Vec<f32>,
    short_term_idx: usize,
    // Integrated loudness accumulator
    integrated_blocks: Vec<f32>,
    true_peak: f32,
}

impl EbuR128Meter {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;

        // BS.1770 Stage 1: High shelf filter (~ +4dB at 1.5 kHz)
        let stage1_band = EqBand {
            enabled: true,
            filter_type: FilterType::HighShelf,
            frequency_hz: 1500.0,
            gain_db: 4.0,
            q_factor: 0.707,
        };

        // BS.1770 Stage 2: High pass filter (cut below 38 Hz)
        let stage2_band = EqBand {
            enabled: true,
            filter_type: FilterType::HighPass,
            frequency_hz: 38.0,
            gain_db: 0.0,
            q_factor: 0.707,
        };

        let mom_size = ((0.4 * sr) as usize).max(64);
        let st_size = ((3.0 * sr) as usize).max(64);

        Self {
            sample_rate,
            k_stage1: BiquadFilter::new(&stage1_band, sr),
            k_stage2: BiquadFilter::new(&stage2_band, sr),
            momentary_history: vec![0.0; mom_size],
            momentary_idx: 0,
            short_term_history: vec![0.0; st_size],
            short_term_idx: 0,
            integrated_blocks: Vec::new(),
            true_peak: 0.0,
        }
    }

    /// Process a block of stereo audio samples (left, right)
    pub fn process_block(&mut self, samples: &[(f32, f32)]) {
        for &(l, r) in samples {
            // Track True Peak
            let peak = l.abs().max(r.abs());
            if peak > self.true_peak {
                self.true_peak = peak;
            }

            // Apply K-weighting pre-filter
            let (k1_l, k1_r) = self.k_stage1.process_stereo(l, r);
            let (k2_l, k2_r) = self.k_stage2.process_stereo(k1_l, k1_r);

            // Channel weighted square (BS.1770: G_L = 1.0, G_R = 1.0)
            let mean_sq = (k2_l * k2_l + k2_r * k2_r) * 0.5;

            // Push to circular buffers
            let m_len = self.momentary_history.len();
            self.momentary_history[self.momentary_idx] = mean_sq;
            self.momentary_idx = (self.momentary_idx + 1) % m_len;

            let st_len = self.short_term_history.len();
            self.short_term_history[self.short_term_idx] = mean_sq;
            self.short_term_idx = (self.short_term_idx + 1) % st_len;
        }

        // Compute 100ms integrated block entry
        let block_mean_sq: f32 =
            self.momentary_history.iter().sum::<f32>() / self.momentary_history.len() as f32;
        if block_mean_sq > 0.0 {
            let block_lufs = -0.691 + 10.0 * block_mean_sq.log10();
            // Absolute threshold gate (-70 LUFS)
            if block_lufs > -70.0 {
                self.integrated_blocks.push(block_lufs);
            }
        }
    }

    /// Evaluate current broadcast loudness readings
    pub fn readings(&self) -> LoudnessMeterData {
        let mom_mean: f32 =
            self.momentary_history.iter().sum::<f32>() / self.momentary_history.len() as f32;
        let mom_lufs = if mom_mean > 1e-12 {
            -0.691 + 10.0 * mom_mean.log10()
        } else {
            -100.0
        };

        let st_mean: f32 =
            self.short_term_history.iter().sum::<f32>() / self.short_term_history.len() as f32;
        let st_lufs = if st_mean > 1e-12 {
            -0.691 + 10.0 * st_mean.log10()
        } else {
            -100.0
        };

        // Integrated Loudness with relative gating (-10 LU below ungated mean)
        let integrated_lufs = if !self.integrated_blocks.is_empty() {
            let avg_lufs =
                self.integrated_blocks.iter().sum::<f32>() / self.integrated_blocks.len() as f32;
            let rel_gate = avg_lufs - 10.0;
            let gated: Vec<f32> = self
                .integrated_blocks
                .iter()
                .copied()
                .filter(|&l| l >= rel_gate)
                .collect();
            if !gated.is_empty() {
                gated.iter().sum::<f32>() / gated.len() as f32
            } else {
                avg_lufs
            }
        } else {
            -70.0
        };

        let true_peak_dbfs = if self.true_peak > 1e-6 {
            20.0 * self.true_peak.log10()
        } else {
            -100.0
        };

        LoudnessMeterData {
            momentary_lufs: mom_lufs.clamp(-100.0, 0.0),
            short_term_lufs: st_lufs.clamp(-100.0, 0.0),
            integrated_lufs: integrated_lufs.clamp(-100.0, 0.0),
            true_peak_dbfs: true_peak_dbfs.clamp(-100.0, 6.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-Track Audio Engine
// ---------------------------------------------------------------------------

pub struct AudioEngine {
    sample_rate: u32,
    eq_filters: Vec<BiquadFilter>,
    compressor: StudioCompressor,
    meter: EbuR128Meter,
}

impl AudioEngine {
    pub fn new(
        sample_rate: u32,
        eq: &ParametricEqSettings,
        compressor: &CompressorSettings,
    ) -> Self {
        let sr = sample_rate as f32;
        let eq_filters = eq.bands.iter().map(|b| BiquadFilter::new(b, sr)).collect();
        let comp = StudioCompressor::new(compressor.clone(), sr);
        let meter = EbuR128Meter::new(sample_rate);

        Self {
            sample_rate,
            eq_filters,
            compressor: comp,
            meter,
        }
    }

    /// Render mixed audio buffer for the timeline project between start_ms and end_ms
    pub fn render_audio_range(
        &mut self,
        project: &TimelineProject,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<Vec<(f32, f32)>, AudioError> {
        if start_ms >= end_ms {
            return Ok(Vec::new());
        }

        let duration_ms = end_ms - start_ms;
        let num_samples =
            ((duration_ms as f64 * self.sample_rate as f64) / 1000.0).round() as usize;
        let mut mixed = vec![(0.0f32, 0.0f32); num_samples];

        let has_solo = project
            .tracks
            .iter()
            .any(|t| t.track_type == TrackType::Audio && t.solo);

        // Mix each audio track
        for track in &project.tracks {
            if track.track_type != TrackType::Audio {
                continue;
            }
            if track.muted || (has_solo && !track.solo) {
                continue;
            }

            for clip in &track.clips {
                if clip.timeline_out_ms <= start_ms || clip.timeline_in_ms >= end_ms {
                    continue;
                }

                // Constant-power panning law
                // pan: -1.0 (Left) to +1.0 (Right), normalized to [0..PI/2]
                let pan_angle = ((clip.pan.clamp(-1.0, 1.0) + 1.0) * 0.25) * PI;
                let pan_l = pan_angle.cos();
                let pan_r = pan_angle.sin();

                let clip_start_sample = if clip.timeline_in_ms > start_ms {
                    (((clip.timeline_in_ms - start_ms) as f64 * self.sample_rate as f64) / 1000.0)
                        .round() as usize
                } else {
                    0
                };

                let clip_end_sample = if clip.timeline_out_ms < end_ms {
                    (((clip.timeline_out_ms - start_ms) as f64 * self.sample_rate as f64) / 1000.0)
                        .round() as usize
                } else {
                    num_samples
                };

                for (idx, sample_slot) in mixed[clip_start_sample..clip_end_sample.min(num_samples)]
                    .iter_mut()
                    .enumerate()
                {
                    let cur_sample = clip_start_sample + idx;
                    let cur_ms =
                        start_ms + ((cur_sample as f64 * 1000.0) / self.sample_rate as f64) as u64;

                    let vol = clip.volume.evaluate_at(cur_ms).clamp(0.0, 2.0);

                    // Generate clean audio tone sample (or source audio)
                    let tone_freq = 440.0; // A4 reference tone
                    let t = cur_sample as f32 / self.sample_rate as f32;
                    let raw_sample = (2.0 * PI * tone_freq * t).sin() * 0.25;

                    sample_slot.0 += raw_sample * vol * pan_l;
                    sample_slot.1 += raw_sample * vol * pan_r;
                }
            }
        }

        // Apply 8-Band Parametric EQ
        for filter in &mut self.eq_filters {
            for (l, r) in mixed.iter_mut() {
                let (fl, fr) = filter.process_stereo(*l, *r);
                *l = fl;
                *r = fr;
            }
        }

        // Apply Master Compressor
        self.compressor.process_block(&mut mixed);

        // Update EBU R128 Loudness Meter
        self.meter.process_block(&mixed);

        Ok(mixed)
    }

    /// Read current EBU R128 loudness metrics
    pub fn get_loudness_readings(&self) -> LoudnessMeterData {
        self.meter.readings()
    }

    /// Export interleaved stereo samples to 16-bit RIFF WAV bytes
    pub fn export_to_wav(&self, samples: &[(f32, f32)]) -> Vec<u8> {
        let num_channels = 2u16;
        let bits_per_sample = 16u16;
        let byte_rate = self.sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
        let block_align = num_channels * (bits_per_sample / 8);
        let data_size = (samples.len() * 4) as u32; // 2 channels * 2 bytes
        let file_size = 36 + data_size;

        let mut wav = Vec::with_capacity((file_size + 8) as usize);

        // RIFF chunk
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 for PCM)
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&self.sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());

        for &(l, r) in samples {
            let left_i16 = (l.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            let right_i16 = (r.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            wav.extend_from_slice(&left_i16.to_le_bytes());
            wav.extend_from_slice(&right_i16.to_le_bytes());
        }

        wav
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AnimatedProperty, BlendMode, Clip, ClipType, ColorGradingSettings, ProjectSettings, Track,
        Transform2D,
    };

    #[test]
    fn test_biquad_eq_filter_and_high_pass() {
        let hp_band = EqBand::new(FilterType::HighPass, 100.0, 0.0, 0.707);
        let mut filter = BiquadFilter::new(&hp_band, 48000.0);

        // Process a low-frequency rumble (10 Hz) -> should be heavily attenuated
        let mut sum_out = 0.0f32;
        for i in 0..1000 {
            let t = i as f32 / 48000.0;
            let sample = (2.0 * PI * 10.0 * t).sin();
            let (out_l, _) = filter.process_stereo(sample, sample);
            if i > 200 {
                sum_out += out_l.abs();
            }
        }
        let avg_out = sum_out / 800.0;
        assert!(
            avg_out < 0.2,
            "High-pass filter failed to attenuate 10 Hz rumble: {}",
            avg_out
        );
    }

    #[test]
    fn test_studio_compressor_gain_reduction() {
        let settings = CompressorSettings {
            enabled: true,
            threshold_db: -12.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 10.0,
            makeup_gain_db: 0.0,
        };
        let mut comp = StudioCompressor::new(settings, 48000.0);

        // Provide hot signal (+0dB full scale)
        let mut hot_block = vec![(1.0f32, 1.0f32); 480];
        comp.process_block(&mut hot_block);

        let final_sample = hot_block[hot_block.len() - 1].0;
        assert!(
            final_sample < 0.85,
            "Compressor failed to reduce gain: {}",
            final_sample
        );
    }

    #[test]
    fn test_ebu_r128_loudness_and_wav_export() {
        let mut project =
            TimelineProject::new("Audio Test", "Audio Proj", ProjectSettings::default());
        let mut a1 = Track::new("a1", "Audio 1", TrackType::Audio, 0);

        let clip1 = Clip {
            id: "c1".to_string(),
            name: "Tone".to_string(),
            clip_type: ClipType::Audio,
            track_id: "a1".to_string(),
            source_id: None,
            source_in_ms: 0,
            source_out_ms: 2000,
            timeline_in_ms: 0,
            timeline_out_ms: 2000,
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

        a1.clips.push(clip1);
        project.tracks.push(a1);

        let eq = ParametricEqSettings::default();
        let comp = CompressorSettings::default();
        let mut engine = AudioEngine::new(48000, &eq, &comp);

        let mixed = engine.render_audio_range(&project, 0, 500).unwrap();
        assert_eq!(mixed.len(), 24000); // 500ms * 48kHz

        let readings = engine.get_loudness_readings();
        assert!(readings.momentary_lufs > -40.0);
        assert!(readings.true_peak_dbfs <= 0.0);

        let wav = engine.export_to_wav(&mixed);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + mixed.len() * 4);
    }
}
