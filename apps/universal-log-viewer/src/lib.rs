//! Universal Log Viewer Core Library
//!
//! High-throughput, memory-mapped log analysis engine capable of processing
//! 500GB+ logs with sub-300MB RAM consumption and sub-50ms search latency.

pub mod cloud_importers;
pub mod commands;
pub mod ffi;
pub mod local_ai;
pub mod mmap_engine;
pub mod parsers;
pub mod plugin_integration;
pub mod redaction;
pub mod search_engine;
pub mod sparse_index;
pub mod tail_mode;
pub mod timeline;
pub mod trace_merger;
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
    compute_field_aggregations, detect_format, parse_timestamp, FieldAggregationReport,
    FieldBreakdown, FieldValueCount, LogFormat, LogSeverity, NormalizedLogRecord,
    StreamingCsvParser, StreamingJsonParser,
};
pub use plugin_integration::LogViewerPluginManager;
pub use redaction::{RedactionEngine, RedactionError};
pub use search_engine::{LogQuery, LogSearchEngine, LogSearchMatch, LogSearchReport};
pub use sparse_index::SparseLineIndex;
pub use tail_mode::{TailEvent, TailFollower};
pub use timeline::{TimelineBucket, TimelineHistogram, TimelineIndex};
pub use trace_merger::{
    correlate_trace_waterfall, merge_log_files_chronologically, MergedLogSummary,
    TraceWaterfallReport, TraceWaterfallSpan,
};
pub use workspace::{BookmarkRecord, WorkspaceRecord, WorkspaceStore};
