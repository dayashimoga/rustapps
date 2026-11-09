//! Air-Gapped Academic & Technical Markdown Publishing Press.
//!
//! Multi-template typesetter (IEEE, ACM, Springer, Enterprise SOP),
//! BibTeX citation manager, cross-reference verifier, and Ed25519 copyright certificate issuer.

pub mod commands;
pub mod ffi;
pub mod models;
pub mod publishing_engine;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use models::*;
pub use publishing_engine::PublishingEngine;
pub use service::PressService;
