//! Air-Gapped Industrial Fieldbus Protocol Debugger.
//!
//! Real-time dissection for Modbus RTU/TCP, CAN 2.0A/B & CAN-FD,
//! IEC 62443 industrial cybersecurity anomaly detection, register-level analysis,
//! and Ed25519-signed fieldbus compliance certificates.

pub mod commands;
pub mod ffi;
pub mod models;
pub mod protocol_engine;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use models::*;
pub use protocol_engine::ProtocolEngine;
pub use service::DebuggerService;
