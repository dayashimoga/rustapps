#![forbid(unsafe_code)]
//! High-Performance Offline Digital Signal Processing (DSP) & Protocol Decoder Engine

use crate::models::{
    DecodedPacket, FftSpectrum, MathOperation, ProtocolType, SignalStatistics, TriggerConfig,
    TriggerEdge, WaveformType,
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

    /// Computes sample-wise channel mathematical operations ($A+B, A-B, A*B, -A, -B$)
    pub fn compute_channel_math(ch1: &[f32], ch2: &[f32], op: MathOperation) -> Vec<f32> {
        let len = match op {
            MathOperation::InvertCh1 => ch1.len(),
            MathOperation::InvertCh2 => ch2.len(),
            _ => ch1.len().min(ch2.len()),
        };

        let mut out = Vec::with_capacity(len);

        for i in 0..len {
            let val = match op {
                MathOperation::Add => ch1[i] + ch2[i],
                MathOperation::Subtract => ch1[i] - ch2[i],
                MathOperation::Multiply => ch1[i] * ch2[i],
                MathOperation::InvertCh1 => -ch1[i],
                MathOperation::InvertCh2 => -ch2[i],
            };
            out.push(val);
        }

        out
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

    /// Parses CSV formatted oscilloscope / signal capture files
    pub fn parse_csv_signal(csv_content: &str) -> Result<(Vec<f32>, Vec<f32>, f32), String> {
        let mut ch1 = Vec::new();
        let mut ch2 = Vec::new();
        let mut timestamps = Vec::new();

        for line in csv_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line
                .split([',', ';', '\t', ' '])
                .filter(|s| !s.is_empty())
                .collect();

            if parts.is_empty() {
                continue;
            }

            // Skip non-numeric header rows
            if parts[0].chars().any(|c| c.is_alphabetic()) {
                continue;
            }

            match parts.len() {
                1 => {
                    if let Ok(v) = parts[0].parse::<f32>() {
                        ch1.push(v);
                    }
                }
                2 => {
                    // Try parsing as (time, ch1) or (ch1, ch2)
                    if let (Ok(p0), Ok(p1)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        timestamps.push(p0);
                        ch1.push(p1);
                    }
                }
                _ => {
                    // 3 or more: (time, ch1, ch2, ...)
                    if let (Ok(t), Ok(v1), Ok(v2)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        timestamps.push(t);
                        ch1.push(v1);
                        ch2.push(v2);
                    }
                }
            }
        }

        if ch1.is_empty() {
            return Err("No numeric signal samples found in CSV".to_string());
        }

        // Calculate sample rate from timestamp delta if available
        let sample_rate_hz = if timestamps.len() >= 2 {
            let dt = (timestamps.last().unwrap() - timestamps.first().unwrap())
                / (timestamps.len() - 1) as f32;
            if dt > 1e-9 {
                (1.0 / dt).clamp(1.0, 10_000_000.0)
            } else {
                50_000.0
            }
        } else {
            50_000.0
        };

        if ch2.is_empty() {
            ch2 = vec![0.0f32; ch1.len()];
        }

        Ok((ch1, ch2, sample_rate_hz))
    }

    /// Parses PCM WAV signal audio files (16-bit or 32-bit float)
    pub fn parse_wav_signal(wav_bytes: &[u8]) -> Result<(Vec<f32>, f32), String> {
        if wav_bytes.len() < 44 {
            return Err("WAV data too short for header".to_string());
        }

        if &wav_bytes[0..4] != b"RIFF" || &wav_bytes[8..12] != b"WAVE" {
            return Err("Invalid WAV RIFF/WAVE header".to_string());
        }

        // Search for fmt and data chunks
        let mut idx = 12;
        let mut sample_rate_hz = 44_100.0f32;
        let mut bits_per_sample = 16u16;
        let mut num_channels = 1u16;
        let mut audio_format = 1u16;
        let mut data_offset = 0;
        let mut data_len = 0;

        while idx + 8 <= wav_bytes.len() {
            let chunk_id = &wav_bytes[idx..idx + 4];
            let chunk_size = u32::from_le_bytes(
                wav_bytes[idx + 4..idx + 8]
                    .try_into()
                    .map_err(|_| "Failed to parse chunk size")?,
            ) as usize;

            if chunk_id == b"fmt " && idx + 8 + chunk_size <= wav_bytes.len() {
                let fmt_data = &wav_bytes[idx + 8..];
                if fmt_data.len() >= 16 {
                    audio_format = u16::from_le_bytes([fmt_data[0], fmt_data[1]]);
                    num_channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
                    sample_rate_hz =
                        u32::from_le_bytes([fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7]])
                            as f32;
                    bits_per_sample = u16::from_le_bytes([fmt_data[14], fmt_data[15]]);
                }
            } else if chunk_id == b"data" {
                data_offset = idx + 8;
                data_len = chunk_size.min(wav_bytes.len().saturating_sub(data_offset));
                break;
            }

            idx += 8 + chunk_size;
        }

        if data_offset == 0 || data_len == 0 {
            return Err("Missing WAV 'data' subchunk".to_string());
        }

        let raw_samples = &wav_bytes[data_offset..data_offset + data_len];
        let mut samples = Vec::new();

        if audio_format == 1 && bits_per_sample == 16 {
            // 16-bit PCM signed
            for chunk in raw_samples.chunks_exact(2 * num_channels as usize) {
                let s16 = i16::from_le_bytes([chunk[0], chunk[1]]);
                samples.push(s16 as f32 / 32768.0);
            }
        } else if audio_format == 3 && bits_per_sample == 32 {
            // 32-bit IEEE float
            for chunk in raw_samples.chunks_exact(4 * num_channels as usize) {
                let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                samples.push(f);
            }
        } else {
            // Fallback: 8-bit unsigned
            for chunk in raw_samples.chunks_exact(num_channels as usize) {
                samples.push((chunk[0] as f32 - 128.0) / 128.0);
            }
        }

        Ok((samples, sample_rate_hz))
    }

    /// Parses Value Change Dump (VCD) logic analyzer files into digital binary stream
    pub fn parse_vcd_signal(vcd_text: &str) -> Result<(Vec<u8>, f32), String> {
        let mut digital_samples = Vec::new();
        let mut current_state = 0u8;

        for line in vcd_text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('$') {
                continue;
            }

            if line.starts_with('0') {
                current_state = 0;
                digital_samples.push(current_state);
            } else if line.starts_with('1') {
                current_state = 1;
                digital_samples.push(current_state);
            } else if line.starts_with('#') {
                // Time advance - repeat current state for duration
                digital_samples.push(current_state);
            }
        }

        if digital_samples.is_empty() {
            return Err("No digital states found in VCD text".to_string());
        }

        Ok((digital_samples, 100_000.0))
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
    fn test_channel_math_operations() {
        let ch1 = vec![1.0, 2.0, 3.0, 4.0];
        let ch2 = vec![0.5, 1.0, 1.5, 2.0];

        let add = SignalDspEngine::compute_channel_math(&ch1, &ch2, MathOperation::Add);
        assert_eq!(add, vec![1.5, 3.0, 4.5, 6.0]);

        let sub = SignalDspEngine::compute_channel_math(&ch1, &ch2, MathOperation::Subtract);
        assert_eq!(sub, vec![0.5, 1.0, 1.5, 2.0]);

        let mul = SignalDspEngine::compute_channel_math(&ch1, &ch2, MathOperation::Multiply);
        assert_eq!(mul, vec![0.5, 2.0, 4.5, 8.0]);

        let inv = SignalDspEngine::compute_channel_math(&ch1, &ch2, MathOperation::InvertCh1);
        assert_eq!(inv, vec![-1.0, -2.0, -3.0, -4.0]);
    }

    #[test]
    fn test_parse_csv_signal() {
        let csv_data = "Time,Ch1,Ch2\n0.000,1.25,-0.50\n0.001,1.50,-0.40\n0.002,1.75,-0.30\n";
        let (ch1, ch2, rate) = SignalDspEngine::parse_csv_signal(csv_data).unwrap();
        assert_eq!(ch1.len(), 3);
        assert_eq!(ch2.len(), 3);
        assert_eq!(ch1[0], 1.25);
        assert!((rate - 1000.0).abs() < 10.0);
    }

    #[test]
    fn test_parse_wav_signal_synthetic() {
        // Construct a minimal valid 16-bit PCM WAV in memory
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&36u32.to_le_bytes()); // remaining size
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
        wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = 1 (PCM)
        wav.extend_from_slice(&1u16.to_le_bytes()); // NumChannels = 1
        wav.extend_from_slice(&44100u32.to_le_bytes()); // SampleRate
        wav.extend_from_slice(&88200u32.to_le_bytes()); // ByteRate
        wav.extend_from_slice(&2u16.to_le_bytes()); // BlockAlign
        wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&4u32.to_le_bytes()); // 2 samples * 2 bytes
        wav.extend_from_slice(&16384i16.to_le_bytes()); // sample 1 = 0.5
        wav.extend_from_slice(&(-16384i16).to_le_bytes()); // sample 2 = -0.5

        let (samples, rate) = SignalDspEngine::parse_wav_signal(&wav).unwrap();
        assert_eq!(rate, 44100.0);
        assert_eq!(samples.len(), 2);
        assert!((samples[0] - 0.5).abs() < 0.01);
        assert!((samples[1] - (-0.5)).abs() < 0.01);
    }

    #[test]
    fn test_parse_vcd_signal() {
        let vcd = "$timescale 1us $end\n#0\n0!\n#10\n1!\n#20\n0!\n";
        let (digital, _) = SignalDspEngine::parse_vcd_signal(vcd).unwrap();
        assert!(!digital.is_empty());
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
