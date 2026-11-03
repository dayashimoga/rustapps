#![forbid(unsafe_code)]
//! Shared Media & DSP Processing Core Subsystem
//!
//! Provides PCM audio buffers, WAV container encode/decode, 8-band parametric EQ,
//! dynamics compressor, noise gate, EBU R128 LUFS metering, .cube 3D LUT grading,
//! and real-time video scopes (RGB Histogram, Luma Waveform, Vectorscope).

pub mod audio_buffer;
pub mod color;
pub mod dsp;
pub mod scopes;

pub use audio_buffer::{AudioError, AudioRingBuffer, PcmAudioBuffer};
pub use color::{apply_color_grade_rgba, ColorError, ColorGradeParams, ColorWheel, Lut3D};
pub use dsp::{AudioCompressor, BiquadFilter, FilterType, LufsMeter, NoiseGate, ParametricEq8Band};
pub use scopes::{
    LumaWaveform, LumaWaveformColumn, RgbHistogram, VectorscopeData, VideoScopeAnalyzer,
};
