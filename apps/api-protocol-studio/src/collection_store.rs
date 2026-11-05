#![forbid(unsafe_code)]
//! SQLite-backed Persistent Storage for Collections, Requests, History, and Variables

use crate::models::{
    ApiCollection, ApiRequestItem, ApiResponseSnapshot, HttpMethod, KeyValuePair, ProtocolType,
};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct CollectionStore {
    conn: Mutex<Connection>,
}

impl CollectionStore {
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                variables_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS requests (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                protocol TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                headers_json TEXT NOT NULL,
                query_params_json TEXT NOT NULL,
                auth_json TEXT NOT NULL,
                body_json TEXT NOT NULL,
                timeout_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
                status_code INTEGER NOT NULL,
                status_text TEXT NOT NULL,
                latency_ms INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                headers_json TEXT NOT NULL,
                body TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_requests_coll ON requests(collection_id);
            CREATE INDEX IF NOT EXISTS idx_history_req ON history(request_id);
            ",
        )?;
        Ok(())
    }

    pub fn save_collection(&self, collection: &ApiCollection) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let vars_json = serde_json::to_string(&collection.variables)?;
        conn.execute(
            "INSERT OR REPLACE INTO collections (id, name, description, variables_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                collection.id,
                collection.name,
                collection.description,
                vars_json,
                collection.created_at,
                collection.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_collection(&self, id: &str) -> Result<Option<ApiCollection>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, variables_json, created_at, updated_at
             FROM collections WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let vars_json: String = row.get(3)?;
            let variables: Vec<KeyValuePair> = serde_json::from_str(&vars_json).unwrap_or_default();
            Ok(Some(ApiCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                variables,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_collections(&self) -> Result<Vec<ApiCollection>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, variables_json, created_at, updated_at
             FROM collections ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let vars_json: String = row.get(3)?;
            let variables: Vec<KeyValuePair> = serde_json::from_str(&vars_json).unwrap_or_default();
            Ok(ApiCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                variables,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn delete_collection(&self, id: &str) -> Result<bool, StoreError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn save_request(&self, request: &ApiRequestItem) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let headers_json = serde_json::to_string(&request.headers)?;
        let query_json = serde_json::to_string(&request.query_params)?;
        let auth_json = serde_json::to_string(&request.auth)?;
        let body_json = serde_json::to_string(&request.body)?;

        conn.execute(
            "INSERT OR REPLACE INTO requests (id, collection_id, name, protocol, method, url, headers_json, query_params_json, auth_json, body_json, timeout_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                request.id,
                request.collection_id,
                request.name,
                request.protocol.as_str(),
                request.method.as_str(),
                request.url,
                headers_json,
                query_json,
                auth_json,
                body_json,
                request.timeout_ms as i64
            ],
        )?;
        Ok(())
    }

    pub fn get_request(&self, id: &str) -> Result<Option<ApiRequestItem>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, name, protocol, method, url, headers_json, query_params_json, auth_json, body_json, timeout_ms
             FROM requests WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let protocol_str: String = row.get(3)?;
            let method_str: String = row.get(4)?;
            let headers_json: String = row.get(6)?;
            let query_json: String = row.get(7)?;
            let auth_json: String = row.get(8)?;
            let body_json: String = row.get(9)?;
            let timeout: i64 = row.get(10)?;

            Ok(Some(ApiRequestItem {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                protocol: ProtocolType::from_str_lenient(&protocol_str),
                method: HttpMethod::from_str_lenient(&method_str),
                url: row.get(5)?,
                headers: serde_json::from_str(&headers_json).unwrap_or_default(),
                query_params: serde_json::from_str(&query_json).unwrap_or_default(),
                auth: serde_json::from_str(&auth_json).unwrap_or_default(),
                body: serde_json::from_str(&body_json).unwrap_or_default(),
                timeout_ms: timeout as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_requests_in_collection(
        &self,
        collection_id: &str,
    ) -> Result<Vec<ApiRequestItem>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, name, protocol, method, url, headers_json, query_params_json, auth_json, body_json, timeout_ms
             FROM requests WHERE collection_id = ?1 ORDER BY name ASC",
        )?;
        let rows = stmt.query_map(params![collection_id], |row| {
            let protocol_str: String = row.get(3)?;
            let method_str: String = row.get(4)?;
            let headers_json: String = row.get(6)?;
            let query_json: String = row.get(7)?;
            let auth_json: String = row.get(8)?;
            let body_json: String = row.get(9)?;
            let timeout: i64 = row.get(10)?;

            Ok(ApiRequestItem {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                protocol: ProtocolType::from_str_lenient(&protocol_str),
                method: HttpMethod::from_str_lenient(&method_str),
                url: row.get(5)?,
                headers: serde_json::from_str(&headers_json).unwrap_or_default(),
                query_params: serde_json::from_str(&query_json).unwrap_or_default(),
                auth: serde_json::from_str(&auth_json).unwrap_or_default(),
                body: serde_json::from_str(&body_json).unwrap_or_default(),
                timeout_ms: timeout as u64,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn delete_request(&self, id: &str) -> Result<bool, StoreError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM requests WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn record_response(&self, snapshot: &ApiResponseSnapshot) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let headers_json = serde_json::to_string(&snapshot.headers)?;
        let id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO history (id, request_id, status_code, status_text, latency_ms, size_bytes, headers_json, body, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                snapshot.request_id,
                snapshot.status_code as i64,
                snapshot.status_text,
                snapshot.latency_ms as i64,
                snapshot.size_bytes as i64,
                headers_json,
                snapshot.body,
                snapshot.timestamp
            ],
        )?;
        Ok(())
    }

    pub fn get_latest_response(
        &self,
        request_id: &str,
    ) -> Result<Option<ApiResponseSnapshot>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT request_id, status_code, status_text, latency_ms, size_bytes, headers_json, body, timestamp
             FROM history WHERE request_id = ?1 ORDER BY timestamp DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![request_id])?;

        if let Some(row) = rows.next()? {
            let status: i64 = row.get(1)?;
            let latency: i64 = row.get(3)?;
            let size: i64 = row.get(4)?;
            let headers_json: String = row.get(5)?;

            Ok(Some(ApiResponseSnapshot {
                request_id: row.get(0)?,
                status_code: status as u16,
                status_text: row.get(2)?,
                latency_ms: latency as u64,
                size_bytes: size as usize,
                headers: serde_json::from_str(&headers_json).unwrap_or_default(),
                body: row.get(6)?,
                timestamp: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Interpolates environment variables into an input string (e.g. `{{baseUrl}}/api`)
    pub fn interpolate_variables(input: &str, vars: &[KeyValuePair]) -> String {
        let mut result = input.to_string();
        for v in vars {
            if v.enabled {
                let placeholder = format!("{{{{{}}}}}", v.key);
                result = result.replace(&placeholder, &v.value);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_store_crud_and_interpolation() {
        let store = CollectionStore::open_in_memory().unwrap();

        let coll = ApiCollection {
            id: "coll-1".to_string(),
            name: "Payment Gateway API".to_string(),
            description: "Internal ledger microservice".to_string(),
            variables: vec![KeyValuePair::new("baseUrl", "http://127.0.0.1:8080")],
            created_at: 1700000000,
            updated_at: 1700000000,
        };
        store.save_collection(&coll).unwrap();

        let req = ApiRequestItem::new(
            "req-1",
            "coll-1",
            "Get Balance",
            HttpMethod::GET,
            "{{baseUrl}}/v1/wallets/123/balance",
        );
        store.save_request(&req).unwrap();

        let fetched_coll = store.get_collection("coll-1").unwrap().unwrap();
        assert_eq!(fetched_coll.name, "Payment Gateway API");

        let fetched_req = store.get_request("req-1").unwrap().unwrap();
        assert_eq!(fetched_req.method, HttpMethod::GET);

        // Test variable interpolation
        let resolved_url =
            CollectionStore::interpolate_variables(&fetched_req.url, &fetched_coll.variables);
        assert_eq!(resolved_url, "http://127.0.0.1:8080/v1/wallets/123/balance");

        // Record response
        let resp = ApiResponseSnapshot {
            request_id: "req-1".to_string(),
            status_code: 200,
            status_text: "OK".to_string(),
            latency_ms: 12,
            size_bytes: 48,
            headers: vec![KeyValuePair::new("Content-Type", "application/json")],
            body: "{\"balance\": 5000}".to_string(),
            timestamp: 1700000005,
        };
        store.record_response(&resp).unwrap();

        let latest = store.get_latest_response("req-1").unwrap().unwrap();
        assert_eq!(latest.status_code, 200);
        assert_eq!(latest.body, "{\"balance\": 5000}");
    }
}
