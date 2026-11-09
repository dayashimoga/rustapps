//! Offline Local RAG Legal & Compliance Contract Auditor
//!
//! Air-gapped intelligent contract auditor, semantic clause extraction,
//! benchmark risk analysis, redline generation, version diffing,
//! and Ed25519 legal audit certificate signing.

pub mod commands;
pub mod ffi;
pub mod models;
pub mod rag_engine;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use models::*;
pub use rag_engine::RagEngine;
pub use service::ContractService;
