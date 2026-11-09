#![deny(unsafe_code)]
//! Air-Gapped API & Protocol Studio Core Library
//!
//! Provides zero-network-egress, 100% offline REST, GraphQL, gRPC, WebSocket, and MQTT
//! request composition, environment variable interpolation, and automated security fuzzing.

pub mod collection_store;
pub mod commands;
#[allow(unsafe_code)]
pub mod ffi;
pub mod fuzzer;
pub mod http_client;
pub mod importers;
pub mod models;
pub mod service;

pub use collection_store::{CollectionStore, StoreError};
pub use commands::dispatch_ipc_command;
pub use fuzzer::ApiFuzzer;
pub use http_client::{AirGappedHttpClient, HttpError};
pub use importers::{CurlParser, OpenApiCodec, PostmanCodec};
pub use models::{
    ApiCollection, ApiEnvironment, ApiRequestItem, ApiResponseSnapshot, AuthConfig,
    CollectionImportSummary, FuzzingResult, FuzzingStrategy, HttpMethod, KeyValuePair,
    OwaspReportFinding, OwaspSecurityReport, ProtocolType, RequestBody, VariableInterpolationToken,
    VariableResolutionSummary,
};
pub use service::{ApiStudioService, ServiceError, StudioStats};
