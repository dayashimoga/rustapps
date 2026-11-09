#![forbid(unsafe_code)]
//! Core Domain Models for Air-Gapped API & Protocol Studio

use serde::{Deserialize, Serialize};

/// Target network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProtocolType {
    #[default]
    HttpRest,
    GraphQl,
    Grpc,
    WebSocket,
    Mqtt,
}

impl ProtocolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolType::HttpRest => "http_rest",
            ProtocolType::GraphQl => "graphql",
            ProtocolType::Grpc => "grpc",
            ProtocolType::WebSocket => "websocket",
            ProtocolType::Mqtt => "mqtt",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "graphql" => ProtocolType::GraphQl,
            "grpc" => ProtocolType::Grpc,
            "websocket" | "ws" => ProtocolType::WebSocket,
            "mqtt" => ProtocolType::Mqtt,
            _ => ProtocolType::HttpRest,
        }
    }
}

/// HTTP request method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            _ => HttpMethod::GET,
        }
    }
}

/// Key-Value pair for headers, params, and variables
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub description: Option<String>,
}

impl KeyValuePair {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: None,
        }
    }
}

/// Authentication scheme for API requests
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "config")]
pub enum AuthConfig {
    #[default]
    None,
    BearerToken {
        token: String,
    },
    BasicAuth {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        in_header: bool,
    },
    VaultSecretRef {
        secret_id: String,
    },
}

/// Request payload body specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequestBody {
    /// "none", "json", "raw", "graphql"
    pub body_type: String,
    pub raw_content: String,
    pub graphql_query: Option<String>,
    pub graphql_variables: Option<String>,
}

/// An individual API endpoint request in a collection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiRequestItem {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub protocol: ProtocolType,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValuePair>,
    pub query_params: Vec<KeyValuePair>,
    pub auth: AuthConfig,
    pub body: RequestBody,
    pub timeout_ms: u64,
}

impl ApiRequestItem {
    pub fn new(
        id: impl Into<String>,
        collection_id: impl Into<String>,
        name: impl Into<String>,
        method: HttpMethod,
        url: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            collection_id: collection_id.into(),
            name: name.into(),
            protocol: ProtocolType::HttpRest,
            method,
            url: url.into(),
            headers: Vec::new(),
            query_params: Vec::new(),
            auth: AuthConfig::None,
            body: RequestBody {
                body_type: "none".to_string(),
                raw_content: String::new(),
                graphql_query: None,
                graphql_variables: None,
            },
            timeout_ms: 10_000,
        }
    }
}

/// Snapshot of an executed network response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponseSnapshot {
    pub request_id: String,
    pub status_code: u16,
    pub status_text: String,
    pub latency_ms: u64,
    pub size_bytes: usize,
    pub headers: Vec<KeyValuePair>,
    pub body: String,
    pub timestamp: i64,
}

/// A logical collection grouping endpoints with shared environment variables
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiCollection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub variables: Vec<KeyValuePair>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Security fuzzing payload strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FuzzingStrategy {
    #[default]
    SqlInjection,
    CommandInjection,
    BufferOverflow,
    SpecialCharacters,
    BoundaryValues,
}

/// Result of an automated fuzzing permutation test
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzingResult {
    pub payload: String,
    pub target_param: String,
    pub status_code: u16,
    pub latency_ms: u64,
    pub anomaly_detected: bool,
    pub notes: String,
}

/// A named execution environment (e.g., Local, Staging, Air-Gap Lab)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiEnvironment {
    pub id: String,
    pub name: String,
    pub variables: Vec<KeyValuePair>,
    pub is_active: bool,
}

impl ApiEnvironment {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            variables: Vec::new(),
            is_active: false,
        }
    }
}

/// Variable token details for live hover preview & syntax highlighting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableInterpolationToken {
    pub key: String,
    pub value: String,
    pub found: bool,
    pub placeholder: String,
    pub start_idx: usize,
    pub end_idx: usize,
}

/// Variable resolution summary with hover tokens
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableResolutionSummary {
    pub original_text: String,
    pub resolved_text: String,
    pub tokens: Vec<VariableInterpolationToken>,
    pub unresolved_count: usize,
}

/// Finding in an OWASP API Security report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwaspReportFinding {
    pub owasp_id: String,
    pub title: String,
    pub severity: String,
    pub strategy: String,
    pub target_param: String,
    pub payload: String,
    pub evidence: String,
    pub remediation: String,
}

/// Comprehensive OWASP API Security Audit Report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwaspSecurityReport {
    pub report_id: String,
    pub generated_at: i64,
    pub target_url: String,
    pub method: String,
    pub total_tests: usize,
    pub anomalies_detected: usize,
    pub overall_risk_score: String,
    pub findings: Vec<OwaspReportFinding>,
    pub markdown_summary: String,
}

/// Summary of an imported collection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionImportSummary {
    pub collection_id: String,
    pub collection_name: String,
    pub requests_count: usize,
    pub variables_count: usize,
    pub format: String,
}
