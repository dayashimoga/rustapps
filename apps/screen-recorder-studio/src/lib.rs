#![deny(unsafe_code)]
//! Offline AI Screen Recorder & Interactive Tutorial Builder Library

pub mod commands;
pub mod engine;
pub mod ffi;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use engine::TutorialEngine;
pub use models::{
    AudioSilenceInterval, ClickType, CursorEvent, ExportConfig, ExportFormat, RecordingSession,
    StudioStats, SubtitleCue, TutorialStep, ZoomKeyframe,
};
pub use service::{RecorderService, StudioError};
