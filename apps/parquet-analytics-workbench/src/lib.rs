#![deny(unsafe_code)]
//! Local Database & Parquet Analytics Workbench Library

pub mod commands;
pub mod data_engine;
pub mod ffi;
pub mod models;
pub mod service;

pub use commands::dispatch_ipc_command;
pub use data_engine::{DataEngine, DataEngineError};
pub use models::{
    AggregationType, ChartDataPoint, ChartRequest, ChartResult, ChartType, ColumnMetadata,
    ColumnStatistics, DataFilter, DataType, DatasetFormat, FilterOperator, QueryPageResult,
    QueryRequest, SchemaSnapshot, SortClause, SortDirection, WorkbenchStats,
};
pub use service::{AnalyticsService, ServiceError};
