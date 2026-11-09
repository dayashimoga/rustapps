//! Offline Local Synthetic Data & Anonymization Studio Core Library
//!
//! 100% offline schema inference, referential integrity preserving multi-table synthesis,
//! Gaussian and categorical distribution cloning, and differential privacy / k-anonymity verification.

pub mod commands;
pub mod ffi;
pub mod models;
pub mod service;
pub mod synth_engine;

pub use commands::dispatch_ipc_command;
pub use models::*;
pub use service::SynthesizerService;
pub use synth_engine::SynthEngine;
