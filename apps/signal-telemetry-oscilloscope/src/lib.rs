#![deny(unsafe_code)]
//! Offline Scientific Signal & Telemetry Oscilloscope Core Library
//!
//! Provides zero-network-egress, offline digital signal processing,
//! Radix-2 Cooley-Tukey FFT spectrum analysis, protocol decoding (UART/I2C/SPI/CAN),
//! multi-channel trace rendering metrics, and SQLite WAL capture persistence.

pub mod commands;
pub mod dsp_engine;
pub mod ffi;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use dsp_engine::SignalDspEngine;
pub use models::*;
pub use service::{OscilloscopeError, OscilloscopeService};
