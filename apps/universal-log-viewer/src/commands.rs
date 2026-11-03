#![forbid(unsafe_code)]
//! Tauri IPC Command Interface and Service Boundary
//!
//! Provides high-level command handlers for the desktop frontend, translating
//! domain operations with `anyhow` boundaries and Serde data transfer models.

use crate::cloud_importers::{CloudWatchImporter, CorrelatedFailureChain, KubernetesImporter};
use crate::local_ai::{
    AnomalyDetector, AnomalySpike, DrainLogMiner, ErrorCluster, LogTemplateCluster,
};
use crate::mmap_engine::MmapLogEngine;
use crate::parsers::{
    detect_format, LogFormat, LogSeverity, NormalizedLogRecord, StreamingCsvParser,
    StreamingJsonParser,
};
use crate::plugin_integration::LogViewerPluginManager;
use crate::search_engine::{LogQuery, LogSearchEngine, LogSearchReport};
use crate::sparse_index::SparseLineIndex;
use crate::tail_mode::TailFollower;
use crate::timeline::{TimelineHistogram, TimelineIndex};
use crate::workspace::{BookmarkRecord, WorkspaceStore};
use platform_common::ProcessResourceUsage;
use shared_plugins::PluginManifest;
use shared_telemetry::LocalTelemetry;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Summary of an opened log file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OpenedFileInfo {
    pub path: String,
    pub total_lines: usize,
    pub file_size_bytes: u64,
}

/// A line record returned for viewport rendering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LogLineRecord {
    pub line_number: usize,
    pub byte_start: u64,
    pub byte_end: u64,
    pub text: String,
}

/// Currently active file state
struct ActiveFileState {
    path: PathBuf,
    engine: MmapLogEngine,
    index: SparseLineIndex,
    tail: Option<TailFollower>,
    detected_format: LogFormat,
    cached_records: Option<Arc<Vec<NormalizedLogRecord>>>,
    cached_timeline: Option<Arc<TimelineIndex>>,
    cached_template_clusters: Option<Arc<Vec<LogTemplateCluster>>>,
    cached_error_clusters: Option<Arc<Vec<ErrorCluster>>>,
}

/// Shared application state container
pub struct LogViewerService {
    active_file: Arc<RwLock<Option<ActiveFileState>>>,
    workspace_store: Arc<WorkspaceStore>,
    telemetry: LocalTelemetry,
    plugin_manager: LogViewerPluginManager,
}

impl LogViewerService {
    /// Create a new service instance.
    pub fn new(workspace_store: Arc<WorkspaceStore>, telemetry: LocalTelemetry) -> Self {
        Self {
            active_file: Arc::new(RwLock::new(None)),
            workspace_store,
            telemetry,
            plugin_manager: LogViewerPluginManager::new(),
        }
    }

    /// Access plugin manager
    pub fn plugin_manager(&self) -> &LogViewerPluginManager {
        &self.plugin_manager
    }

    /// Open a log file, map it into virtual memory, and build its sparse index.
    pub async fn open_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<OpenedFileInfo> {
        let path_ref = path.as_ref();
        let engine = MmapLogEngine::open(path_ref)?;
        let index = SparseLineIndex::build(engine.as_bytes());

        // Sample initial lines for format detection
        let mut sample_lines = Vec::new();
        let bytes = engine.as_bytes();
        let sample_limit = index.total_lines().min(20);
        for i in 0..sample_limit {
            if let Some((start, end)) = index.get_line_byte_range(i, bytes) {
                if let Ok(text) = engine.read_text_range(start, end) {
                    sample_lines.push(text);
                }
            }
        }
        let sample_refs: Vec<&str> = sample_lines.iter().map(String::as_str).collect();
        let detected_format = detect_format(&sample_refs);

        let file_info = OpenedFileInfo {
            path: path_ref.to_string_lossy().to_string(),
            total_lines: index.total_lines(),
            file_size_bytes: engine.file_size(),
        };

        {
            let mut active_lock = self.active_file.write().await;
            *active_lock = Some(ActiveFileState {
                path: path_ref.to_path_buf(),
                engine,
                index,
                tail: None,
                detected_format,
                cached_records: None,
                cached_timeline: None,
                cached_template_clusters: None,
                cached_error_clusters: None,
            });
        }

        self.telemetry.record_counter("log_files_opened", 1);

        Ok(file_info)
    }

    /// Retrieve detected format of active file
    pub async fn get_detected_format(&self) -> anyhow::Result<LogFormat> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;
        Ok(state.detected_format)
    }

    /// Retrieve a slice of lines for virtualized viewport rendering (sub-millisecond latency).
    pub async fn get_lines_slice(
        &self,
        start_line: usize,
        count: usize,
    ) -> anyhow::Result<Vec<LogLineRecord>> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let total_lines = state.index.total_lines();
        let end_line = (start_line + count).min(total_lines);

        let mut lines = Vec::with_capacity(end_line.saturating_sub(start_line));
        let bytes = state.engine.as_bytes();

        for line_idx in start_line..end_line {
            if let Some((start, end)) = state.index.get_line_byte_range(line_idx, bytes) {
                let text = state.engine.read_text_range(start, end)?;
                lines.push(LogLineRecord {
                    line_number: line_idx,
                    byte_start: start,
                    byte_end: end,
                    text,
                });
            }
        }

        Ok(lines)
    }

    /// Retrieve specific lines by line index list (e.g. for timerange filtering or search hits)
    pub async fn get_lines_by_indices(
        &self,
        line_numbers: &[usize],
    ) -> anyhow::Result<Vec<LogLineRecord>> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let bytes = state.engine.as_bytes();
        let mut lines = Vec::with_capacity(line_numbers.len());

        for &line_idx in line_numbers {
            if let Some((start, end)) = state.index.get_line_byte_range(line_idx, bytes) {
                let text = state.engine.read_text_range(start, end)?;
                lines.push(LogLineRecord {
                    line_number: line_idx,
                    byte_start: start,
                    byte_end: end,
                    text,
                });
            }
        }

        Ok(lines)
    }

    /// Parse normalized records from the active file
    pub async fn parse_records(
        &self,
        max_records: usize,
    ) -> anyhow::Result<Vec<NormalizedLogRecord>> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        // If all records are cached and cover the requested count, slice from cache
        if let Some(ref cached) = state.cached_records {
            let limit = cached.len().min(max_records);
            return Ok(cached[..limit].to_vec());
        }

        let total_lines = state.index.total_lines();
        let limit = total_lines.min(max_records);
        let bytes = state.engine.as_bytes();

        let mut records = Vec::with_capacity(limit);
        let format = state.detected_format;
        let k8s_meta =
            KubernetesImporter::extract_metadata_from_path(&state.path.to_string_lossy());

        let csv_parser = if format == LogFormat::Csv || format == LogFormat::Tsv {
            let delim = if format == LogFormat::Tsv {
                b'\t'
            } else {
                b','
            };
            let mut p = StreamingCsvParser::new(delim);
            if let Some((start, end)) = state.index.get_line_byte_range(0, bytes) {
                if let Ok(hdr) = state.engine.read_text_range(start, end) {
                    p.with_header_line(&hdr);
                }
            }
            Some(p)
        } else {
            None
        };

        for line_idx in 0..limit {
            if let Some((start, end)) = state.index.get_line_byte_range(line_idx, bytes) {
                if let Ok(text) = state.engine.read_text_range(start, end) {
                    let rec = match format {
                        LogFormat::JsonLines => StreamingJsonParser::parse_line(line_idx, &text),
                        LogFormat::CloudWatch => CloudWatchImporter::parse_line(line_idx, &text)
                            .unwrap_or_else(|| StreamingJsonParser::parse_line(line_idx, &text)),
                        LogFormat::Kubernetes => {
                            KubernetesImporter::parse_cri_line(line_idx, &text, k8s_meta.as_ref())
                        }
                        LogFormat::Csv | LogFormat::Tsv => {
                            if let Some(ref p) = csv_parser {
                                p.parse_row(line_idx, &text)
                            } else {
                                StreamingJsonParser::parse_line(line_idx, &text)
                            }
                        }
                        _ => {
                            // Raw text
                            let sev = LogSeverity::parse_str(&text);
                            NormalizedLogRecord {
                                line_number: line_idx,
                                raw: text.clone(),
                                timestamp_raw: None,
                                timestamp_epoch_ms: crate::parsers::parse_timestamp(&text),
                                severity: sev,
                                message: text,
                                source: None,
                                attributes: std::collections::BTreeMap::new(),
                            }
                        }
                    };
                    records.push(rec);
                }
            }
        }

        Ok(records)
    }

    /// Ensure timeline index, error clusters, and Drain templates are analyzed and cached
    pub async fn ensure_analyzed(&self) -> anyhow::Result<()> {
        let mut active_lock = self.active_file.write().await;
        let state = active_lock
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        if state.cached_timeline.is_some() {
            return Ok(());
        }

        let total_lines = state.index.total_lines();
        let bytes = state.engine.as_bytes();
        let format = state.detected_format;
        let k8s_meta =
            KubernetesImporter::extract_metadata_from_path(&state.path.to_string_lossy());

        let csv_parser = if format == LogFormat::Csv || format == LogFormat::Tsv {
            let delim = if format == LogFormat::Tsv {
                b'\t'
            } else {
                b','
            };
            let mut p = StreamingCsvParser::new(delim);
            if let Some((start, end)) = state.index.get_line_byte_range(0, bytes) {
                if let Ok(hdr) = state.engine.read_text_range(start, end) {
                    p.with_header_line(&hdr);
                }
            }
            Some(p)
        } else {
            None
        };

        let mut records = Vec::with_capacity(total_lines);
        let mut miner = DrainLogMiner::new();

        for line_idx in 0..total_lines {
            if let Some((start, end)) = state.index.get_line_byte_range(line_idx, bytes) {
                if let Ok(text) = state.engine.read_text_range(start, end) {
                    let rec = match format {
                        LogFormat::JsonLines => StreamingJsonParser::parse_line(line_idx, &text),
                        LogFormat::CloudWatch => CloudWatchImporter::parse_line(line_idx, &text)
                            .unwrap_or_else(|| StreamingJsonParser::parse_line(line_idx, &text)),
                        LogFormat::Kubernetes => {
                            KubernetesImporter::parse_cri_line(line_idx, &text, k8s_meta.as_ref())
                        }
                        LogFormat::Csv | LogFormat::Tsv => {
                            if let Some(ref p) = csv_parser {
                                p.parse_row(line_idx, &text)
                            } else {
                                StreamingJsonParser::parse_line(line_idx, &text)
                            }
                        }
                        _ => {
                            let sev = LogSeverity::parse_str(&text);
                            NormalizedLogRecord {
                                line_number: line_idx,
                                raw: text.clone(),
                                timestamp_raw: None,
                                timestamp_epoch_ms: crate::parsers::parse_timestamp(&text),
                                severity: sev,
                                message: text.clone(),
                                source: None,
                                attributes: std::collections::BTreeMap::new(),
                            }
                        }
                    };
                    miner.add_log(rec.line_number, &rec.message);
                    records.push(rec);
                }
            }
        }

        let timeline = TimelineIndex::from_records(&records);
        let error_clusters = AnomalyDetector::cluster_errors(&records);
        let template_clusters = miner.get_clusters();

        state.cached_records = Some(Arc::new(records));
        state.cached_timeline = Some(Arc::new(timeline));
        state.cached_template_clusters = Some(Arc::new(template_clusters));
        state.cached_error_clusters = Some(Arc::new(error_clusters));

        Ok(())
    }

    /// Retrieve or compute aggregated timeline histogram (sub-millisecond when cached)
    pub async fn get_timeline_histogram(
        &self,
        target_buckets: usize,
    ) -> anyhow::Result<TimelineHistogram> {
        self.ensure_analyzed().await?;
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let timeline = state
            .cached_timeline
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Timeline index was not cached"))?;

        timeline
            .generate_histogram(target_buckets)
            .ok_or_else(|| anyhow::anyhow!("No timestamped records available for timeline"))
    }

    /// Filter line numbers by time range [start_ms, end_ms] (sub-millisecond when cached)
    pub async fn filter_by_timerange(
        &self,
        start_ms: i64,
        end_ms: i64,
    ) -> anyhow::Result<Vec<usize>> {
        self.ensure_analyzed().await?;
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let timeline = state
            .cached_timeline
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Timeline index was not cached"))?;

        Ok(timeline.filter_by_timerange(start_ms, end_ms))
    }

    /// Mine log templates using offline Drain prefix tree (sub-millisecond when cached)
    pub async fn mine_templates(
        &self,
        _max_records: usize,
    ) -> anyhow::Result<Vec<LogTemplateCluster>> {
        self.ensure_analyzed().await?;
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let clusters = state
            .cached_template_clusters
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Template clusters were not cached"))?;

        Ok((**clusters).clone())
    }

    /// Detect volume spikes and error anomalies
    pub async fn detect_anomalies(
        &self,
        target_buckets: usize,
        z_threshold: f64,
    ) -> anyhow::Result<Vec<AnomalySpike>> {
        let hist = self.get_timeline_histogram(target_buckets).await?;
        let bucket_tuples: Vec<(i64, i64, usize, usize)> = hist
            .buckets
            .iter()
            .map(|b| {
                (
                    b.start_epoch_ms,
                    b.end_epoch_ms,
                    b.total_count,
                    b.error_count + b.fatal_count,
                )
            })
            .collect();

        Ok(AnomalyDetector::detect_spikes(&bucket_tuples, z_threshold))
    }

    /// Group error records by template clusters (sub-millisecond when cached)
    pub async fn cluster_errors(&self, _max_records: usize) -> anyhow::Result<Vec<ErrorCluster>> {
        self.ensure_analyzed().await?;
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let clusters = state
            .cached_error_clusters
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Error clusters were not cached"))?;

        Ok((**clusters).clone())
    }

    /// Correlate container failure chains across time window
    pub async fn correlate_failures(
        &self,
        window_ms: i64,
    ) -> anyhow::Result<Vec<CorrelatedFailureChain>> {
        let records = self.parse_records(10000).await?;
        Ok(KubernetesImporter::correlate_failures(&records, window_ms))
    }

    /// Redact active log records and export to destination file (Business Tier)
    pub async fn export_redacted(
        &self,
        dest_path: &Path,
        gate: &shared_licensing::TierGate,
    ) -> anyhow::Result<usize> {
        self.ensure_analyzed().await?;
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let records = state
            .cached_records
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Records not cached"))?;

        let engine = crate::redaction::RedactionEngine::new();
        let mut file = std::fs::File::create(dest_path)?;
        let count = engine.export_redacted_stream(records, &mut file, gate)?;
        Ok(count)
    }

    /// Execute a registered plugin
    pub async fn execute_plugin(&self, plugin_id: &str, input: &str) -> anyhow::Result<String> {
        self.plugin_manager
            .execute(plugin_id, input)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// List active plugins
    pub fn list_plugins(&self) -> Vec<PluginManifest> {
        self.plugin_manager.list_plugins()
    }

    /// Execute a search query across the active log file.
    pub async fn search(&self, query: LogQuery) -> anyhow::Result<LogSearchReport> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let report = LogSearchEngine::search(&state.engine, &state.index, &query)?;

        self.telemetry.record_counter("search_queries_executed", 1);
        self.telemetry
            .record_duration_ms("search_latency_ms", report.elapsed_micros as f64 / 1000.0);

        Ok(report)
    }

    /// Toggle live tail mode.
    pub async fn toggle_tail_mode(&self, enable: bool) -> anyhow::Result<bool> {
        let mut active_lock = self.active_file.write().await;
        let state = active_lock
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        if enable {
            if state.tail.is_none() {
                let tail = TailFollower::start(&state.path)?;
                state.tail = Some(tail);
            }
            Ok(true)
        } else {
            state.tail = None;
            Ok(false)
        }
    }

    /// Add a line bookmark.
    pub async fn add_bookmark(
        &self,
        line_number: usize,
        note: String,
    ) -> anyhow::Result<BookmarkRecord> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let record = BookmarkRecord {
            id: format!(
                "bm_{}_{}",
                line_number,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            workspace_id: "default".to_string(),
            file_path: state.path.to_string_lossy().to_string(),
            line_number,
            label: note,
        };
        self.workspace_store.add_bookmark(&record)?;
        Ok(record)
    }

    /// List all bookmarks for the currently opened file.
    pub async fn list_bookmarks(&self) -> anyhow::Result<Vec<BookmarkRecord>> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let list = self
            .workspace_store
            .list_bookmarks(&state.path.to_string_lossy())?;
        Ok(list)
    }

    /// Query current system metrics (RSS RAM and CPU usage).
    pub fn get_system_metrics(&self) -> ProcessResourceUsage {
        ProcessResourceUsage::sample()
    }
}

static DEFAULT_LOG_SERVICE: std::sync::OnceLock<(Arc<LogViewerService>, PathBuf)> =
    std::sync::OnceLock::new();

fn ensure_sample_log_file(dir: &Path) -> PathBuf {
    let sample_path = dir.join("production_cluster_sample.log");
    if !sample_path.exists() {
        let services = [
            "auth-service",
            "payment-gateway",
            "ingress-controller",
            "database-pool",
            "worker-queue",
        ];
        let levels = ["INFO", "INFO", "INFO", "WARN", "ERROR", "DEBUG"];
        let messages = [
            "Connection established from 192.168.1.104:443",
            "Transaction committed successfully in 14.2ms",
            "High heap memory threshold reached (84.2%)",
            "Connection pool timeout acquiring PostgreSQL handle",
            "JWT token validation failed for user_id=98421",
            "Cache miss for key: session_token_88ab",
            "Worker job processed successfully: job_id=89231",
        ];
        let mut buf = String::with_capacity(2000 * 96);
        for i in 0..2000usize {
            let sec = i % 60;
            let min = (i / 60) % 60;
            let lvl = if (500..540).contains(&i) {
                "ERROR"
            } else {
                levels[i % levels.len()]
            };
            let svc = services[i % services.len()];
            let msg = messages[i % messages.len()];
            buf.push_str(&format!(
                "2026-10-19T10:{:02}:{:02}Z [{}] [{}] #{}: {}\n",
                min,
                sec,
                lvl,
                svc,
                i + 1,
                msg
            ));
        }
        let _ = std::fs::write(&sample_path, buf.as_bytes());
    }
    sample_path
}

pub fn get_default_log_service() -> Result<(Arc<LogViewerService>, PathBuf), String> {
    if let Some((svc, sample_path)) = DEFAULT_LOG_SERVICE.get() {
        return Ok((Arc::clone(svc), sample_path.clone()));
    }
    let (ws_store, sample_path) =
        if let Ok(paths) = platform_common::AppPaths::from_app_name("universal-log-viewer") {
            let _ = paths.ensure_directories();
            let db_path = paths.data_dir.join("workspaces.db");
            let ws = WorkspaceStore::open(&db_path)
                .or_else(|_| WorkspaceStore::open_in_memory())
                .map_err(|e| e.to_string())?;
            let sp = ensure_sample_log_file(&paths.cache_dir);
            (Arc::new(ws), sp)
        } else {
            let ws = WorkspaceStore::open_in_memory().map_err(|e| e.to_string())?;
            let sp = ensure_sample_log_file(&std::env::temp_dir());
            (Arc::new(ws), sp)
        };

    let svc = Arc::new(LogViewerService::new(ws_store, LocalTelemetry::new(true)));
    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        let _ = rt.block_on(svc.open_file(&sample_path));
    }
    let _ = DEFAULT_LOG_SERVICE.set((Arc::clone(&svc), sample_path.clone()));
    Ok((svc, sample_path))
}

/// Dispatch desktop IPC commands from the Svelte 5 Universal Log Viewer frontend
pub fn dispatch_ipc_command(
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (svc, sample_path) = get_default_log_service()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async move {
        match cmd {
            "open_file" => {
                let raw_path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let target_path = if raw_path.is_empty()
                    || raw_path == "default"
                    || !Path::new(raw_path).exists()
                {
                    sample_path
                } else {
                    PathBuf::from(raw_path)
                };
                let info = svc
                    .open_file(&target_path)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(info).map_err(|e| e.to_string())
            }
            "get_detected_format" => {
                let fmt = svc.get_detected_format().await.map_err(|e| e.to_string())?;
                serde_json::to_value(fmt).map_err(|e| e.to_string())
            }
            "get_lines_slice" => {
                let start_line = args
                    .get("startLine")
                    .or_else(|| args.get("start_line"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(60) as usize;
                let slice = svc
                    .get_lines_slice(start_line, count)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(slice).map_err(|e| e.to_string())
            }
            "get_lines_by_indices" => {
                let line_numbers = args
                    .get("lineNumbers")
                    .or_else(|| args.get("line_numbers"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_u64().map(|n| n as usize))
                            .collect::<Vec<usize>>()
                    })
                    .unwrap_or_default();
                let lines = svc
                    .get_lines_by_indices(&line_numbers)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(lines).map_err(|e| e.to_string())
            }
            "parse_records" => {
                let max_records = args
                    .get("maxRecords")
                    .or_else(|| args.get("max_records"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000) as usize;
                let recs = svc
                    .parse_records(max_records)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(recs).map_err(|e| e.to_string())
            }
            "get_timeline_histogram" => {
                let target_buckets = args
                    .get("targetBuckets")
                    .or_else(|| args.get("target_buckets"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(60) as usize;
                let hist = svc
                    .get_timeline_histogram(target_buckets)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(hist).map_err(|e| e.to_string())
            }
            "filter_by_timerange" => {
                let start_ms = args
                    .get("startMs")
                    .or_else(|| args.get("start_ms"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let end_ms = args
                    .get("endMs")
                    .or_else(|| args.get("end_ms"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(i64::MAX);
                let lines = svc
                    .filter_by_timerange(start_ms, end_ms)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(lines).map_err(|e| e.to_string())
            }
            "mine_templates" => {
                let max_records = args
                    .get("maxRecords")
                    .or_else(|| args.get("max_records"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50000) as usize;
                let clusters = svc
                    .mine_templates(max_records)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(clusters).map_err(|e| e.to_string())
            }
            "detect_anomalies" => {
                let target_buckets = args
                    .get("targetBuckets")
                    .or_else(|| args.get("target_buckets"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(60) as usize;
                let z_threshold = args
                    .get("zThreshold")
                    .or_else(|| args.get("z_threshold"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.0);
                let spikes = svc
                    .detect_anomalies(target_buckets, z_threshold)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(spikes).map_err(|e| e.to_string())
            }
            "cluster_errors" => {
                let max_records = args
                    .get("maxRecords")
                    .or_else(|| args.get("max_records"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50000) as usize;
                let clusters = svc
                    .cluster_errors(max_records)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(clusters).map_err(|e| e.to_string())
            }
            "correlate_failures" => {
                let window_ms = args
                    .get("windowMs")
                    .or_else(|| args.get("window_ms"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(5000);
                let chains = svc
                    .correlate_failures(window_ms)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(chains).map_err(|e| e.to_string())
            }
            "search" => {
                let q_val = args.get("query").cloned().unwrap_or(args);
                let query: LogQuery = serde_json::from_value(q_val).map_err(|e| e.to_string())?;
                let report = svc.search(query).await.map_err(|e| e.to_string())?;
                serde_json::to_value(report).map_err(|e| e.to_string())
            }
            "toggle_tail_mode" => {
                let enable = args
                    .get("enable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let active = svc
                    .toggle_tail_mode(enable)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!(active))
            }
            "add_bookmark" => {
                let line_number = args
                    .get("lineNumber")
                    .or_else(|| args.get("line_number"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let note = args
                    .get("note")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Bookmark")
                    .to_string();
                let bm = svc
                    .add_bookmark(line_number, note)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(bm).map_err(|e| e.to_string())
            }
            "list_bookmarks" => {
                let list = svc.list_bookmarks().await.map_err(|e| e.to_string())?;
                serde_json::to_value(list).map_err(|e| e.to_string())
            }
            "list_plugins" => {
                let list = svc.list_plugins();
                serde_json::to_value(list).map_err(|e| e.to_string())
            }
            "get_system_metrics" => {
                let metrics = svc.get_system_metrics();
                serde_json::to_value(metrics).map_err(|e| e.to_string())
            }
            "export_redacted" => {
                let dest_path_str = args
                    .get("destPath")
                    .or_else(|| args.get("dest_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("redacted_export.log");
                let gate =
                    shared_licensing::TierGate::for_tier(shared_licensing::LicenseTier::Business);
                let count = svc
                    .export_redacted(std::path::Path::new(dest_path_str), &gate)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "dest_path": dest_path_str,
                    "records_redacted": count,
                }))
            }
            _ => Err(format!("Unknown Universal Log Viewer IPC command: {}", cmd)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_commands_service_workflow() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("commands_test.log");
        let content =
            b"2026-10-19T10:00:00Z [INFO] Init\n2026-10-19T10:00:01Z [WARN] Disk 85%\n2026-10-19T10:00:02Z [ERROR] Crash\n";
        std::fs::write(&log_path, content).unwrap();

        let ws_store = Arc::new(WorkspaceStore::open_in_memory().unwrap());
        let tel = LocalTelemetry::new(true);
        let service = LogViewerService::new(ws_store, tel);

        let info = service.open_file(&log_path).await.unwrap();
        assert_eq!(info.total_lines, 3);
        assert_eq!(info.file_size_bytes, content.len() as u64);

        // Get lines slice (0..2)
        let slice = service.get_lines_slice(0, 2).await.unwrap();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].text, "2026-10-19T10:00:00Z [INFO] Init");

        // Get lines by indices [0, 2]
        let by_indices = service.get_lines_by_indices(&[0, 2]).await.unwrap();
        assert_eq!(by_indices.len(), 2);
        assert_eq!(by_indices[0].line_number, 0);
        assert_eq!(by_indices[1].line_number, 2);
        assert_eq!(by_indices[1].text, "2026-10-19T10:00:02Z [ERROR] Crash");

        // Parse records
        let records = service.parse_records(10).await.unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].severity, LogSeverity::Info);
        assert_eq!(records[2].severity, LogSeverity::Error);

        // Timeline Histogram
        let hist = service.get_timeline_histogram(2).await.unwrap();
        assert_eq!(hist.total_records, 3);

        // Template mining
        let templates = service.mine_templates(10).await.unwrap();
        assert!(!templates.is_empty());

        // Error clustering
        let clusters = service.cluster_errors(10).await.unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].error_count, 1);

        // Plugins
        let plugins = service.list_plugins();
        assert_eq!(plugins.len(), 2);

        // Metrics
        let metrics = service.get_system_metrics();
        assert!(metrics.resident_set_size_bytes > 0);

        // Export redacted
        let export_path = temp.path().join("redacted_out.log");
        let gate = shared_licensing::TierGate::for_tier(shared_licensing::LicenseTier::Business);
        let redacted_count = service.export_redacted(&export_path, &gate).await.unwrap();
        assert_eq!(redacted_count, 3);
        assert!(export_path.exists());
    }
}
