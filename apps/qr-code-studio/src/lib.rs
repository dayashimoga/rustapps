//! Offline QR Code Scanner & Generator Studio Library Core
//!
//! Pure-Rust ISO/IEC 18004 QR engine, threat heuristics analyzer,
//! SQLite WAL history storage, and desktop/mobile IPC bindings.

pub mod commands;
pub mod ffi;
pub mod label_sheet;
pub mod models;
pub mod qr_engine;
pub mod security_analyzer;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use label_sheet::*;
pub use models::*;
pub use qr_engine::*;
pub use security_analyzer::*;
pub use service::QrStudioService;
