#![forbid(unsafe_code)]
//! Core Domain Models for Offline Scientific Signal & Telemetry Oscilloscope

use serde::{Deserialize, Serialize};

/// Oscilloscope hardware or virtual capture channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SignalChannel {
    #[default]
    Ch1,
    Ch2,
    DigitalD0,
    DigitalD1,
}

/// Synthesized or analyzed signal waveform geometry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WaveformType {
    #[default]
    Sine,
    Square,
    Triangle,
    Sawtooth,
    Noise,
    UartStream,
}

/// Comprehensive waveform statistical measurements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SignalStatistics {
    pub vpp: f32,
    pub vrms: f32,
    pub vmin: f32,
    pub vmax: f32,
    pub vmean: f32,
    pub frequency_hz: f32,
    pub period_ms: f32,
    pub duty_cycle_percent: f32,
}

/// Fast Fourier Transform (FFT) frequency spectrum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FftSpectrum {
    pub sample_rate_hz: f32,
    pub bin_frequencies: Vec<f32>,
    pub magnitudes_db: Vec<f32>,
    pub dominant_freq_hz: f32,
    pub snr_db: f32,
    pub thd_percent: f32,
}

/// Low-level serial bus protocol decoder standard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProtocolType {
    #[default]
    Uart,
    I2c,
    Spi,
    Can,
}

/// Decoded serial transmission packet
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedPacket {
    pub packet_id: u32,
    pub timestamp_us: u64,
    pub protocol: ProtocolType,
    pub address_hex: Option<String>,
    pub data_hex: String,
    pub ascii_preview: String,
    pub is_ack: bool,
    pub error_flags: Option<String>,
}

/// Signal edge direction for hardware/software triggering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TriggerEdge {
    #[default]
    Rising,
    Falling,
    Either,
}

/// Trigger condition configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub channel: SignalChannel,
    pub level_volts: f32,
    pub edge: TriggerEdge,
    pub holdoff_ms: f32,
    pub auto_mode: bool,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            channel: SignalChannel::Ch1,
            level_volts: 0.0,
            edge: TriggerEdge::Rising,
            holdoff_ms: 10.0,
            auto_mode: true,
        }
    }
}

/// Per-channel visual and acquisition configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel: SignalChannel,
    pub enabled: bool,
    pub volts_per_div: f32,
    pub offset_volts: f32,
    pub coupling: String,
    pub color_hex: String,
}

/// Saved oscilloscope capture session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureSession {
    pub session_id: String,
    pub title: String,
    pub created_at: i64,
    pub sample_rate_hz: f32,
    pub total_samples: usize,
    pub time_per_div_ms: f32,
    pub ch1_samples: Vec<f32>,
    pub ch2_samples: Vec<f32>,
    pub digital_samples: Vec<u8>,
    pub stats_ch1: SignalStatistics,
    pub stats_ch2: SignalStatistics,
    pub spectrum: FftSpectrum,
    pub decoded_packets: Vec<DecodedPacket>,
}

/// Aggregated oscilloscope engine metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OscilloscopeStats {
    pub total_captures: usize,
    pub total_samples_analyzed: usize,
    pub total_packets_decoded: usize,
    pub avg_dominant_freq_hz: f32,
    pub avg_snr_db: f32,
}
