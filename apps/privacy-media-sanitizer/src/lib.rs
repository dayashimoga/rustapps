//! Forensic Air-Gapped Media & Document Privacy Sanitizer
//!
//! Deep EXIF/XMP/IPTC and PDF metadata inspection, zero-trace binary purging,
//! camera serial number destruction, GPS geolocation zeroing, and Ed25519 audit certificates.

pub mod commands;
pub mod ffi;
pub mod models;
pub mod sanitizer_engine;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use models::*;
pub use sanitizer_engine::SanitizerEngine;
pub use service::SanitizerService;
