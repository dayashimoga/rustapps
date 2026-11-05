#![forbid(unsafe_code)]
//! High-Level Analytical Service for Parquet, SQLite, and Delimited Data

use crate::data_engine::{DataEngine, DataEngineError};
use crate::models::{
    ChartRequest, ChartResult, ColumnStatistics, DatasetFormat, QueryPageResult, QueryRequest,
    SchemaSnapshot, WorkbenchStats,
};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Data engine failure: {0}")]
    Engine(#[from] DataEngineError),
    #[error("Dataset not found: {0}")]
    DatasetNotFound(String),
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),
}

/// Orchestrates dataset loading, caching, queries, statistics, and charting
pub struct AnalyticsService {
    engine: Arc<DataEngine>,
}

impl AnalyticsService {
    /// Creates a new analytics service instance with in-memory analytical cache
    pub fn new() -> Result<Self, ServiceError> {
        let engine = Arc::new(DataEngine::open_in_memory()?);
        Ok(Self { engine })
    }

    /// Loads a dataset from local file path by format
    pub fn load_dataset(
        &self,
        dataset_id: &str,
        name: &str,
        path: impl AsRef<Path>,
    ) -> Result<SchemaSnapshot, ServiceError> {
        let p = path.as_ref();
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("csv")
            .to_ascii_lowercase();

        let format = DatasetFormat::from_extension(&ext);
        let schema = match format {
            DatasetFormat::Csv => self
                .engine
                .ingest_delimited_file(dataset_id, name, p, b',')?,
            DatasetFormat::Tsv => self
                .engine
                .ingest_delimited_file(dataset_id, name, p, b'\t')?,
            DatasetFormat::Sqlite => self
                .engine
                .attach_sqlite_database(dataset_id, name, p, None)?,
            DatasetFormat::Parquet => self.engine.ingest_parquet_file(dataset_id, name, p)?,
            DatasetFormat::JsonLines => {
                // For JSONL, fallback to delimited parser or treat as tabular lines
                self.engine
                    .ingest_delimited_file(dataset_id, name, p, b',')?
            }
        };

        Ok(schema)
    }

    /// Seeds an initial demonstration dataset for instant verification
    pub fn seed_demo_dataset(&self) -> Result<SchemaSnapshot, ServiceError> {
        let temp_dir = std::env::temp_dir();
        let sample_csv_path = temp_dir.join("global_server_telemetry.csv");

        let mut sample_content =
            String::from("server_id,datacenter,cpu_percent,memory_mb,error_count,status\n");
        let dcs = ["us-east-1", "us-west-2", "eu-central-1", "ap-northeast-1"];
        for i in 1..=100 {
            let dc = dcs[i % dcs.len()];
            let cpu = 10.5 + ((i * 7) % 85) as f64;
            let mem = 2048 + (i * 128);
            let errs = if i % 13 == 0 { 2 } else { 0 };
            let status = if errs > 0 { "WARNING" } else { "HEALTHY" };
            sample_content.push_str(&format!(
                "srv-{:03},{},{:.1},{},{},{}\n",
                i, dc, cpu, mem, errs, status
            ));
        }

        std::fs::write(&sample_csv_path, sample_content)?;
        self.load_dataset("demo-telemetry", "Global Server Telemetry", sample_csv_path)
    }

    /// Lists all currently active loaded schemas
    pub fn list_datasets(&self) -> Vec<SchemaSnapshot> {
        self.engine.list_schemas()
    }

    /// Retrieves schema by dataset ID
    pub fn get_schema(&self, dataset_id: &str) -> Result<SchemaSnapshot, ServiceError> {
        self.engine
            .get_schema(dataset_id)
            .ok_or_else(|| ServiceError::DatasetNotFound(dataset_id.to_string()))
    }

    /// Executes filtered, sorted, paginated table query
    pub fn query_dataset(&self, req: &QueryRequest) -> Result<QueryPageResult, ServiceError> {
        Ok(self.engine.query_dataset(req)?)
    }

    /// Computes aggregated chart data
    pub fn compute_chart_data(&self, req: &ChartRequest) -> Result<ChartResult, ServiceError> {
        Ok(self.engine.compute_chart_data(req)?)
    }

    /// Computes column statistics
    pub fn compute_column_stats(
        &self,
        dataset_id: &str,
        column_name: &str,
    ) -> Result<ColumnStatistics, ServiceError> {
        let schema = self.get_schema(dataset_id)?;
        let col = schema
            .columns
            .iter()
            .find(|c| c.name == column_name)
            .ok_or_else(|| {
                ServiceError::DatasetNotFound(format!("Column {} not found", column_name))
            })?;

        // Query column values to calculate stats
        let q = QueryRequest {
            dataset_id: dataset_id.to_string(),
            filters: Vec::new(),
            sort: None,
            search_query: None,
            limit: 5000,
            offset: 0,
        };
        let page = self.query_dataset(&q)?;
        let col_idx = page
            .columns
            .iter()
            .position(|c| c == column_name)
            .unwrap_or(0);

        let mut total = 0usize;
        let mut null_count = 0usize;
        let mut distinct_set = std::collections::HashSet::new();
        let mut min_val: Option<String> = None;
        let mut max_val: Option<String> = None;
        let mut num_sum = 0.0f64;
        let mut num_count = 0usize;

        for row in &page.rows {
            if let Some(val) = row.get(col_idx) {
                total += 1;
                if val.is_empty() {
                    null_count += 1;
                } else {
                    distinct_set.insert(val.clone());
                    if min_val.as_ref().map_or(true, |m| val < m) {
                        min_val = Some(val.clone());
                    }
                    if max_val.as_ref().map_or(true, |m| val > m) {
                        max_val = Some(val.clone());
                    }
                    if col.data_type.is_numeric() {
                        if let Ok(n) = val.parse::<f64>() {
                            num_sum += n;
                            num_count += 1;
                        }
                    }
                }
            }
        }

        let mean = if num_count > 0 {
            Some(num_sum / num_count as f64)
        } else {
            None
        };

        Ok(ColumnStatistics {
            column: column_name.to_string(),
            data_type: col.data_type,
            total_count: total,
            null_count,
            distinct_count: distinct_set.len(),
            min_value: min_val,
            max_value: max_val,
            mean_numeric: mean,
        })
    }

    /// Computes overall operational workbench statistics
    pub fn get_workbench_stats(&self) -> WorkbenchStats {
        let schemas = self.list_datasets();
        let total_datasets = schemas.len();
        let total_cached_rows = schemas.iter().map(|s| s.total_rows).sum();
        let total_bytes_analyzed = schemas.iter().map(|s| s.total_bytes).sum();

        WorkbenchStats {
            total_datasets,
            total_cached_rows,
            total_bytes_analyzed,
        }
    }
}
