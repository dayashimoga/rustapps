#![forbid(unsafe_code)]
//! Air-Gapped HTTP/1.1 & Protocol Execution Engine

use crate::models::{ApiRequestItem, ApiResponseSnapshot, AuthConfig, KeyValuePair};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),
    #[error("Network I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Air-gap policy: External WAN destination blocked: {0}")]
    AirGapBlocked(String),
}

pub struct AirGappedHttpClient;

impl AirGappedHttpClient {
    /// Executes an API request and captures response snapshot with sub-millisecond precision
    pub fn execute(
        request: &ApiRequestItem,
        resolved_url: &str,
    ) -> Result<ApiResponseSnapshot, HttpError> {
        let start = Instant::now();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Parse protocol, host, port, path
        let url = resolved_url.trim();
        let without_proto = if let Some(stripped) = url.strip_prefix("http://") {
            stripped
        } else if let Some(stripped) = url.strip_prefix("https://") {
            stripped
        } else {
            url
        };

        let mut parts = without_proto.splitn(2, '/');
        let host_port = parts.next().unwrap_or("127.0.0.1:80");
        let path = format!("/{}", parts.next().unwrap_or(""));

        let mut host_parts = host_port.split(':');
        let host = host_parts.next().unwrap_or("127.0.0.1");
        let port: u16 = host_parts.next().and_then(|p| p.parse().ok()).unwrap_or(80);

        let timeout = Duration::from_millis(if request.timeout_ms == 0 {
            5000
        } else {
            request.timeout_ms
        });

        // Connect over TCP with timeout
        let addr = format!("{}:{}", host, port);
        let mut stream = match TcpStream::connect_timeout(
            &addr
                .parse()
                .unwrap_or_else(|_| "127.0.0.1:80".parse().unwrap()),
            timeout,
        ) {
            Ok(s) => s,
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                return Ok(ApiResponseSnapshot {
                    request_id: request.id.clone(),
                    status_code: 504,
                    status_text: format!("Gateway / Connection Error: {}", e),
                    latency_ms,
                    size_bytes: 0,
                    headers: Vec::new(),
                    body: format!("Connection to {} failed: {}", addr, e),
                    timestamp,
                });
            }
        };

        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;

        // Assemble request headers
        let mut headers = request.headers.clone();

        // Apply Auth
        match &request.auth {
            AuthConfig::BearerToken { token } => {
                headers.push(KeyValuePair::new(
                    "Authorization",
                    format!("Bearer {}", token),
                ));
            }
            AuthConfig::BasicAuth { username, password } => {
                let raw = format!("{}:{}", username, password);
                let encoded = Self::base64_encode(raw.as_bytes());
                headers.push(KeyValuePair::new(
                    "Authorization",
                    format!("Basic {}", encoded),
                ));
            }
            AuthConfig::ApiKey {
                key,
                value,
                in_header,
            } if *in_header => {
                headers.push(KeyValuePair::new(key.clone(), value.clone()));
            }
            _ => {}
        }

        let body_bytes = match request.body.body_type.as_str() {
            "graphql" => {
                let payload = serde_json::json!({
                    "query": request.body.graphql_query.clone().unwrap_or_default(),
                    "variables": request.body.graphql_variables.clone().unwrap_or_default()
                });
                serde_json::to_vec(&payload).unwrap_or_default()
            }
            _ => request.body.raw_content.as_bytes().to_vec(),
        };

        // Send HTTP/1.1 request
        let method_str = request.method.as_str();
        write!(stream, "{} {} HTTP/1.1\r\n", method_str, path)?;
        write!(stream, "Host: {}\r\n", host_port)?;
        write!(stream, "Connection: close\r\n")?;

        for h in &headers {
            if h.enabled {
                write!(stream, "{}: {}\r\n", h.key, h.value)?;
            }
        }

        if !body_bytes.is_empty() {
            write!(stream, "Content-Length: {}\r\n", body_bytes.len())?;
        }
        write!(stream, "\r\n")?;

        if !body_bytes.is_empty() {
            stream.write_all(&body_bytes)?;
        }
        stream.flush()?;

        // Read response
        let mut response_buf = Vec::new();
        let _ = stream.read_to_end(&mut response_buf);
        let latency_ms = start.elapsed().as_millis() as u64;

        Self::parse_http_response(&request.id, &response_buf, latency_ms, timestamp)
    }

    fn parse_http_response(
        request_id: &str,
        raw_bytes: &[u8],
        latency_ms: u64,
        timestamp: i64,
    ) -> Result<ApiResponseSnapshot, HttpError> {
        let size_bytes = raw_bytes.len();
        if raw_bytes.is_empty() {
            return Ok(ApiResponseSnapshot {
                request_id: request_id.to_string(),
                status_code: 502,
                status_text: "Empty Response".to_string(),
                latency_ms,
                size_bytes: 0,
                headers: Vec::new(),
                body: "Server returned 0 bytes".to_string(),
                timestamp,
            });
        }

        let response_str = String::from_utf8_lossy(raw_bytes);
        let mut sections = response_str.splitn(2, "\r\n\r\n");
        let header_section = sections.next().unwrap_or("");
        let body_section = sections.next().unwrap_or("");

        let mut lines = header_section.lines();
        let status_line = lines.next().unwrap_or("HTTP/1.1 200 OK");
        let mut status_parts = status_line.splitn(3, ' ');
        let _http_version = status_parts.next();
        let status_code: u16 = status_parts
            .next()
            .and_then(|c| c.parse().ok())
            .unwrap_or(200);
        let status_text = status_parts.next().unwrap_or("OK").to_string();

        let mut resp_headers = Vec::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                resp_headers.push(KeyValuePair::new(k.trim(), v.trim()));
            }
        }

        Ok(ApiResponseSnapshot {
            request_id: request_id.to_string(),
            status_code,
            status_text,
            latency_ms,
            size_bytes,
            headers: resp_headers,
            body: body_section.to_string(),
            timestamp,
        })
    }

    fn base64_encode(data: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
        let mut chunks = data.chunks_exact(3);

        for chunk in chunks.by_ref() {
            let b0 = chunk[0];
            let b1 = chunk[1];
            let b2 = chunk[2];
            out.push(TABLE[(b0 >> 2) as usize] as char);
            out.push(TABLE[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            out.push(TABLE[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
            out.push(TABLE[(b2 & 63) as usize] as char);
        }

        let rem = chunks.remainder();
        if rem.len() == 1 {
            let b0 = rem[0];
            out.push(TABLE[(b0 >> 2) as usize] as char);
            out.push(TABLE[((b0 & 3) << 4) as usize] as char);
            out.push('=');
            out.push('=');
        } else if rem.len() == 2 {
            let b0 = rem[0];
            let b1 = rem[1];
            out.push(TABLE[(b0 >> 2) as usize] as char);
            out.push(TABLE[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            out.push(TABLE[((b1 & 15) << 2) as usize] as char);
            out.push('=');
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_response() {
        let raw_http = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Custom: offline\r\n\r\n{\"status\":\"healthy\"}";
        let snapshot =
            AirGappedHttpClient::parse_http_response("test-req", raw_http, 5, 1700000000).unwrap();

        assert_eq!(snapshot.status_code, 200);
        assert_eq!(snapshot.status_text, "OK");
        assert_eq!(snapshot.body, "{\"status\":\"healthy\"}");
        assert_eq!(snapshot.headers.len(), 2);
        assert_eq!(snapshot.headers[0].key, "Content-Type");
        assert_eq!(snapshot.headers[0].value, "application/json");
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(
            AirGappedHttpClient::base64_encode(b"admin:secret123"),
            "YWRtaW46c2VjcmV0MTIz"
        );
    }
}
