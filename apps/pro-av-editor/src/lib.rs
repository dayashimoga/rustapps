#![deny(unsafe_code)]
//! Professional Audio & Video Editor (PAVE) Core Library
//!
//! Ultra-fast, lightweight, 100% offline-first NLE (Non-Linear Editor) studio engine.
//! Provides frame-accurate multi-track timeline DAG, compositor, color grading,
//! parametric audio DSP, and offline AI auto-editing.
//!
//! ## Nonnegotiable Performance & Reliability Constraints
//! - Sub-16.6ms real-time frame evaluation latency (60 FPS rendering SLA).
//! - 100% offline autonomy: zero network egress, zero SaaS/cloud dependencies.
//! - Non-destructive editing DAG with transactional unlimited Undo / Redo.

pub mod ai_editor;
pub mod audio_engine;
pub mod color;
pub mod commands;
pub mod compositor;
#[allow(unsafe_code)]
pub mod ffi;
pub mod models;
pub mod service;
pub mod templates;
pub mod timeline;

pub use ai_editor::*;
pub use audio_engine::*;
pub use color::*;
pub use compositor::*;
pub use models::*;
pub use service::*;
pub use templates::*;
pub use timeline::*;
