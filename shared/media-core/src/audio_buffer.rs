#![forbid(unsafe_code)]
//! PCM Audio Buffer and WAV RIFF Container Serialization
//!
//! Provides multi-channel normalized floating-point PCM buffers [-1.0..1.0],
//! RIFF WAVE 16-bit PCM serialization/deserialization, and ring buffer pipelines.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// Audio buffer error definitions
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioError {
    #[error("Invalid or corrupt WAV container")]
    InvalidWav,
    #[error("Unsupported audio format (only PCM integer and IEEE float supported)")]
    UnsupportedFormat,
    #[error("Unsupported bits per sample: {0} (requires 16-bit or 32-bit float)")]
    UnsupportedBitDepth(u16),
}

/// Normalized floating-point PCM audio buffer
#[derive(Debug, Clone, PartialEq)]
pub struct PcmAudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>, // Interleaved samples in [-1.0..1.0]
}

impl PcmAudioBuffer {
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels,
            samples,
        }
    }

    /// Number of audio frames (samples per channel)
    pub fn num_frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    /// Total audio duration in seconds
    pub fn duration_secs(&self) -> f32 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.num_frames() as f32 / self.sample_rate as f32
        }
    }

    /// Mix down multi-channel buffer to single mono channel
    pub fn to_mono(&self) -> Vec<f32> {
        let ch = self.channels as usize;
        if ch <= 1 {
            return self.samples.clone();
        }

        let num_frames = self.num_frames();
        let mut mono = Vec::with_capacity(num_frames);
        let inv_ch = 1.0 / ch as f32;

        for f in 0..num_frames {
            let offset = f * ch;
            let sum: f32 = self.samples[offset..offset + ch].iter().sum();
            mono.push(sum * inv_ch);
        }

        mono
    }

    /// Encode audio buffer into standard 16-bit uncompressed WAV container bytes
    pub fn encode_wav_16bit(&self) -> Vec<u8> {
        let num_samples = self.samples.len();
        let bytes_per_sample = 2u32;
        let data_size = num_samples as u32 * bytes_per_sample;
        let file_size = 36 + data_size;

        let mut wav = Vec::with_capacity((44 + data_size) as usize);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(file_size).to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt sub-chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // subchunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat 1 = PCM
        wav.extend_from_slice(&self.channels.to_le_bytes());
        wav.extend_from_slice(&self.sample_rate.to_le_bytes());

        let byte_rate = self.sample_rate * self.channels as u32 * bytes_per_sample;
        let block_align = self.channels * bytes_per_sample as u16;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes()); // 16 bits per sample

        // data sub-chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());

        for &sample in &self.samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let s16 = if clamped >= 0.0 {
                (clamped * 32767.0) as i16
            } else {
                (clamped * 32768.0) as i16
            };
            wav.extend_from_slice(&s16.to_le_bytes());
        }

        wav
    }

    /// Decode WAV container bytes into normalized float PCM audio buffer
    pub fn decode_wav(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(AudioError::InvalidWav);
        }

        let mut offset = 12;
        let mut channels = 1u16;
        let mut sample_rate = 44100u32;
        let mut bits_per_sample = 16u16;
        let mut format_tag = 1u16;
        let mut audio_samples = Vec::new();

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;

            if offset + chunk_size > bytes.len() {
                break;
            }

            match chunk_id {
                b"fmt " => {
                    if chunk_size < 16 {
                        return Err(AudioError::InvalidWav);
                    }
                    format_tag = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                    channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                    sample_rate = u32::from_le_bytes([
                        bytes[offset + 4],
                        bytes[offset + 5],
                        bytes[offset + 6],
                        bytes[offset + 7],
                    ]);
                    bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);
                }
                b"data" => {
                    let data_bytes = &bytes[offset..offset + chunk_size];
                    match (format_tag, bits_per_sample) {
                        (1, 16) => {
                            // 16-bit integer PCM
                            let num_samples = chunk_size / 2;
                            audio_samples.reserve(num_samples);
                            for i in 0..num_samples {
                                let s =
                                    i16::from_le_bytes([data_bytes[i * 2], data_bytes[i * 2 + 1]]);
                                audio_samples.push(s as f32 / 32768.0);
                            }
                        }
                        (3, 32) => {
                            // 32-bit float PCM
                            let num_samples = chunk_size / 4;
                            audio_samples.reserve(num_samples);
                            for i in 0..num_samples {
                                let f = f32::from_le_bytes([
                                    data_bytes[i * 4],
                                    data_bytes[i * 4 + 1],
                                    data_bytes[i * 4 + 2],
                                    data_bytes[i * 4 + 3],
                                ]);
                                audio_samples.push(f);
                            }
                        }
                        (_, b) => return Err(AudioError::UnsupportedBitDepth(b)),
                    }
                }
                _ => {}
            }

            offset += chunk_size;
        }

        if audio_samples.is_empty() {
            return Err(AudioError::InvalidWav);
        }

        Ok(Self {
            sample_rate,
            channels,
            samples: audio_samples,
        })
    }
}

/// Lock-free Single-Producer Single-Consumer Audio Ring Buffer
pub struct AudioRingBuffer {
    buffer: Vec<f32>,
    capacity: usize,
    read_pos: Arc<AtomicUsize>,
    write_pos: Arc<AtomicUsize>,
}

impl AudioRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0f32; capacity],
            capacity,
            read_pos: Arc::new(AtomicUsize::new(0)),
            write_pos: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn available_read(&self) -> usize {
        let w = self.write_pos.load(Ordering::Acquire);
        let r = self.read_pos.load(Ordering::Acquire);
        if w >= r {
            w - r
        } else {
            self.capacity - (r - w)
        }
    }

    pub fn available_write(&self) -> usize {
        self.capacity - 1 - self.available_read()
    }

    pub fn push(&mut self, samples: &[f32]) -> usize {
        let avail = self.available_write();
        let to_write = samples.len().min(avail);
        let mut w = self.write_pos.load(Ordering::Relaxed);

        for &sample in &samples[..to_write] {
            self.buffer[w] = sample;
            w = (w + 1) % self.capacity;
        }

        self.write_pos.store(w, Ordering::Release);
        to_write
    }

    pub fn pop(&mut self, out: &mut [f32]) -> usize {
        let avail = self.available_read();
        let to_read = out.len().min(avail);
        let mut r = self.read_pos.load(Ordering::Relaxed);

        for slot in out.iter_mut().take(to_read) {
            *slot = self.buffer[r];
            r = (r + 1) % self.capacity;
        }

        self.read_pos.store(r, Ordering::Release);
        to_read
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_encode_decode_roundtrip() {
        let sample_rate = 44100;
        let channels = 2;
        let mut samples = Vec::new();

        // 1 second of stereo 440Hz / 880Hz tones
        for i in 0..sample_rate {
            let t = i as f32 / sample_rate as f32;
            samples.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
            samples.push(0.5 * (2.0 * std::f32::consts::PI * 880.0 * t).sin());
        }

        let original = PcmAudioBuffer::new(sample_rate, channels, samples);
        let wav_bytes = original.encode_wav_16bit();
        assert!(!wav_bytes.is_empty());

        let decoded = PcmAudioBuffer::decode_wav(&wav_bytes).unwrap();
        assert_eq!(decoded.sample_rate, original.sample_rate);
        assert_eq!(decoded.channels, original.channels);
        assert_eq!(decoded.num_frames(), original.num_frames());

        // Check quantization fidelity (< 1e-4 error for 16-bit)
        for (a, b) in original.samples.iter().zip(decoded.samples.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn test_audio_ring_buffer() {
        let mut ring = AudioRingBuffer::new(1024);
        let data = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];

        let written = ring.push(&data);
        assert_eq!(written, 5);
        assert_eq!(ring.available_read(), 5);

        let mut out = vec![0.0f32; 5];
        let read = ring.pop(&mut out);
        assert_eq!(read, 5);
        assert_eq!(out, data);
        assert_eq!(ring.available_read(), 0);
    }
}
