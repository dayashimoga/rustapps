#![forbid(unsafe_code)]
//! Universal Log Viewer Core Library
//!
//! High-throughput, memory-mapped log analysis engine capable of processing
//! 500GB+ logs with sub-300MB RAM consumption and sub-50ms search latency.

pub mod cloud_importers;
pub mod commands;
pub mod local_ai;
pub mod mmap_engine;
pub mod parsers;
pub mod plugin_integration;
pub mod search_engine;
pub mod sparse_index;
pub mod tail_mode;
pub mod timeline;
pub mod workspace;

// Re-export core types
pub use cloud_importers::{
    CloudWatchImporter, CorrelatedFailureChain, KubernetesImporter, KubernetesMetadata,
};
pub use commands::{LogLineRecord, LogViewerService, OpenedFileInfo};
pub use local_ai::{
    AnomalyDetector, AnomalySpike, DrainLogMiner, ErrorCluster, LogTemplateCluster,
};
pub use mmap_engine::MmapLogEngine;
pub use parsers::{
    detect_format, parse_timestamp, LogFormat, LogSeverity, NormalizedLogRecord,
    StreamingCsvParser, StreamingJsonParser,
};
pub use plugin_integration::LogViewerPluginManager;
pub use search_engine::{LogQuery, LogSearchEngine, LogSearchMatch, LogSearchReport};
pub use sparse_index::SparseLineIndex;
pub use tail_mode::{TailEvent, TailFollower};
pub use timeline::{TimelineBucket, TimelineHistogram, TimelineIndex};
pub use workspace::{BookmarkRecord, WorkspaceRecord, WorkspaceStore};
