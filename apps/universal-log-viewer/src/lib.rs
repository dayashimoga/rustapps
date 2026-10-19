#![forbid(unsafe_code)]
//! Universal Log Viewer Core Library
//!
//! High-throughput, memory-mapped log analysis engine capable of processing
//! 500GB+ logs with sub-300MB RAM consumption and sub-50ms search latency.

pub mod commands;
pub mod mmap_engine;
pub mod search_engine;
pub mod sparse_index;
pub mod tail_mode;
pub mod workspace;

// Re-export core types
pub use commands::{LogLineRecord, LogViewerService, OpenedFileInfo};
pub use mmap_engine::MmapLogEngine;
pub use search_engine::{LogQuery, LogSearchEngine, LogSearchMatch, LogSearchReport};
pub use sparse_index::SparseLineIndex;
pub use tail_mode::{TailEvent, TailFollower};
pub use workspace::{BookmarkRecord, WorkspaceRecord, WorkspaceStore};
