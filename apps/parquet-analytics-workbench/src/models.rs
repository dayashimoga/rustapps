#![forbid(unsafe_code)]
//! Core Domain Models for Local Database & Parquet Analytics Workbench

use serde::{Deserialize, Serialize};

/// Supported offline data formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DatasetFormat {
    #[default]
    Csv,
    Tsv,
    Parquet,
    Sqlite,
    JsonLines,
}

impl DatasetFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatasetFormat::Csv => "csv",
            DatasetFormat::Tsv => "tsv",
            DatasetFormat::Parquet => "parquet",
            DatasetFormat::Sqlite => "sqlite",
            DatasetFormat::JsonLines => "jsonl",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "tsv" => DatasetFormat::Tsv,
            "parquet" | "pq" => DatasetFormat::Parquet,
            "db" | "sqlite" | "sqlite3" => DatasetFormat::Sqlite,
            "jsonl" | "ndjson" => DatasetFormat::JsonLines,
            _ => DatasetFormat::Csv,
        }
    }
}

/// Column scalar data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DataType {
    #[default]
    Utf8,
    Int64,
    Float64,
    Boolean,
    DateTime,
    Null,
}

impl DataType {
    pub fn is_numeric(&self) -> bool {
        matches!(self, DataType::Int64 | DataType::Float64)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::Utf8 => "Utf8",
            DataType::Int64 => "Int64",
            DataType::Float64 => "Float64",
            DataType::Boolean => "Boolean",
            DataType::DateTime => "DateTime",
            DataType::Null => "Null",
        }
    }
}

/// Metadata description of a single column in a dataset
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: DataType,
    pub null_count: usize,
    pub is_numeric: bool,
}

/// Dataset schema snapshot with row count and byte metrics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaSnapshot {
    pub dataset_id: String,
    pub name: String,
    pub file_path: String,
    pub format: DatasetFormat,
    pub columns: Vec<ColumnMetadata>,
    pub total_rows: usize,
    pub total_bytes: u64,
    pub last_scanned: i64,
}

/// Relational filter operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FilterOperator {
    #[default]
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    StartsWith,
    EndsWith,
    IsNull,
    IsNotNull,
}

/// Column filter predicate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFilter {
    pub column: String,
    pub operator: FilterOperator,
    pub value: String,
}

/// Sorting direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

/// Sorting clause
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortClause {
    pub column: String,
    pub direction: SortDirection,
}

/// Request to query rows from a dataset
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub dataset_id: String,
    pub filters: Vec<DataFilter>,
    pub sort: Option<SortClause>,
    pub search_query: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

/// Paginated query result for virtualized table viewports
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPageResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_matching_rows: usize,
    pub execution_time_ms: u64,
}

/// Statistical summary for numeric and categorical columns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnStatistics {
    pub column: String,
    pub data_type: DataType,
    pub total_count: usize,
    pub null_count: usize,
    pub distinct_count: usize,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub mean_numeric: Option<f64>,
}

/// Chart visualization type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ChartType {
    #[default]
    Bar,
    Line,
    Scatter,
    Histogram,
}

/// Chart data aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AggregationType {
    #[default]
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Request to compute aggregated chart data points
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartRequest {
    pub dataset_id: String,
    pub chart_type: ChartType,
    pub x_column: String,
    pub y_column: Option<String>,
    pub aggregation: AggregationType,
    pub limit: usize,
}

/// Single aggregated data point for charts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartDataPoint {
    pub label: String,
    pub value: f64,
}

/// Computed chart data package
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartResult {
    pub points: Vec<ChartDataPoint>,
    pub x_label: String,
    pub y_label: String,
    pub computed_in_ms: u64,
}

/// Global workbench operational metrics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkbenchStats {
    pub total_datasets: usize,
    pub total_cached_rows: usize,
    pub total_bytes_analyzed: u64,
}
