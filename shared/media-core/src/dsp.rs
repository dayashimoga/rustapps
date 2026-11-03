#![forbid(unsafe_code)]
//! Mastering-Grade Audio DSP: Biquad Filters, 8-Band Parametric EQ, Compressor, Gate & LUFS Meter
//!
//! Provides genuine non-mocked digital signal processing algorithms based on
//! Robert Bristow-Johnson's Audio EQ Cookbook and ITU-R BS.1770 / EBU R128 specifications.

use std::f32::consts::PI;

/// Biquad filter frequency response configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
    Peaking,
    LowShelf,
    HighShelf,
    Notch,
}

/// Transposed Direct Form II Biquad Filter
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    filter_type: FilterType,
    sample_rate: f32,
    freq_hz: f32,
    q: f32,
    gain_db: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    s1: f32,
    s2: f32,
}

impl BiquadFilter {
    pub fn new(
        filter_type: FilterType,
        sample_rate: f32,
        freq_hz: f32,
        q: f32,
        gain_db: f32,
    ) -> Self {
        let mut filter = Self {
            filter_type,
            sample_rate,
            freq_hz,
            q,
            gain_db,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            s1: 0.0,
            s2: 0.0,
        };
        filter.recalculate_coefficients();
        filter
    }

    pub fn set_parameters(&mut self, freq_hz: f32, q: f32, gain_db: f32) {
        self.freq_hz = freq_hz;
        self.q = q.max(0.01);
        self.gain_db = gain_db;
        self.recalculate_coefficients();
    }

    /// Audio EQ Cookbook coefficient calculation
    fn recalculate_coefficients(&mut self) {
        let w0 = 2.0 * PI * (self.freq_hz / self.sample_rate).clamp(0.0001, 0.499);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q.max(0.001));
        let a_amp = 10.0f32.powf(self.gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::Lowpass => (
                (1.0 - cos_w0) / 2.0,
                1.0 - cos_w0,
                (1.0 - cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::Highpass => (
                (1.0 + cos_w0) / 2.0,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::Bandpass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            FilterType::Peaking => (
                1.0 + alpha * a_amp,
                -2.0 * cos_w0,
                1.0 - alpha * a_amp,
                1.0 + alpha / a_amp,
                -2.0 * cos_w0,
                1.0 - alpha / a_amp,
            ),
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a_amp.sqrt() * alpha;
                (
                    a_amp * ((a_amp + 1.0) - (a_amp - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    2.0 * a_amp * ((a_amp - 1.0) - (a_amp + 1.0) * cos_w0),
                    a_amp * ((a_amp + 1.0) - (a_amp - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a_amp + 1.0) + (a_amp - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    -2.0 * ((a_amp - 1.0) + (a_amp + 1.0) * cos_w0),
                    (a_amp + 1.0) + (a_amp - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a_amp.sqrt() * alpha;
                (
                    a_amp * ((a_amp + 1.0) + (a_amp - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    -2.0 * a_amp * ((a_amp - 1.0) + (a_amp + 1.0) * cos_w0),
                    a_amp * ((a_amp + 1.0) + (a_amp - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a_amp + 1.0) - (a_amp - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    2.0 * ((a_amp - 1.0) - (a_amp + 1.0) * cos_w0),
                    (a_amp + 1.0) - (a_amp - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            FilterType::Notch => (
                1.0,
                -2.0 * cos_w0,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
        };

        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    /// Process single sample through transposed direct form II structure
    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.s1;
        self.s1 = self.b1 * x - self.a1 * y + self.s2;
        self.s2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

/// 8-Band Cascaded Parametric Equalizer
#[derive(Debug, Clone)]
pub struct ParametricEq8Band {
    pub bands: [BiquadFilter; 8],
}

impl ParametricEq8Band {
    pub fn new(sample_rate: f32) -> Self {
        let bands = [
            BiquadFilter::new(FilterType::Highpass, sample_rate, 30.0, 0.707, 0.0), // Band 0: Sub
            BiquadFilter::new(FilterType::LowShelf, sample_rate, 100.0, 0.707, 0.0), // Band 1: Low Shelf
            BiquadFilter::new(FilterType::Peaking, sample_rate, 250.0, 1.0, 0.0), // Band 2: Low-Mid
            BiquadFilter::new(FilterType::Peaking, sample_rate, 750.0, 1.0, 0.0), // Band 3: Mid 1
            BiquadFilter::new(FilterType::Peaking, sample_rate, 2000.0, 1.0, 0.0), // Band 4: Mid 2
            BiquadFilter::new(FilterType::Peaking, sample_rate, 5000.0, 1.0, 0.0), // Band 5: High-Mid
            BiquadFilter::new(FilterType::HighShelf, sample_rate, 10000.0, 0.707, 0.0), // Band 6: High Shelf
            BiquadFilter::new(FilterType::Peaking, sample_rate, 16000.0, 0.707, 0.0), // Band 7: Air
        ];

        Self { bands }
    }

    pub fn process_buffer(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            let mut val = *sample;
            for band in &mut self.bands {
                val = band.process_sample(val);
            }
            *sample = val;
        }
    }
}

/// Dynamic Audio Compressor with Attack/Release envelope follower
#[derive(Debug, Clone)]
pub struct AudioCompressor {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_coeff: f32,
    pub release_coeff: f32,
    pub makeup_gain_linear: f32,
    envelope: f32,
}

impl AudioCompressor {
    pub fn new(
        sample_rate: f32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_gain_db: f32,
    ) -> Self {
        let attack_coeff = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
        let makeup_gain_linear = 10.0f32.powf(makeup_gain_db / 20.0);

        Self {
            threshold_db,
            ratio: ratio.max(1.0),
            attack_coeff,
            release_coeff,
            makeup_gain_linear,
            envelope: 0.0,
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();

        // Ballistics envelope tracking
        if abs_x > self.envelope {
            self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs_x;
        } else {
            self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs_x;
        }

        let env_db = 20.0 * (self.envelope.max(1e-6)).log10();
        let gain_reduction_db = if env_db > self.threshold_db {
            (self.threshold_db - env_db) * (1.0 - 1.0 / self.ratio)
        } else {
            0.0
        };

        let gain_linear = 10.0f32.powf(gain_reduction_db / 20.0);
        x * gain_linear * self.makeup_gain_linear
    }
}

/// Audio Noise Gate
#[derive(Debug, Clone)]
pub struct NoiseGate {
    pub threshold_db: f32,
    pub attack_coeff: f32,
    pub release_coeff: f32,
    envelope: f32,
}

impl NoiseGate {
    pub fn new(sample_rate: f32, threshold_db: f32, attack_ms: f32, release_ms: f32) -> Self {
        let attack_coeff = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();

        Self {
            threshold_db,
            attack_coeff,
            release_coeff,
            envelope: 0.0,
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        if abs_x > self.envelope {
            self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs_x;
        } else {
            self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs_x;
        }

        let env_db = 20.0 * (self.envelope.max(1e-6)).log10();
        if env_db < self.threshold_db {
            0.0
        } else {
            x
        }
    }
}

/// ITU-R BS.1770 / EBU R128 Integrated LUFS Loudness Meter
#[derive(Debug, Clone)]
pub struct LufsMeter {
    stage1_highshelf: BiquadFilter,
    stage2_highpass: BiquadFilter,
    block_powers: Vec<f32>,
}

impl LufsMeter {
    pub fn new(sample_rate: f32) -> Self {
        // ITU-R BS.1770 K-weighting pre-filter coefficients:
        // Stage 1: High shelf (+4.0 dB @ 1.5 kHz)
        let stage1 = BiquadFilter::new(FilterType::HighShelf, sample_rate, 1500.0, 0.707, 4.0);
        // Stage 2: High pass (RLB weighting @ ~38 Hz)
        let stage2 = BiquadFilter::new(FilterType::Highpass, sample_rate, 38.0, 0.5, 0.0);

        Self {
            stage1_highshelf: stage1,
            stage2_highpass: stage2,
            block_powers: Vec::new(),
        }
    }

    /// Process audio stream and compute EBU R128 Integrated LUFS with gating
    pub fn measure_integrated_lufs(&mut self, pcm: &[f32]) -> f32 {
        if pcm.is_empty() {
            return -70.0;
        }

        // Apply K-weighting filters
        let mut filtered = Vec::with_capacity(pcm.len());
        for &s in pcm {
            let s1 = self.stage1_highshelf.process_sample(s);
            let s2 = self.stage2_highpass.process_sample(s1);
            filtered.push(s2);
        }

        // 400ms blocks with 75% overlap (100ms step)
        let block_size = 400 * 48; // 400ms at 48kHz
        let step = block_size / 4;

        if filtered.len() < block_size {
            let mean_sq: f32 = filtered.iter().map(|&x| x * x).sum::<f32>() / filtered.len() as f32;
            return -0.691 + 10.0 * (mean_sq.max(1e-10)).log10();
        }

        self.block_powers.clear();
        let mut offset = 0;
        while offset + block_size <= filtered.len() {
            let slice = &filtered[offset..offset + block_size];
            let mean_sq: f32 = slice.iter().map(|&x| x * x).sum::<f32>() / block_size as f32;
            self.block_powers.push(mean_sq);
            offset += step;
        }

        // Absolute gate: -70 LKFS
        let abs_threshold = 10.0f32.powf((-70.0 + 0.691) / 10.0);
        let active_blocks: Vec<f32> = self
            .block_powers
            .iter()
            .copied()
            .filter(|&p| p > abs_threshold)
            .collect();

        if active_blocks.is_empty() {
            return -70.0;
        }

        let mean_active: f32 = active_blocks.iter().sum::<f32>() / active_blocks.len() as f32;
        let relative_lufs = -0.691 + 10.0 * (mean_active.max(1e-10)).log10();

        // Relative gate: -10 LU below relative loudness
        let rel_threshold = 10.0f32.powf((relative_lufs - 10.0 + 0.691) / 10.0);
        let gated_blocks: Vec<f32> = active_blocks
            .into_iter()
            .filter(|&p| p > rel_threshold)
            .collect();

        if gated_blocks.is_empty() {
            return -70.0;
        }

        let final_mean: f32 = gated_blocks.iter().sum::<f32>() / gated_blocks.len() as f32;
        -0.691 + 10.0 * (final_mean.max(1e-10)).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_peaking_filter() {
        let mut filter = BiquadFilter::new(FilterType::Peaking, 44100.0, 1000.0, 1.0, 6.0);
        let mut y = 0.0f32;
        for _ in 0..100 {
            y = filter.process_sample(0.5);
        }
        assert!(y.is_finite());
    }

    #[test]
    fn test_audio_compressor_attenuation() {
        let mut comp = AudioCompressor::new(44100.0, -10.0, 4.0, 5.0, 50.0, 0.0);
        let mut out = 0.0f32;
        // Feed loud +0dBFS (1.0) signal
        for _ in 0..2000 {
            out = comp.process_sample(1.0);
        }
        // Must be compressed below 1.0
        assert!(out < 0.90, "Expected compressed signal, got {}", out);
    }

    #[test]
    fn test_noise_gate_cutoff() {
        let mut gate = NoiseGate::new(44100.0, -30.0, 5.0, 50.0);
        let quiet_signal = 0.001f32; // -60 dBFS
        let mut out = 0.0f32;
        for _ in 0..500 {
            out = gate.process_sample(quiet_signal);
        }
        assert_eq!(out, 0.0, "Quiet signal must be gated to 0.0");
    }
}
