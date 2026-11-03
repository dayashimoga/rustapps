#![forbid(unsafe_code)]
//! Voice Activity Detection (VAD) and 80-bin Log-Mel Spectrogram DSP
//!
//! Consumes 16kHz single-channel PCM audio streams and computes
//! energy/ZCR voice activity segmentation and log-Mel spectrogram features for ASR.

use std::f32::consts::PI;

/// Speech activity segment
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechSegment {
    pub start_sample: usize,
    pub end_sample: usize,
    pub start_ms: u64,
    pub end_ms: u64,
    pub is_speech: bool,
}

/// 16kHz Voice Activity Detection Engine
#[derive(Debug, Clone)]
pub struct VadEngine {
    sample_rate: u32,
    frame_size_samples: usize,
    energy_threshold: f32,
    zcr_threshold: f32,
}

impl Default for VadEngine {
    fn default() -> Self {
        Self::new(16000, 480, 0.005, 0.02) // 30ms frames @ 16kHz
    }
}

impl VadEngine {
    pub fn new(
        sample_rate: u32,
        frame_size_samples: usize,
        energy_threshold: f32,
        zcr_threshold: f32,
    ) -> Self {
        Self {
            sample_rate,
            frame_size_samples,
            energy_threshold,
            zcr_threshold,
        }
    }

    /// Classify a single frame of PCM audio [-1.0..1.0] as speech or silence
    pub fn is_speech_frame(&self, frame: &[f32]) -> bool {
        if frame.is_empty() {
            return false;
        }

        // 1. RMS Energy
        let sum_sq: f32 = frame.iter().map(|&x| x * x).sum();
        let rms = (sum_sq / frame.len() as f32).sqrt();

        // 2. Zero Crossing Rate (ZCR)
        let mut zcr_count = 0usize;
        for i in 1..frame.len() {
            if (frame[i] >= 0.0 && frame[i - 1] < 0.0) || (frame[i] < 0.0 && frame[i - 1] >= 0.0) {
                zcr_count += 1;
            }
        }
        let zcr = zcr_count as f32 / frame.len() as f32;

        rms > self.energy_threshold && zcr > self.zcr_threshold
    }

    /// Segment a complete audio buffer into contiguous speech/non-speech chunks
    pub fn segment_audio(&self, pcm: &[f32]) -> Vec<SpeechSegment> {
        let mut segments = Vec::new();
        let mut offset = 0;

        let mut current_is_speech = false;
        let mut segment_start = 0;

        while offset + self.frame_size_samples <= pcm.len() {
            let frame = &pcm[offset..offset + self.frame_size_samples];
            let speech = self.is_speech_frame(frame);

            if offset == 0 {
                current_is_speech = speech;
                segment_start = 0;
            } else if speech != current_is_speech {
                // End current segment
                segments.push(SpeechSegment {
                    start_sample: segment_start,
                    end_sample: offset,
                    start_ms: (segment_start as u64 * 1000) / self.sample_rate as u64,
                    end_ms: (offset as u64 * 1000) / self.sample_rate as u64,
                    is_speech: current_is_speech,
                });
                segment_start = offset;
                current_is_speech = speech;
            }

            offset += self.frame_size_samples;
        }

        if segment_start < pcm.len() {
            segments.push(SpeechSegment {
                start_sample: segment_start,
                end_sample: pcm.len(),
                start_ms: (segment_start as u64 * 1000) / self.sample_rate as u64,
                end_ms: (pcm.len() as u64 * 1000) / self.sample_rate as u64,
                is_speech: current_is_speech,
            });
        }

        segments
    }
}

/// 80-bin Log-Mel Spectrogram Filterbank Extractor
#[derive(Debug, Clone)]
pub struct MelSpectrogramExtractor {
    pub sample_rate: usize,
    pub n_fft: usize,
    pub frame_length: usize,
    pub hop_length: usize,
    pub n_mels: usize,
    window: Vec<f32>,
    mel_filters: Vec<Vec<f32>>, // [n_mels, (n_fft / 2) + 1]
}

impl Default for MelSpectrogramExtractor {
    fn default() -> Self {
        Self::new(16000, 512, 400, 160, 80) // 25ms window, 10ms hop, 80 mel bins
    }
}

impl MelSpectrogramExtractor {
    #[allow(clippy::needless_range_loop)]
    pub fn new(
        sample_rate: usize,
        n_fft: usize,
        frame_length: usize,
        hop_length: usize,
        n_mels: usize,
    ) -> Self {
        // Precompute Hamming window
        let mut window = Vec::with_capacity(frame_length);
        for i in 0..frame_length {
            let val = 0.54 - 0.46 * (2.0 * PI * i as f32 / (frame_length - 1) as f32).cos();
            window.push(val);
        }

        // Precompute triangular Mel filterbank matrix
        let num_bins = (n_fft / 2) + 1;
        let mut mel_filters = vec![vec![0.0f32; num_bins]; n_mels];

        let min_mel = Self::hz_to_mel(0.0);
        let max_mel = Self::hz_to_mel(sample_rate as f32 / 2.0);

        let mut mel_points = Vec::with_capacity(n_mels + 2);
        for i in 0..=(n_mels + 1) {
            let mel = min_mel + (i as f32 / (n_mels + 1) as f32) * (max_mel - min_mel);
            mel_points.push(Self::mel_to_hz(mel));
        }

        let mut bin_points = Vec::with_capacity(n_mels + 2);
        for &hz in &mel_points {
            let bin = ((n_fft + 1) as f32 * hz / sample_rate as f32).floor() as usize;
            bin_points.push(bin.min(num_bins - 1));
        }

        for m in 0..n_mels {
            let left = bin_points[m];
            let center = bin_points[m + 1];
            let right = bin_points[m + 2];

            for k in left..center {
                if center > left {
                    mel_filters[m][k] = (k - left) as f32 / (center - left) as f32;
                }
            }
            for k in center..right {
                if right > center {
                    mel_filters[m][k] = (right - k) as f32 / (right - center) as f32;
                }
            }
        }

        Self {
            sample_rate,
            n_fft,
            frame_length,
            hop_length,
            n_mels,
            window,
            mel_filters,
        }
    }

    fn hz_to_mel(hz: f32) -> f32 {
        2595.0 * (1.0 + hz / 700.0).log10()
    }

    fn mel_to_hz(mel: f32) -> f32 {
        700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
    }

    /// Extract 2D log-Mel spectrogram matrix of shape `[num_frames, 80]`
    #[allow(clippy::needless_range_loop)]
    pub fn extract(&self, pcm: &[f32]) -> Vec<Vec<f32>> {
        if pcm.len() < self.frame_length {
            return Vec::new();
        }

        let num_frames = (pcm.len() - self.frame_length) / self.hop_length + 1;
        let num_bins = (self.n_fft / 2) + 1;
        let mut spectrogram = Vec::with_capacity(num_frames);

        let mut windowed = vec![0.0f32; self.n_fft];
        let mut power_spectrum = vec![0.0f32; num_bins];

        for f in 0..num_frames {
            let start = f * self.hop_length;
            let slice = &pcm[start..start + self.frame_length];

            // Apply Hamming window and zero-pad to n_fft
            for i in 0..self.frame_length {
                windowed[i] = slice[i] * self.window[i];
            }
            for i in self.frame_length..self.n_fft {
                windowed[i] = 0.0;
            }

            // Real discrete Fourier transform for power spectrum (bins 0..=n_fft/2)
            for k in 0..num_bins {
                let mut real = 0.0f32;
                let mut imag = 0.0f32;
                let angle_step = 2.0 * PI * k as f32 / self.n_fft as f32;

                for (n, &sample) in windowed.iter().enumerate() {
                    let angle = angle_step * n as f32;
                    real += sample * angle.cos();
                    imag -= sample * angle.sin();
                }

                power_spectrum[k] = real * real + imag * imag;
            }

            // Apply triangular Mel filterbank and take log
            let mut mel_energies = Vec::with_capacity(self.n_mels);
            for m in 0..self.n_mels {
                let mut energy = 0.0f32;
                for k in 0..num_bins {
                    energy += power_spectrum[k] * self.mel_filters[m][k];
                }
                mel_energies.push((energy + 1e-5).ln());
            }

            spectrogram.push(mel_energies);
        }

        spectrogram
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_speech_detection() {
        let vad = VadEngine::default();

        // Silence buffer
        let silence = vec![0.0f32; 480];
        assert!(!vad.is_speech_frame(&silence));

        // 440 Hz synthetic voice tone
        let mut tone = Vec::with_capacity(480);
        for i in 0..480 {
            tone.push(0.5 * (2.0 * PI * 440.0 * i as f32 / 16000.0).sin());
        }
        assert!(vad.is_speech_frame(&tone));
    }

    #[test]
    fn test_mel_spectrogram_dimensions() {
        let extractor = MelSpectrogramExtractor::default();
        // 1600 samples = 100ms @ 16kHz
        let pcm = vec![0.1f32; 1600];

        let mel = extractor.extract(&pcm);
        assert!(!mel.is_empty());
        assert_eq!(mel[0].len(), 80); // 80 Mel bins
    }
}
