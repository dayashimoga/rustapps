#![forbid(unsafe_code)]
//! API & Protocol Studio Service Orchestrator

use crate::collection_store::{CollectionStore, StoreError};
use crate::fuzzer::ApiFuzzer;
use crate::http_client::{AirGappedHttpClient, HttpError};
use crate::models::{
    ApiCollection, ApiRequestItem, ApiResponseSnapshot, FuzzingResult, FuzzingStrategy,
    KeyValuePair,
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

        // Merge collection variables with runtime overrides
        let mut vars = Vec::new();
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

        let vars = if let Ok(Some(coll)) = self.store.get_collection(&request.collection_id) {
            coll.variables
        } else {
            Vec::new()
        };
        let resolved_url = CollectionStore::interpolate_variables(&request.url, &vars);

        Ok(ApiFuzzer::fuzz_endpoint(
            &request,
            &resolved_url,
            strategy,
            param_name,
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
    }
}
