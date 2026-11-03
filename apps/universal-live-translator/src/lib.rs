#![deny(unsafe_code)]
//! Universal Live Translator Core Library
//!
//! Ultra-fast, lightweight, 100% offline-first real-time voice, text,
//! conversation, camera OCR, and document translator.
//!
//! ## Nonnegotiable Performance & Security Constraints
//! - Automatic Language Identification (LID) latency strictly $< 500\,\mu\text{s}$.
//! - Text translation latency strictly $< 150\,\text{ms}$.
//! - Streaming ASR latency strictly $< 400\,\text{ms}$.
//! - 100% offline autonomy: zero network egress, zero SaaS/cloud dependencies.

pub mod camera_translate;
pub mod commands;
#[allow(unsafe_code)]
pub mod ffi;
pub mod lid;
pub mod models;
pub mod nmt;
pub mod phrasebook;
pub mod service;
pub mod speech;
pub mod subtitles;
pub mod tts;

pub use camera_translate::{CameraTranslateError, CameraTranslator};
pub use lid::{LanguageDetector, LidResult};
pub use models::*;
pub use nmt::{OfflineTranslationEngine, TranslationError};
pub use phrasebook::{PhrasebookError, PhrasebookStore, TranslationHistoryRecord};
pub use service::{ServiceError, TranslatorService};
pub use speech::{BatchSpeechRecognizer, SpeechError, StreamingSpeechRecognizer};
pub use subtitles::{SubtitleError, SubtitleTranslator};
pub use tts::{encode_pcm_to_wav, TtsError, TtsSynthesizer};
