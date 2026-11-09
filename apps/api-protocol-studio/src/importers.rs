#![forbid(unsafe_code)]
//! Git-Native Collection Importers & Exporters (cURL, OpenAPI 3.1, Postman v2.1)

use crate::models::{
    ApiCollection, ApiRequestItem, AuthConfig, HttpMethod, KeyValuePair, ProtocolType, RequestBody,
};
use serde_json::{json, Value};
use uuid::Uuid;

pub struct CurlParser;

impl CurlParser {
    /// Parses a cURL command string into an `ApiRequestItem`
    pub fn parse(curl_cmd: &str, collection_id: &str) -> Result<ApiRequestItem, String> {
        let tokens = Self::tokenize(curl_cmd)?;
        if tokens.is_empty() {
            return Err("Empty curl command".to_string());
        }

        let mut method = HttpMethod::GET;
        let mut url = String::new();
        let mut headers = Vec::new();
        let mut body_content = String::new();
        let mut has_body = false;
        let mut auth = AuthConfig::None;

        let mut i = 0;
        // Skip leading 'curl' if present
        if i < tokens.len() && tokens[i].eq_ignore_ascii_case("curl") {
            i += 1;
        }

        while i < tokens.len() {
            let token = &tokens[i];
            match token.as_str() {
                "-X" | "--request" => {
                    i += 1;
                    if i < tokens.len() {
                        method = HttpMethod::from_str_lenient(&tokens[i]);
                    }
                }
                "-H" | "--header" => {
                    i += 1;
                    if i < tokens.len() {
                        let header_str = &tokens[i];
                        if let Some((k, v)) = header_str.split_once(':') {
                            let key = k.trim().to_string();
                            let value = v.trim().to_string();
                            if key.eq_ignore_ascii_case("Authorization")
                                && value.starts_with("Bearer ")
                            {
                                auth = AuthConfig::BearerToken {
                                    token: value.trim_start_matches("Bearer ").trim().to_string(),
                                };
                            }
                            headers.push(KeyValuePair::new(key, value));
                        }
                    }
                }
                "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                    i += 1;
                    if i < tokens.len() {
                        if !body_content.is_empty() {
                            body_content.push('&');
                        }
                        body_content.push_str(&tokens[i]);
                        has_body = true;
                    }
                }
                "-u" | "--user" => {
                    i += 1;
                    if i < tokens.len() {
                        let creds = &tokens[i];
                        let (user, pass) = match creds.split_once(':') {
                            Some((u, p)) => (u.to_string(), p.to_string()),
                            None => (creds.to_string(), String::new()),
                        };
                        auth = AuthConfig::BasicAuth {
                            username: user,
                            password: pass,
                        };
                    }
                }
                "-b" | "--cookie" => {
                    i += 1;
                    if i < tokens.len() {
                        headers.push(KeyValuePair::new("Cookie", tokens[i].clone()));
                    }
                }
                other => {
                    if !other.starts_with('-')
                        && (url.is_empty()
                            || other.starts_with("http://")
                            || other.starts_with("https://")
                            || other.starts_with("{{"))
                    {
                        url = other.to_string();
                    }
                }
            }
            i += 1;
        }

        if url.is_empty() {
            return Err("Could not extract a valid URL from cURL command".to_string());
        }

        // Auto-switch to POST if data was provided and method was not explicitly changed
        if has_body && method == HttpMethod::GET {
            method = HttpMethod::POST;
        }

        // Extract query parameters from URL if present
        let mut query_params = Vec::new();
        if let Some((base_url, query_str)) = url.split_once('?') {
            for pair in query_str.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    query_params.push(KeyValuePair::new(k, v));
                } else if !pair.is_empty() {
                    query_params.push(KeyValuePair::new(pair, ""));
                }
            }
            url = base_url.to_string();
        }

        let body_type = if has_body {
            let trimmed = body_content.trim();
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                "json"
            } else {
                "raw"
            }
        } else {
            "none"
        };

        let req_id = format!("req-curl-{}", &Uuid::new_v4().to_string()[..8]);
        let name = format!(
            "{} {}",
            method.as_str(),
            url.split('/').next_back().unwrap_or("Endpoint")
        );

        Ok(ApiRequestItem {
            id: req_id,
            collection_id: collection_id.to_string(),
            name,
            protocol: ProtocolType::HttpRest,
            method,
            url,
            headers,
            query_params,
            auth,
            body: RequestBody {
                body_type: body_type.to_string(),
                raw_content: body_content,
                graphql_query: None,
                graphql_variables: None,
            },
            timeout_ms: 10_000,
        })
    }

    /// Splits command line into tokens respecting single and double quotes
    fn tokenize(cmd: &str) -> Result<Vec<String>, String> {
        let mut tokens = Vec::new();
        let chars = cmd.chars();
        let mut current = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escape_next = false;

        for ch in chars {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if !in_single_quote => {
                    escape_next = true;
                }
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                ' ' | '\t' | '\r' | '\n' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        Ok(tokens)
    }
}

pub struct OpenApiCodec;

impl OpenApiCodec {
    /// Imports an OpenAPI 3.0 / 3.1 JSON document into a collection and requests
    pub fn import(json_str: &str) -> Result<(ApiCollection, Vec<ApiRequestItem>), String> {
        let root: Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse OpenAPI JSON: {}", e))?;

        let title = root
            .pointer("/info/title")
            .and_then(|v| v.as_str())
            .unwrap_or("OpenAPI Specification")
            .to_string();

        let description = root
            .pointer("/info/description")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported from OpenAPI 3.1 specification")
            .to_string();

        let collection_id = format!("coll-openapi-{}", &Uuid::new_v4().to_string()[..8]);

        // Extract base server URL if available
        let mut variables = Vec::new();
        if let Some(servers) = root.get("servers").and_then(|s| s.as_array()) {
            if let Some(first_url) = servers
                .first()
                .and_then(|s| s.get("url"))
                .and_then(|u| u.as_str())
            {
                variables.push(KeyValuePair::new("baseUrl", first_url));
            }
        }
        if variables.is_empty() {
            variables.push(KeyValuePair::new("baseUrl", "http://localhost:8080"));
        }

        let mut requests = Vec::new();

        if let Some(paths) = root.get("paths").and_then(|p| p.as_object()) {
            for (path, path_item) in paths {
                if let Some(methods) = path_item.as_object() {
                    for (method_str, op) in methods {
                        let http_method = match method_str.to_ascii_uppercase().as_str() {
                            "GET" => HttpMethod::GET,
                            "POST" => HttpMethod::POST,
                            "PUT" => HttpMethod::PUT,
                            "DELETE" => HttpMethod::DELETE,
                            "PATCH" => HttpMethod::PATCH,
                            "HEAD" => HttpMethod::HEAD,
                            "OPTIONS" => HttpMethod::OPTIONS,
                            _ => continue,
                        };

                        let summary = op
                            .get("summary")
                            .or_else(|| op.get("operationId"))
                            .and_then(|s| s.as_str())
                            .unwrap_or(path.as_str());

                        let mut headers = Vec::new();
                        let mut query_params = Vec::new();

                        if let Some(parameters) = op.get("parameters").and_then(|p| p.as_array()) {
                            for param in parameters {
                                let name = param.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                let location =
                                    param.get("in").and_then(|l| l.as_str()).unwrap_or("");
                                let example =
                                    param.get("example").and_then(|e| e.as_str()).unwrap_or("");

                                match location {
                                    "query" => query_params.push(KeyValuePair::new(name, example)),
                                    "header" => headers.push(KeyValuePair::new(name, example)),
                                    _ => {}
                                }
                            }
                        }

                        let mut body_content = String::new();
                        let mut body_type = "none".to_string();

                        if let Some(content) = op.pointer("/requestBody/content/application~1json")
                        {
                            headers.push(KeyValuePair::new("Content-Type", "application/json"));
                            body_type = "json".to_string();
                            if let Some(example) = content.get("example") {
                                body_content =
                                    serde_json::to_string_pretty(example).unwrap_or_default();
                            } else {
                                body_content = "{}".to_string();
                            }
                        }

                        let full_url = format!("{{{{baseUrl}}}}{}", path);
                        let req_id = format!("req-openapi-{}", &Uuid::new_v4().to_string()[..8]);

                        requests.push(ApiRequestItem {
                            id: req_id,
                            collection_id: collection_id.clone(),
                            name: summary.to_string(),
                            protocol: ProtocolType::HttpRest,
                            method: http_method,
                            url: full_url,
                            headers,
                            query_params,
                            auth: AuthConfig::None,
                            body: RequestBody {
                                body_type,
                                raw_content: body_content,
                                graphql_query: None,
                                graphql_variables: None,
                            },
                            timeout_ms: 10_000,
                        });
                    }
                }
            }
        }

        let now = 1700000000;
        let collection = ApiCollection {
            id: collection_id,
            name: title,
            description,
            variables,
            created_at: now,
            updated_at: now,
        };

        Ok((collection, requests))
    }

    /// Exports a collection and its requests into OpenAPI 3.1 JSON format
    pub fn export(collection: &ApiCollection, requests: &[ApiRequestItem]) -> Value {
        let mut paths_obj = serde_json::Map::new();

        for req in requests {
            // Strip {{baseUrl}} if present to get path
            let path = if req.url.starts_with("{{baseUrl}}") {
                req.url.trim_start_matches("{{baseUrl}}").to_string()
            } else if let Some(idx) = req.url.find("://") {
                let rest = &req.url[idx + 3..];
                if let Some(slash_idx) = rest.find('/') {
                    rest[slash_idx..].to_string()
                } else {
                    "/".to_string()
                }
            } else {
                req.url.clone()
            };

            let clean_path = if path.is_empty() || !path.starts_with('/') {
                format!("/{}", path)
            } else {
                path
            };

            let method_key = req.method.as_str().to_ascii_lowercase();

            let mut parameters = Vec::new();
            for q in &req.query_params {
                parameters.push(json!({
                    "name": q.key,
                    "in": "query",
                    "required": false,
                    "schema": { "type": "string" },
                    "example": q.value
                }));
            }
            for h in &req.headers {
                if !h.key.eq_ignore_ascii_case("content-type") {
                    parameters.push(json!({
                        "name": h.key,
                        "in": "header",
                        "required": false,
                        "schema": { "type": "string" },
                        "example": h.value
                    }));
                }
            }

            let mut op_obj = json!({
                "summary": req.name,
                "operationId": req.id,
                "parameters": parameters,
                "responses": {
                    "200": {
                        "description": "Successful operation"
                    }
                }
            });

            if req.body.body_type == "json" && !req.body.raw_content.is_empty() {
                let parsed_body: Value = serde_json::from_str(&req.body.raw_content)
                    .unwrap_or(json!({ "raw": req.body.raw_content }));
                op_obj["requestBody"] = json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "type": "object" },
                            "example": parsed_body
                        }
                    }
                });
            }

            let path_entry = paths_obj.entry(clean_path).or_insert_with(|| json!({}));
            if let Some(map) = path_entry.as_object_mut() {
                map.insert(method_key, op_obj);
            }
        }

        let base_url = collection
            .variables
            .iter()
            .find(|v| v.key == "baseUrl")
            .map(|v| v.value.as_str())
            .unwrap_or("http://localhost:8080");

        json!({
            "openapi": "3.1.0",
            "info": {
                "title": collection.name,
                "description": collection.description,
                "version": "1.0.0"
            },
            "servers": [
                {
                    "url": base_url,
                    "description": "Default Environment Server"
                }
            ],
            "paths": paths_obj
        })
    }
}

pub struct PostmanCodec;

impl PostmanCodec {
    /// Imports a Postman Collection v2.0 / v2.1 JSON string
    pub fn import(json_str: &str) -> Result<(ApiCollection, Vec<ApiRequestItem>), String> {
        let root: Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse Postman JSON: {}", e))?;

        let name = root
            .pointer("/info/name")
            .and_then(|v| v.as_str())
            .unwrap_or("Postman Collection")
            .to_string();

        let description = root
            .pointer("/info/description")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported from Postman Collection v2.1")
            .to_string();

        let collection_id = format!("coll-postman-{}", &Uuid::new_v4().to_string()[..8]);

        // Variables
        let mut variables = Vec::new();
        if let Some(vars) = root.get("variable").and_then(|v| v.as_array()) {
            for v in vars {
                let k = v.get("key").and_then(|s| s.as_str()).unwrap_or("");
                let val = v.get("value").and_then(|s| s.as_str()).unwrap_or("");
                if !k.is_empty() {
                    variables.push(KeyValuePair::new(k, val));
                }
            }
        }

        let mut requests = Vec::new();
        if let Some(items) = root.get("item").and_then(|i| i.as_array()) {
            Self::flatten_items(items, &collection_id, &mut requests);
        }

        let now = 1700000000;
        let collection = ApiCollection {
            id: collection_id,
            name,
            description,
            variables,
            created_at: now,
            updated_at: now,
        };

        Ok((collection, requests))
    }

    fn flatten_items(items: &[Value], collection_id: &str, acc: &mut Vec<ApiRequestItem>) {
        for item in items {
            // Nested folder
            if let Some(sub_items) = item.get("item").and_then(|i| i.as_array()) {
                Self::flatten_items(sub_items, collection_id, acc);
                continue;
            }

            if let Some(req_val) = item.get("request") {
                let req_name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Request")
                    .to_string();

                let method_str = req_val
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("GET");
                let method = HttpMethod::from_str_lenient(method_str);

                // URL resolution
                let mut url = String::new();
                let mut query_params = Vec::new();

                if let Some(url_val) = req_val.get("url") {
                    if let Some(s) = url_val.as_str() {
                        url = s.to_string();
                    } else if let Some(obj) = url_val.as_object() {
                        if let Some(raw) = obj.get("raw").and_then(|r| r.as_str()) {
                            url = raw.to_string();
                        }
                        if let Some(q_arr) = obj.get("query").and_then(|q| q.as_array()) {
                            for q in q_arr {
                                let key = q.get("key").and_then(|k| k.as_str()).unwrap_or("");
                                let val = q.get("value").and_then(|v| v.as_str()).unwrap_or("");
                                if !key.is_empty() {
                                    query_params.push(KeyValuePair::new(key, val));
                                }
                            }
                        }
                    }
                }

                // Headers
                let mut headers = Vec::new();
                if let Some(h_arr) = req_val.get("header").and_then(|h| h.as_array()) {
                    for h in h_arr {
                        let key = h.get("key").and_then(|k| k.as_str()).unwrap_or("");
                        let val = h.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        if !key.is_empty() {
                            headers.push(KeyValuePair::new(key, val));
                        }
                    }
                }

                // Auth
                let mut auth = AuthConfig::None;
                if let Some(auth_val) = req_val.get("auth") {
                    if let Some(auth_type) = auth_val.get("type").and_then(|t| t.as_str()) {
                        if auth_type == "bearer" {
                            if let Some(token) =
                                auth_val.pointer("/bearer/0/value").and_then(|t| t.as_str())
                            {
                                auth = AuthConfig::BearerToken {
                                    token: token.to_string(),
                                };
                            }
                        }
                    }
                }

                // Body
                let mut body_type = "none".to_string();
                let mut raw_content = String::new();
                if let Some(body_obj) = req_val.get("body") {
                    let mode = body_obj.get("mode").and_then(|m| m.as_str()).unwrap_or("");
                    if mode == "raw" {
                        raw_content = body_obj
                            .get("raw")
                            .and_then(|r| r.as_str())
                            .unwrap_or("")
                            .to_string();
                        let trimmed = raw_content.trim();
                        if trimmed.starts_with('{') || trimmed.starts_with('[') {
                            body_type = "json".to_string();
                        } else {
                            body_type = "raw".to_string();
                        }
                    }
                }

                let req_id = format!("req-pm-{}", &Uuid::new_v4().to_string()[..8]);

                acc.push(ApiRequestItem {
                    id: req_id,
                    collection_id: collection_id.to_string(),
                    name: req_name,
                    protocol: ProtocolType::HttpRest,
                    method,
                    url,
                    headers,
                    query_params,
                    auth,
                    body: RequestBody {
                        body_type,
                        raw_content,
                        graphql_query: None,
                        graphql_variables: None,
                    },
                    timeout_ms: 10_000,
                });
            }
        }
    }

    /// Exports a collection and requests into Postman Collection v2.1 JSON
    pub fn export(collection: &ApiCollection, requests: &[ApiRequestItem]) -> Value {
        let items: Vec<Value> = requests
            .iter()
            .map(|r| {
                let headers: Vec<Value> = r
                    .headers
                    .iter()
                    .map(|h| {
                        json!({
                            "key": h.key,
                            "value": h.value,
                            "type": "text"
                        })
                    })
                    .collect();

                let body_json = if r.body.body_type == "json" || r.body.body_type == "raw" {
                    json!({
                        "mode": "raw",
                        "raw": r.body.raw_content,
                        "options": {
                            "raw": {
                                "language": if r.body.body_type == "json" { "json" } else { "text" }
                            }
                        }
                    })
                } else {
                    json!({ "mode": "none" })
                };

                json!({
                    "name": r.name,
                    "request": {
                        "method": r.method.as_str(),
                        "header": headers,
                        "body": body_json,
                        "url": {
                            "raw": r.url
                        }
                    }
                })
            })
            .collect();

        let variables: Vec<Value> = collection
            .variables
            .iter()
            .map(|v| {
                json!({
                    "key": v.key,
                    "value": v.value,
                    "type": "string"
                })
            })
            .collect();

        json!({
            "info": {
                "_postman_id": collection.id,
                "name": collection.name,
                "description": collection.description,
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": items,
            "variable": variables
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curl_parser_basic_get() {
        let cmd = "curl https://api.example.com/v1/users?limit=10";
        let req = CurlParser::parse(cmd, "c1").unwrap();
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "https://api.example.com/v1/users");
        assert_eq!(req.query_params.len(), 1);
        assert_eq!(req.query_params[0].key, "limit");
        assert_eq!(req.query_params[0].value, "10");
    }

    #[test]
    fn test_curl_parser_post_with_headers_and_body() {
        let cmd = r#"curl -X POST "http://127.0.0.1:8080/auth/login" -H "Content-Type: application/json" -H "Authorization: Bearer secret-tok" -d '{"username":"admin","pass":"sec"}'"#;
        let req = CurlParser::parse(cmd, "c1").unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "http://127.0.0.1:8080/auth/login");
        assert_eq!(req.headers.len(), 2);
        assert_eq!(req.body.body_type, "json");
        assert!(req.body.raw_content.contains("admin"));
        match req.auth {
            AuthConfig::BearerToken { token } => assert_eq!(token, "secret-tok"),
            _ => panic!("Expected bearer token auth"),
        }
    }

    #[test]
    fn test_openapi_roundtrip() {
        let openapi_json = r#"{
            "openapi": "3.1.0",
            "info": {
                "title": "Petstore Minimal",
                "description": "Offline Petstore Demo",
                "version": "1.0.0"
            },
            "servers": [{ "url": "https://petstore.swagger.io/v2" }],
            "paths": {
                "/pet/{petId}": {
                    "get": {
                        "summary": "Find pet by ID",
                        "operationId": "getPetById",
                        "parameters": [
                            { "name": "petId", "in": "query", "example": "123" }
                        ]
                    }
                }
            }
        }"#;

        let (coll, reqs) = OpenApiCodec::import(openapi_json).unwrap();
        assert_eq!(coll.name, "Petstore Minimal");
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].name, "Find pet by ID");
        assert_eq!(reqs[0].method, HttpMethod::GET);

        let exported = OpenApiCodec::export(&coll, &reqs);
        assert_eq!(exported["info"]["title"], "Petstore Minimal");
        assert!(exported["paths"].get("/pet/{petId}").is_some());
    }

    #[test]
    fn test_postman_roundtrip() {
        let pm_json = r#"{
            "info": {
                "name": "Auth Suite",
                "description": "Integration test auth",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "variable": [
                { "key": "baseUrl", "value": "http://127.0.0.1:3000" }
            ],
            "item": [
                {
                    "name": "Login Request",
                    "request": {
                        "method": "POST",
                        "url": "http://127.0.0.1:3000/login",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\"user\": \"test\"}"
                        }
                    }
                }
            ]
        }"#;

        let (coll, reqs) = PostmanCodec::import(pm_json).unwrap();
        assert_eq!(coll.name, "Auth Suite");
        assert_eq!(coll.variables.len(), 1);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].name, "Login Request");
        assert_eq!(reqs[0].method, HttpMethod::POST);

        let exported = PostmanCodec::export(&coll, &reqs);
        assert_eq!(exported["info"]["name"], "Auth Suite");
        assert_eq!(exported["item"].as_array().unwrap().len(), 1);
    }
}
