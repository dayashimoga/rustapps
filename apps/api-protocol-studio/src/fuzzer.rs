#![forbid(unsafe_code)]
//! Automated Offline Security & Protocol Fuzzing Engine

use crate::http_client::AirGappedHttpClient;
use crate::models::{ApiRequestItem, FuzzingResult, FuzzingStrategy};

pub struct ApiFuzzer;

impl ApiFuzzer {
    /// Generates adversarial payloads based on strategy
    pub fn generate_payloads(strategy: FuzzingStrategy) -> Vec<&'static str> {
        match strategy {
            FuzzingStrategy::SqlInjection => vec![
                "' OR '1'='1",
                "1; DROP TABLE test;--",
                "' UNION SELECT NULL, NULL, NULL--",
                "admin' --",
                "1' AND 1=(SELECT COUNT(*) FROM tabname); --",
            ],
            FuzzingStrategy::CommandInjection => vec![
                "; whoami",
                "| cat /etc/passwd",
                "& dir",
                "$(id)",
                "`id`",
            ],
            FuzzingStrategy::BufferOverflow => vec![
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
            ],
            FuzzingStrategy::SpecialCharacters => vec![
                "\0",
                "\r\n\r\n",
                "\\x00\\x00",
                "<script>alert(1)</script>",
                "🔥👾💻💥",
                "\u{202E}reversed",
            ],
            FuzzingStrategy::BoundaryValues => vec![
                "0",
                "-1",
                "2147483647",
                "-2147483648",
                "9223372036854775807",
                "null",
                "NaN",
                "undefined",
                "Infinity",
                "-Infinity",
            ],
        }
    }

    /// Executes fuzzing test suite on an endpoint by mutating query parameters or URL
    pub fn fuzz_endpoint(
        base_request: &ApiRequestItem,
        resolved_url: &str,
        strategy: FuzzingStrategy,
        param_name: &str,
    ) -> Vec<FuzzingResult> {
        let payloads = Self::generate_payloads(strategy);
        let mut results = Vec::new();

        for payload in payloads {
            let fuzzed_request = base_request.clone();

            // Mutate target parameter in query parameters or URL
            let mut target_url = resolved_url.to_string();
            if target_url.contains('?') {
                target_url.push_str(&format!("&{}={}", param_name, payload));
            } else {
                target_url.push_str(&format!("?{}={}", param_name, payload));
            }

            let resp =
                AirGappedHttpClient::execute(&fuzzed_request, &target_url).unwrap_or_else(|e| {
                    crate::models::ApiResponseSnapshot {
                        request_id: base_request.id.clone(),
                        status_code: 500,
                        status_text: format!("Error: {}", e),
                        latency_ms: 0,
                        size_bytes: 0,
                        headers: Vec::new(),
                        body: String::new(),
                        timestamp: 0,
                    }
                });

            // Detect anomaly
            let mut anomaly_detected = false;
            let mut notes = Vec::new();

            if resp.status_code >= 500 {
                anomaly_detected = true;
                notes.push(format!(
                    "Server returned 5xx unhandled status ({})",
                    resp.status_code
                ));
            }

            let body_lower = resp.body.to_ascii_lowercase();
            if body_lower.contains("syntax error")
                || body_lower.contains("sql")
                || body_lower.contains("exception")
                || body_lower.contains("stack trace")
                || body_lower.contains("panic")
            {
                anomaly_detected = true;
                notes.push("Internal framework error or stack trace detected in body".to_string());
            }

            if resp.latency_ms > 2000 {
                anomaly_detected = true;
                notes.push(format!("High response latency: {}ms", resp.latency_ms));
            }

            if notes.is_empty() {
                notes.push("Normal handled response".to_string());
            }

            results.push(FuzzingResult {
                payload: payload.to_string(),
                target_param: param_name.to_string(),
                status_code: resp.status_code,
                latency_ms: resp.latency_ms,
                anomaly_detected,
                notes: notes.join("; "),
            });
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HttpMethod;

    #[test]
    fn test_payload_generation() {
        let sql = ApiFuzzer::generate_payloads(FuzzingStrategy::SqlInjection);
        assert!(!sql.is_empty());
        assert!(sql[0].contains("OR"));

        let cmd = ApiFuzzer::generate_payloads(FuzzingStrategy::CommandInjection);
        assert!(cmd[0].contains("whoami"));
    }

    #[test]
    fn test_fuzz_endpoint_mock() {
        let req = ApiRequestItem::new(
            "fuzz-req",
            "c1",
            "Test",
            HttpMethod::GET,
            "http://127.0.0.1:8080/api",
        );
        let results =
            ApiFuzzer::fuzz_endpoint(&req, &req.url, FuzzingStrategy::BoundaryValues, "id");
        assert_eq!(results.len(), 10);
        assert_eq!(results[0].target_param, "id");
    }
}
