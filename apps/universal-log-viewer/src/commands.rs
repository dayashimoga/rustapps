#![forbid(unsafe_code)]
//! Tauri IPC Command Interface and Service Boundary
//!
//! Provides high-level command handlers for the desktop frontend, translating
//! domain operations with `anyhow` boundaries and Serde data transfer models.

use crate::mmap_engine::MmapLogEngine;
use crate::search_engine::{LogQuery, LogSearchEngine, LogSearchReport};
use crate::sparse_index::SparseLineIndex;
use crate::tail_mode::TailFollower;
use crate::workspace::{BookmarkRecord, WorkspaceStore};
use platform_common::ProcessResourceUsage;
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
}

/// Shared application state container
pub struct LogViewerService {
    active_file: Arc<RwLock<Option<ActiveFileState>>>,
    workspace_store: Arc<WorkspaceStore>,
    telemetry: LocalTelemetry,
}

impl LogViewerService {
    /// Create a new service instance.
    pub fn new(workspace_store: Arc<WorkspaceStore>, telemetry: LocalTelemetry) -> Self {
        Self {
            active_file: Arc::new(RwLock::new(None)),
            workspace_store,
            telemetry,
        }
    }

    /// Open a log file, map it into virtual memory, and build its sparse index.
    pub async fn open_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<OpenedFileInfo> {
        let path_ref = path.as_ref();
        let engine = MmapLogEngine::open(path_ref)?;
        let index = SparseLineIndex::build(engine.as_bytes());

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
            });
        }

        self.telemetry.record_counter("log_files_opened", 1);

        Ok(file_info)
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
            if let Some(tail) = state.tail.take() {
                tail.stop();
            }
            Ok(false)
        }
    }

    /// Add a bookmark on the currently opened file.
    pub async fn add_bookmark(
        &self,
        line_number: usize,
        label: String,
    ) -> anyhow::Result<BookmarkRecord> {
        let active_lock = self.active_file.read().await;
        let state = active_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active log file opened"))?;

        let bm = BookmarkRecord {
            id: format!(
                "bm_{}_{}",
                line_number,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis()
            ),
            workspace_id: "default_workspace".to_string(),
            file_path: state.path.to_string_lossy().to_string(),
            line_number,
            label,
        };

        self.workspace_store.add_bookmark(&bm)?;
        Ok(bm)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_commands_service_workflow() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("commands_test.log");
        let content =
            b"2026-10-19 [INFO] Init\n2026-10-19 [WARN] Disk 85%\n2026-10-19 [ERROR] Crash\n";
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
        assert_eq!(slice[0].text, "2026-10-19 [INFO] Init");
        assert_eq!(slice[1].text, "2026-10-19 [WARN] Disk 85%");

        // Search query
        let query = LogQuery {
            pattern: "ERROR".to_string(),
            is_regex: false,
            case_sensitive: true,
            limit: 5,
            start_line: 0,
        };
        let report = service.search(query).await.unwrap();
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].line_number, 2);

        // Bookmarks
        let bm = service
            .add_bookmark(2, "Investigate crash".to_string())
            .await
            .unwrap();
        assert_eq!(bm.line_number, 2);

        let bookmarks = service.list_bookmarks().await.unwrap();
        assert_eq!(bookmarks.len(), 1);

        // Metrics
        let metrics = service.get_system_metrics();
        assert!(metrics.resident_set_size_bytes > 0);
    }
}
