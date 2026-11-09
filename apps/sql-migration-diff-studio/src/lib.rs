//! Visual Offline SQL Schema & Migration Diff Studio
//!
//! Air-gapped database schema parser, differential AST comparator,
//! safe backward-compatibility linter, and transactional UP/DOWN migration synthesizer.

pub mod commands;
pub mod diff_engine;
pub mod ffi;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use diff_engine::DiffEngine;
pub use models::*;
pub use service::SchemaService;
