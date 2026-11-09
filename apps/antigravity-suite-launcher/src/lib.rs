//! Antigravity Offline Enterprise Multi-App Suite Launcher & Health Monitor.
//!
//! Centralized dashboard for cataloging, self-testing, and running all 23 flagship applications,
//! managing master enterprise licenses, and certifying offline air-gapped system integrity.

pub mod commands;
pub mod ffi;
pub mod launcher_engine;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use launcher_engine::LauncherEngine;
pub use models::*;
pub use service::LauncherService;
