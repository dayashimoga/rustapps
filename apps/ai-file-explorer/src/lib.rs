#![forbid(unsafe_code)]
//! # AI File Explorer Core Library
//!
//! Ultra-fast, lightweight, offline-first local file browser, deduplicator,
//! and storage analytics engine.
//!
//! ## Nonnegotiable Performance & Security Constraints
//! - Sub-50ms search latency across 10,000,000 indexed files.
//! - Peak resident memory (RSS) strictly below 300MB.
//! - 100% offline-first: zero network egress, zero SaaS/cloud dependencies.
//! - `#![forbid(unsafe_code)]` compliance.

pub mod analytics;
pub mod batch_rename;
pub mod commands;
pub mod crawler;
pub mod dedup;
pub mod indexer;
pub mod models;
pub mod service;
pub mod treemap;
pub mod watcher;

// Re-exports
pub use analytics::StorageAnalytics;
pub use batch_rename::BatchRenamer;
pub use crawler::FileCrawler;
pub use dedup::DuplicateDetector;
pub use indexer::FileSearchIndexer;
pub use models::*;
pub use service::FileExplorerService;
pub use treemap::TreemapGenerator;
pub use watcher::DirectoryWatcher;
