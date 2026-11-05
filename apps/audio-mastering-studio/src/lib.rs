#![deny(unsafe_code)]
//! Offline AI Stem Splitter & Audio Mastering Studio Core Library
//!
//! Provides zero-network-egress, offline 4-stem separation (vocals, drums, bass, other),
//! 5-band biquad parametric equalization, dynamics compression, spectral de-reverb,
//! EBU R128 loudness measurement & normalization, and SQLite WAL project persistence.

pub mod commands;
pub mod dsp_engine;
pub mod ffi;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use dsp_engine::AudioDspEngine;
pub use models::*;
pub use service::{AudioMasteringService, MasteringError};
