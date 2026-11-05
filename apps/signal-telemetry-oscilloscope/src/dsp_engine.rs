#![forbid(unsafe_code)]
//! High-Performance Offline Digital Signal Processing (DSP) & Protocol Decoder Engine

use crate::models::{
    DecodedPacket, FftSpectrum, ProtocolType, SignalStatistics, TriggerConfig, TriggerEdge,
    WaveformType,
};
use std::f32::consts::PI;

pub struct SignalDspEngine;

impl SignalDspEngine {
    /// Computes electrical voltage and frequency statistics for a waveform sample buffer
    pub fn compute_statistics(samples: &[f32], sample_rate_hz: f32) -> SignalStatistics {
        if samples.is_empty() {
            return SignalStatistics::default();
        }

        let mut vmin = f32::MAX;
        let mut vmax = f32::MIN;
        let mut sum = 0.0f32;
        let mut sum_sq = 0.0f32;

        for &s in samples {
            if s < vmin {
                vmin = s;
            }
            if s > vmax {
                vmax = s;
            }
            sum += s;
            sum_sq += s * s;
        }

        let n = samples.len() as f32;
        let vmean = sum / n;
        let vpp = vmax - vmin;
        let vrms = (sum_sq / n).sqrt();

        // Frequency detection via zero-crossing (threshold = vmean)
        let mut crossings = Vec::new();
        let mut high_count = 0usize;

        for i in 1..samples.len() {
            if samples[i] > vmean {
                high_count += 1;
            }
            if samples[i - 1] <= vmean && samples[i] > vmean {
                // Linear interpolation of precise crossing position
                let dy = samples[i] - samples[i - 1];
                let frac = if dy.abs() > 1e-6 {
                    (vmean - samples[i - 1]) / dy
                } else {
                    0.5
                };
                crossings.push((i - 1) as f32 + frac);
            }
        }

        let duty_cycle_percent = (high_count as f32 / n) * 100.0;

        let (frequency_hz, period_ms) = if crossings.len() >= 2 {
            let total_cycles = (crossings.len() - 1) as f32;
            let total_samples = crossings.last().unwrap() - crossings.first().unwrap();
            let avg_period_samples = total_samples / total_cycles;
            let period_s = avg_period_samples / sample_rate_hz;
            let freq = if period_s > 1e-7 { 1.0 / period_s } else { 0.0 };
            (freq, period_s * 1000.0)
        } else {
            (0.0, 0.0)
        };

        SignalStatistics {
            vpp,
            vrms,
            vmin,
            vmax,
            vmean,
            frequency_hz,
            period_ms,
            duty_cycle_percent,
        }
    }

    /// Computes Fast Fourier Transform (FFT) power spectral density with Hann windowing
    pub fn compute_fft_spectrum(samples: &[f32], sample_rate_hz: f32) -> FftSpectrum {
        if samples.len() < 8 {
            return FftSpectrum::default();
        }

        // Use largest power-of-two window up to 1024 points for fast 60 FPS rendering
        let n = samples.len().min(1024);
        let fft_size = if n.is_power_of_two() {
            n
        } else {
            1 << (usize::BITS - 1 - n.leading_zeros())
        };
        if fft_size < 4 {
            return FftSpectrum::default();
        }

        // Apply Hann window: w[n] = 0.5 - 0.5 * cos(2*pi*n / (N-1))
        let mut real = vec![0.0f32; fft_size];
        let mut imag = vec![0.0f32; fft_size];

        for i in 0..fft_size {
            let w = 0.5 - 0.5 * (2.0 * PI * (i as f32) / (fft_size as f32 - 1.0)).cos();
            real[i] = samples[i] * w;
        }

        // Radix-2 Cooley-Tukey FFT algorithm
        Self::fft_radix2(&mut real, &mut imag);

        let half = fft_size / 2;
        let mut bin_frequencies = Vec::with_capacity(half);
        let mut magnitudes_db = Vec::with_capacity(half);

        let freq_resolution = sample_rate_hz / (fft_size as f32);
        let mut max_mag = f32::MIN;
        let mut peak_idx = 1usize;
        let mut dominant_freq_hz = 0.0f32;
        let mut total_power = 0.0f32;

        for i in 0..half {
            let freq = (i as f32) * freq_resolution;
            let power = real[i] * real[i] + imag[i] * imag[i];
            let mag = (power / (fft_size as f32)).sqrt();
            let mag_db = 20.0 * (mag + 1e-6).log10();

            bin_frequencies.push(freq);
            magnitudes_db.push(mag_db);

            // Ignore DC bin 0 for dominant frequency search
            if i > 0 && mag > max_mag {
                max_mag = mag;
                peak_idx = i;
                dominant_freq_hz = freq;
            }
            if i > 0 {
                total_power += power;
            }
        }

        // Sum mainlobe signal power around peak (Hann window spreads over adjacent bins)
        let low_bin = peak_idx.saturating_sub(2).max(1);
        let high_bin = (peak_idx + 2).min(half.saturating_sub(1));
        let mut signal_power = 0.0f32;
        for i in low_bin..=high_bin {
            signal_power += real[i] * real[i] + imag[i] * imag[i];
        }

        let noise_power = (total_power - signal_power).max(1e-6);
        let snr_db = 10.0 * (signal_power / noise_power).log10().clamp(-20.0, 90.0);
        let thd_percent =
            ((noise_power / signal_power.max(1e-6)).sqrt() * 100.0).clamp(0.01, 100.0);

        FftSpectrum {
            sample_rate_hz,
            bin_frequencies,
            magnitudes_db,
            dominant_freq_hz,
            snr_db,
            thd_percent,
        }
    }

    /// Internal Radix-2 decimation-in-time Cooley-Tukey FFT
    fn fft_radix2(real: &mut [f32], imag: &mut [f32]) {
        let n = real.len();
        if n <= 1 {
            return;
        }

        // Bit-reversal permutation
        let mut j = 0usize;
        for i in 0..n {
            if i < j {
                real.swap(i, j);
                imag.swap(i, j);
            }
            let mut k = n >> 1;
            while k > 0 && j >= k {
                j -= k;
                k >>= 1;
            }
            j += k;
        }

        // Butterfly computations
        let mut len = 2usize;
        while len <= n {
            let half = len / 2;
            let angle = -2.0 * PI / (len as f32);
            let w_step_re = angle.cos();
            let w_step_im = angle.sin();

            let mut i = 0usize;
            while i < n {
                let mut w_re = 1.0f32;
                let mut w_im = 0.0f32;

                for k in 0..half {
                    let u_re = real[i + k];
                    let u_im = imag[i + k];

                    let v_re = real[i + k + half] * w_re - imag[i + k + half] * w_im;
                    let v_im = real[i + k + half] * w_im + imag[i + k + half] * w_re;

                    real[i + k] = u_re + v_re;
                    imag[i + k] = u_im + v_im;

                    real[i + k + half] = u_re - v_re;
                    imag[i + k + half] = u_im - v_im;

                    let next_w_re = w_re * w_step_re - w_im * w_step_im;
                    let next_w_im = w_re * w_step_im + w_im * w_step_re;
                    w_re = next_w_re;
                    w_im = next_w_im;
                }
                i += len;
            }
            len <<= 1;
        }
    }

    /// Finds index matching signal trigger threshold and edge direction
    pub fn find_trigger_index(samples: &[f32], trigger: &TriggerConfig) -> Option<usize> {
        if samples.len() < 2 {
            return None;
        }

        for i in 1..samples.len() {
            let prev = samples[i - 1];
            let curr = samples[i];
            let lvl = trigger.level_volts;

            let is_match = match trigger.edge {
                TriggerEdge::Rising => prev <= lvl && curr > lvl,
                TriggerEdge::Falling => prev >= lvl && curr < lvl,
                TriggerEdge::Either => (prev <= lvl && curr > lvl) || (prev >= lvl && curr < lvl),
            };

            if is_match {
                return Some(i);
            }
        }

        if trigger.auto_mode {
            Some(0)
        } else {
            None
        }
    }

    /// Decodes UART 8N1 serial protocol from a digital binary sample stream
    pub fn decode_uart(
        digital_samples: &[u8],
        sample_rate_hz: f32,
        baud_rate: u32,
    ) -> Vec<DecodedPacket> {
        let mut packets = Vec::new();
        let samples_per_bit = (sample_rate_hz / baud_rate as f32).round() as usize;
        if samples_per_bit < 2 || digital_samples.len() < samples_per_bit * 10 {
            return packets;
        }

        let mut idx = 0usize;
        let mut packet_id = 1u32;

        while idx + samples_per_bit * 10 < digital_samples.len() {
            // Hunt for START bit: idle high (1) transitions to low (0)
            if digital_samples[idx] == 1 && digital_samples[idx + 1] == 0 {
                // Jump to center of START bit
                let start_center = idx + samples_per_bit / 2;
                if digital_samples[start_center] == 0 {
                    let mut byte_val = 0u8;
                    let mut valid = true;

                    // Sample 8 data bits (LSB first)
                    for bit in 0..8 {
                        let sample_pos = start_center + samples_per_bit * (bit + 1);
                        if sample_pos >= digital_samples.len() {
                            valid = false;
                            break;
                        }
                        let bit_val = digital_samples[sample_pos];
                        if bit_val == 1 {
                            byte_val |= 1 << bit;
                        }
                    }

                    // Verify STOP bit (must be 1)
                    let stop_pos = start_center + samples_per_bit * 9;
                    if valid && stop_pos < digital_samples.len() {
                        let is_stop_ok = digital_samples[stop_pos] == 1;
                        let error_flags = if is_stop_ok {
                            None
                        } else {
                            Some("Framing error: invalid STOP bit".to_string())
                        };

                        let ascii_char = if (32..=126).contains(&byte_val) {
                            (byte_val as char).to_string()
                        } else {
                            ".".to_string()
                        };

                        let timestamp_us =
                            ((idx as f64 / sample_rate_hz as f64) * 1_000_000.0) as u64;

                        packets.push(DecodedPacket {
                            packet_id,
                            timestamp_us,
                            protocol: ProtocolType::Uart,
                            address_hex: None,
                            data_hex: format!("0x{:02X}", byte_val),
                            ascii_preview: ascii_char,
                            is_ack: is_stop_ok,
                            error_flags,
                        });

                        packet_id += 1;
                        idx += samples_per_bit * 10;
                        continue;
                    }
                }
            }
            idx += 1;
        }

        packets
    }

    /// Synthesizes clean mathematical waveforms for test verification and live demo simulation
    pub fn generate_synthetic_waveform(
        waveform_type: WaveformType,
        num_samples: usize,
        sample_rate_hz: f32,
        frequency_hz: f32,
        amplitude_volts: f32,
    ) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = (i as f32) / sample_rate_hz;
            let phase = 2.0 * PI * frequency_hz * t;

            let val = match waveform_type {
                WaveformType::Sine => {
                    // Sine with fundamental + subtle 2nd harmonic
                    amplitude_volts * (phase.sin() + 0.15 * (2.0 * phase).sin())
                }
                WaveformType::Square => {
                    if phase.sin() >= 0.0 {
                        amplitude_volts
                    } else {
                        -amplitude_volts
                    }
                }
                WaveformType::Triangle => {
                    let p = (phase / (2.0 * PI)).fract();
                    if p < 0.5 {
                        amplitude_volts * (4.0 * p - 1.0)
                    } else {
                        amplitude_volts * (3.0 - 4.0 * p)
                    }
                }
                WaveformType::Sawtooth => {
                    let p = (phase / (2.0 * PI)).fract();
                    amplitude_volts * (2.0 * p - 1.0)
                }
                WaveformType::Noise => {
                    // Deterministic pseudo-random noise generator
                    let pseudorand = ((i * 1664525 + 1013904223) % 1000) as f32 / 1000.0;
                    amplitude_volts * (pseudorand * 2.0 - 1.0)
                }
                WaveformType::UartStream => {
                    // Digital pulse stream: start bit (LOW), data bits 1..8, stop bit (HIGH)
                    let bit = ((i / 20) % 10) as u8;
                    if bit == 9 || (bit > 0 && (bit % 2) == 1) {
                        amplitude_volts
                    } else {
                        0.0
                    }
                }
            };

            samples.push(val);
        }

        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_statistics_sine() {
        let sample_rate = 100_000.0;
        let freq = 1_000.0;
        let amp = 2.5;
        let samples = SignalDspEngine::generate_synthetic_waveform(
            WaveformType::Sine,
            1000,
            sample_rate,
            freq,
            amp,
        );

        let stats = SignalDspEngine::compute_statistics(&samples, sample_rate);
        assert!((stats.vpp - (amp * 2.0)).abs() < 1.0);
        assert!(stats.vrms > 1.5 && stats.vrms < 2.5);
        assert!((stats.frequency_hz - freq).abs() < 50.0);
    }

    #[test]
    fn test_compute_fft_spectrum() {
        let sample_rate = 20_000.0;
        let freq = 2_500.0;
        let samples = SignalDspEngine::generate_synthetic_waveform(
            WaveformType::Sine,
            1024,
            sample_rate,
            freq,
            1.0,
        );

        let spectrum = SignalDspEngine::compute_fft_spectrum(&samples, sample_rate);
        assert!(!spectrum.bin_frequencies.is_empty());
        assert!((spectrum.dominant_freq_hz - freq).abs() < 100.0);
        assert!(spectrum.snr_db > 10.0);
    }

    #[test]
    fn test_find_trigger_index() {
        let samples = vec![-1.0, -0.5, 0.2, 1.0, 0.5, -0.5, 0.5];
        let trig = TriggerConfig {
            channel: crate::models::SignalChannel::Ch1,
            level_volts: 0.0,
            edge: TriggerEdge::Rising,
            holdoff_ms: 1.0,
            auto_mode: false,
        };

        let idx = SignalDspEngine::find_trigger_index(&samples, &trig);
        assert_eq!(idx, Some(2)); // index 2 crosses from -0.5 to 0.2
    }

    #[test]
    fn test_decode_uart() {
        let sample_rate = 96_000.0;
        let baud = 9600;
        let samples_per_bit = 10usize; // 96000 / 9600 = 10

        // Transmit ASCII 'A' (0x41 = 0b01000001)
        // Bit sequence: START(0), D0(1), D1(0), D2(0), D3(0), D4(0), D5(0), D6(1), D7(0), STOP(1)
        let bits = [0u8, 1, 0, 0, 0, 0, 0, 1, 0, 1];
        let mut stream = vec![1u8; 10]; // Initial idle line high
        for &b in &bits {
            for _ in 0..samples_per_bit {
                stream.push(b);
            }
        }
        stream.extend_from_slice(&[1u8; 20]); // Trailing idle line

        let packets = SignalDspEngine::decode_uart(&stream, sample_rate, baud);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].data_hex, "0x41");
        assert_eq!(packets[0].ascii_preview, "A");
        assert!(packets[0].is_ack);
    }
}
