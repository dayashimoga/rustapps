#![forbid(unsafe_code)]
//! API & Protocol Studio Service Orchestrator

use crate::collection_store::{CollectionStore, StoreError};
use crate::fuzzer::ApiFuzzer;
use crate::http_client::{AirGappedHttpClient, HttpError};
use crate::importers::{CurlParser, OpenApiCodec, PostmanCodec};
use crate::models::{
    ApiCollection, ApiEnvironment, ApiRequestItem, ApiResponseSnapshot, CollectionImportSummary,
    FuzzingResult, FuzzingStrategy, KeyValuePair, OwaspSecurityReport, VariableResolutionSummary,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),
    #[error("Request not found: {0}")]
    RequestNotFound(String),
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),
    #[error("Import error: {0}")]
    ImportError(String),
    #[error("Export error: {0}")]
    ExportError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StudioStats {
    pub total_collections: usize,
    pub total_requests: usize,
    pub active_protocols: Vec<String>,
}

pub struct ApiStudioService {
    store: Arc<CollectionStore>,
}

impl ApiStudioService {
    pub fn open_in_memory() -> Result<Self, ServiceError> {
        let store = Arc::new(CollectionStore::open_in_memory()?);
        Ok(Self { store })
    }

    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, ServiceError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| {
            StoreError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;
        let db_path = dir.join("protocol_studio.db");
        let store = Arc::new(CollectionStore::open(db_path)?);
        Ok(Self { store })
    }

    pub fn save_collection(&self, collection: &ApiCollection) -> Result<(), ServiceError> {
        Ok(self.store.save_collection(collection)?)
    }

    pub fn get_collection(&self, id: &str) -> Result<Option<ApiCollection>, ServiceError> {
        Ok(self.store.get_collection(id)?)
    }

    pub fn list_collections(&self) -> Result<Vec<ApiCollection>, ServiceError> {
        Ok(self.store.list_collections()?)
    }

    pub fn delete_collection(&self, id: &str) -> Result<bool, ServiceError> {
        Ok(self.store.delete_collection(id)?)
    }

    pub fn save_request(&self, request: &ApiRequestItem) -> Result<(), ServiceError> {
        Ok(self.store.save_request(request)?)
    }

    pub fn get_request(&self, id: &str) -> Result<Option<ApiRequestItem>, ServiceError> {
        Ok(self.store.get_request(id)?)
    }

    pub fn list_requests_in_collection(
        &self,
        collection_id: &str,
    ) -> Result<Vec<ApiRequestItem>, ServiceError> {
        Ok(self.store.list_requests_in_collection(collection_id)?)
    }

    pub fn delete_request(&self, id: &str) -> Result<bool, ServiceError> {
        Ok(self.store.delete_request(id)?)
    }

    /// Executes an API request with full variable interpolation and records response in history
    pub fn execute_request(
        &self,
        request_id: &str,
        override_vars: Option<Vec<KeyValuePair>>,
    ) -> Result<ApiResponseSnapshot, ServiceError> {
        let request = self
            .store
            .get_request(request_id)?
            .ok_or_else(|| ServiceError::RequestNotFound(request_id.to_string()))?;

        // Merge active environment variables, collection variables, and runtime overrides
        let mut vars = Vec::new();
        if let Ok(Some(env)) = self.store.get_active_environment() {
            vars.extend(env.variables);
        }
        if let Ok(Some(coll)) = self.store.get_collection(&request.collection_id) {
            vars.extend(coll.variables);
        }
        if let Some(overrides) = override_vars {
            vars.extend(overrides);
        }

        // Interpolate URL
        let resolved_url = CollectionStore::interpolate_variables(&request.url, &vars);

        // Interpolate headers
        let mut resolved_headers = Vec::new();
        for h in &request.headers {
            resolved_headers.push(KeyValuePair {
                key: CollectionStore::interpolate_variables(&h.key, &vars),
                value: CollectionStore::interpolate_variables(&h.value, &vars),
                enabled: h.enabled,
                description: h.description.clone(),
            });
        }

        let mut resolved_request = request.clone();
        resolved_request.headers = resolved_headers;

        // Execute request
        let snapshot = AirGappedHttpClient::execute(&resolved_request, &resolved_url)?;

        // Record in history
        let _ = self.store.record_response(&snapshot);

        Ok(snapshot)
    }

    /// Runs automated security fuzzing against an endpoint
    pub fn fuzz_request(
        &self,
        request_id: &str,
        strategy: FuzzingStrategy,
        param_name: &str,
    ) -> Result<Vec<FuzzingResult>, ServiceError> {
        let request = self
            .store
            .get_request(request_id)?
            .ok_or_else(|| ServiceError::RequestNotFound(request_id.to_string()))?;

        let mut vars = Vec::new();
        if let Ok(Some(env)) = self.store.get_active_environment() {
            vars.extend(env.variables);
        }
        if let Ok(Some(coll)) = self.store.get_collection(&request.collection_id) {
            vars.extend(coll.variables);
        }
        let resolved_url = CollectionStore::interpolate_variables(&request.url, &vars);

        Ok(ApiFuzzer::fuzz_endpoint(
            &request,
            &resolved_url,
            strategy,
            param_name,
        ))
    }

    /// Saves an environment configuration
    pub fn save_environment(&self, env: &ApiEnvironment) -> Result<(), ServiceError> {
        Ok(self.store.save_environment(env)?)
    }

    /// Fetches an environment by id
    pub fn get_environment(&self, id: &str) -> Result<Option<ApiEnvironment>, ServiceError> {
        Ok(self.store.get_environment(id)?)
    }

    /// Lists all saved environments
    pub fn list_environments(&self) -> Result<Vec<ApiEnvironment>, ServiceError> {
        Ok(self.store.list_environments()?)
    }

    /// Deletes an environment by id
    pub fn delete_environment(&self, id: &str) -> Result<bool, ServiceError> {
        Ok(self.store.delete_environment(id)?)
    }

    /// Sets the active environment
    pub fn set_active_environment(&self, id: &str) -> Result<bool, ServiceError> {
        Ok(self.store.set_active_environment(id)?)
    }

    /// Gets the currently active environment
    pub fn get_active_environment(&self) -> Result<Option<ApiEnvironment>, ServiceError> {
        Ok(self.store.get_active_environment()?)
    }

    /// Resolves variable tokens for live hover tooltips & previews
    pub fn resolve_variables(
        &self,
        text: &str,
        collection_id: Option<&str>,
        env_id: Option<&str>,
    ) -> Result<VariableResolutionSummary, ServiceError> {
        let mut vars = Vec::new();
        if let Some(eid) = env_id {
            if let Ok(Some(env)) = self.store.get_environment(eid) {
                vars.extend(env.variables);
            }
        } else if let Ok(Some(env)) = self.store.get_active_environment() {
            vars.extend(env.variables);
        }

        if let Some(cid) = collection_id {
            if let Ok(Some(coll)) = self.store.get_collection(cid) {
                vars.extend(coll.variables);
            }
        }

        Ok(CollectionStore::resolve_variable_tokens(text, &vars))
    }

    /// Imports a request from a cURL command string
    pub fn import_curl(
        &self,
        cmd: &str,
        collection_id: &str,
    ) -> Result<ApiRequestItem, ServiceError> {
        let req = CurlParser::parse(cmd, collection_id).map_err(ServiceError::ImportError)?;
        self.store.save_request(&req)?;
        Ok(req)
    }

    /// Imports an OpenAPI 3.1 JSON specification
    pub fn import_openapi(&self, json_str: &str) -> Result<CollectionImportSummary, ServiceError> {
        let (coll, requests) = OpenApiCodec::import(json_str).map_err(ServiceError::ImportError)?;
        let coll_id = coll.id.clone();
        let coll_name = coll.name.clone();
        let req_count = requests.len();
        let var_count = coll.variables.len();

        self.store.save_collection(&coll)?;
        for req in requests {
            self.store.save_request(&req)?;
        }

        Ok(CollectionImportSummary {
            collection_id: coll_id,
            collection_name: coll_name,
            requests_count: req_count,
            variables_count: var_count,
            format: "OpenAPI 3.1".to_string(),
        })
    }

    /// Exports a collection as OpenAPI 3.1 JSON specification
    pub fn export_openapi(&self, collection_id: &str) -> Result<serde_json::Value, ServiceError> {
        let coll = self
            .store
            .get_collection(collection_id)?
            .ok_or_else(|| ServiceError::CollectionNotFound(collection_id.to_string()))?;
        let requests = self.store.list_requests_in_collection(collection_id)?;
        Ok(OpenApiCodec::export(&coll, &requests))
    }

    /// Imports a Postman Collection v2.1 JSON
    pub fn import_postman(&self, json_str: &str) -> Result<CollectionImportSummary, ServiceError> {
        let (coll, requests) = PostmanCodec::import(json_str).map_err(ServiceError::ImportError)?;
        let coll_id = coll.id.clone();
        let coll_name = coll.name.clone();
        let req_count = requests.len();
        let var_count = coll.variables.len();

        self.store.save_collection(&coll)?;
        for req in requests {
            self.store.save_request(&req)?;
        }

        Ok(CollectionImportSummary {
            collection_id: coll_id,
            collection_name: coll_name,
            requests_count: req_count,
            variables_count: var_count,
            format: "Postman v2.1".to_string(),
        })
    }

    /// Exports a collection as Postman Collection v2.1 JSON
    pub fn export_postman(&self, collection_id: &str) -> Result<serde_json::Value, ServiceError> {
        let coll = self
            .store
            .get_collection(collection_id)?
            .ok_or_else(|| ServiceError::CollectionNotFound(collection_id.to_string()))?;
        let requests = self.store.list_requests_in_collection(collection_id)?;
        Ok(PostmanCodec::export(&coll, &requests))
    }

    /// Runs a comprehensive OWASP API Security Audit against an endpoint
    pub fn run_owasp_audit(
        &self,
        request_id: &str,
        param_name: &str,
    ) -> Result<OwaspSecurityReport, ServiceError> {
        let request = self
            .store
            .get_request(request_id)?
            .ok_or_else(|| ServiceError::RequestNotFound(request_id.to_string()))?;

        let mut vars = Vec::new();
        if let Ok(Some(env)) = self.store.get_active_environment() {
            vars.extend(env.variables);
        }
        if let Ok(Some(coll)) = self.store.get_collection(&request.collection_id) {
            vars.extend(coll.variables);
        }
        let resolved_url = CollectionStore::interpolate_variables(&request.url, &vars);

        let strategies = [
            FuzzingStrategy::SqlInjection,
            FuzzingStrategy::CommandInjection,
            FuzzingStrategy::BufferOverflow,
            FuzzingStrategy::SpecialCharacters,
            FuzzingStrategy::BoundaryValues,
        ];

        let mut all_results = Vec::new();
        for strat in strategies {
            let res = ApiFuzzer::fuzz_endpoint(&request, &resolved_url, strat, param_name);
            all_results.extend(res);
        }

        Ok(ApiFuzzer::generate_owasp_report(
            &request,
            &resolved_url,
            &all_results,
        ))
    }

    pub fn get_latest_response(
        &self,
        request_id: &str,
    ) -> Result<Option<ApiResponseSnapshot>, ServiceError> {
        Ok(self.store.get_latest_response(request_id)?)
    }

    pub fn get_stats(&self) -> Result<StudioStats, ServiceError> {
        let collections = self.store.list_collections()?;
        let total_collections = collections.len();
        let mut total_requests = 0;
        let mut protocols = std::collections::HashSet::new();

        for coll in &collections {
            if let Ok(reqs) = self.store.list_requests_in_collection(&coll.id) {
                total_requests += reqs.len();
                for r in reqs {
                    protocols.insert(r.protocol.as_str().to_string());
                }
            }
        }

        if protocols.is_empty() {
            protocols.insert("http_rest".to_string());
            protocols.insert("graphql".to_string());
        }

        Ok(StudioStats {
            total_collections,
            total_requests,
            active_protocols: protocols.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApiCollection, ApiRequestItem, HttpMethod};

    #[test]
    fn test_service_workflow() {
        let service = ApiStudioService::open_in_memory().unwrap();

        let coll = ApiCollection {
            id: "c-test".to_string(),
            name: "Microservices Test".to_string(),
            description: "Local test suite".to_string(),
            variables: vec![KeyValuePair::new("port", "8080")],
            created_at: 1700000000,
            updated_at: 1700000000,
        };
        service.save_collection(&coll).unwrap();

        let req = ApiRequestItem::new(
            "r-test",
            "c-test",
            "Healthcheck",
            HttpMethod::GET,
            "http://127.0.0.1:{{port}}/health",
        );
        service.save_request(&req).unwrap();

        let stats = service.get_stats().unwrap();
        assert_eq!(stats.total_collections, 1);
        assert_eq!(stats.total_requests, 1);

        // Execute request against offline mock/closed port (expect 504 gateway response, no crash)
        let resp = service.execute_request("r-test", None).unwrap();
        assert!(resp.status_code == 200 || resp.status_code == 504);

        // Test Environment Switching & Token Preview
        let env = ApiEnvironment {
            id: "env-staging".to_string(),
            name: "Staging Cluster".to_string(),
            variables: vec![
                KeyValuePair::new("port", "9090"),
                KeyValuePair::new("subdomain", "stage"),
            ],
            is_active: true,
        };
        service.save_environment(&env).unwrap();
        service.set_active_environment("env-staging").unwrap();

        let preview = service
            .resolve_variables(
                "https://{{subdomain}}.internal:{{port}}/v1/test",
                None,
                None,
            )
            .unwrap();
        assert_eq!(preview.resolved_text, "https://stage.internal:9090/v1/test");
        assert_eq!(preview.tokens.len(), 2);
        assert_eq!(preview.unresolved_count, 0);

        // Test cURL import
        let curl_req = service
            .import_curl(
                r#"curl -X PUT "http://127.0.0.1:8080/users/42" -H "Content-Type: application/json" -d '{"active":true}'"#,
                "c-test",
            )
            .unwrap();
        assert_eq!(curl_req.method, HttpMethod::PUT);
        assert_eq!(curl_req.collection_id, "c-test");

        // Test OpenAPI import & export
        let openapi_doc = r#"{
            "openapi": "3.1.0",
            "info": { "title": "Service OpenAPI", "version": "1.0.0" },
            "paths": {
                "/metrics": {
                    "get": { "summary": "System metrics" }
                }
            }
        }"#;
        let oas_summary = service.import_openapi(openapi_doc).unwrap();
        assert_eq!(oas_summary.requests_count, 1);
        let exported_oas = service.export_openapi(&oas_summary.collection_id).unwrap();
        assert!(exported_oas["paths"].get("/metrics").is_some());

        // Test Postman import & export
        let pm_doc = r#"{
            "info": {
                "name": "Service Postman",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [{
                "name": "Ping",
                "request": {
                    "method": "GET",
                    "url": "http://127.0.0.1:8080/ping"
                }
            }]
        }"#;
        let pm_summary = service.import_postman(pm_doc).unwrap();
        assert_eq!(pm_summary.requests_count, 1);
        let exported_pm = service.export_postman(&pm_summary.collection_id).unwrap();
        assert_eq!(exported_pm["info"]["name"], "Service Postman");

        // Test OWASP Audit Report Generation
        let owasp_report = service.run_owasp_audit("r-test", "id").unwrap();
        assert_eq!(owasp_report.method, "GET");
        assert!(owasp_report.total_tests >= 20);
        assert!(owasp_report
            .markdown_summary
            .contains("OWASP API Security Audit Report"));
    }
}
