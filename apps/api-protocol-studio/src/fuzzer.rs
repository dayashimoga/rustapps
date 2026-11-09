#![forbid(unsafe_code)]
//! Automated Offline Security & Protocol Fuzzing Engine

use crate::http_client::AirGappedHttpClient;
use crate::models::{
    ApiRequestItem, FuzzingResult, FuzzingStrategy, OwaspReportFinding, OwaspSecurityReport,
};

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
            let mut fuzzed_request = base_request.clone();
            // Bound fuzzing timeout to at most 100ms per permutation for offline responsiveness
            fuzzed_request.timeout_ms = base_request.timeout_ms.clamp(20, 100);

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

    /// Synthesizes fuzzing permutation results into a formal OWASP API Security Audit Report
    pub fn generate_owasp_report(
        base_request: &ApiRequestItem,
        resolved_url: &str,
        results: &[FuzzingResult],
    ) -> OwaspSecurityReport {
        let mut findings = Vec::new();
        let total_tests = results.len();
        let mut anomalies_detected = 0;
        let mut highest_severity = 0;

        for res in results {
            if res.anomaly_detected {
                anomalies_detected += 1;

                let (owasp_id, title, severity, sev_score, rem) = if res.payload.contains("whoami")
                    || res.payload.contains("/etc/passwd")
                    || res.payload.contains("dir")
                {
                    (
                        "API8:2023",
                        "Command Injection / Remote Code Execution",
                        "CRITICAL",
                        4,
                        "Sanitize input using strict allowlists, never pass user inputs into OS command shells, and use parameterized system APIs.",
                    )
                } else if res.payload.contains("UNION SELECT")
                    || res.payload.contains("DROP TABLE")
                    || res.payload.contains("' OR '1'='1")
                {
                    (
                        "API8:2023",
                        "SQL Injection Flaw Detected",
                        "HIGH",
                        3,
                        "Use parameterized SQL prepared statements or an ORM; never concatenate query strings directly into database drivers.",
                    )
                } else if res.payload.len() > 80 {
                    (
                        "API4:2023",
                        "Unrestricted Resource Consumption / Buffer Size",
                        "MEDIUM",
                        2,
                        "Enforce strict maximum string length limits and request payload size validation in reverse proxy/middleware.",
                    )
                } else if res.payload.contains("<script>") || res.payload.contains('\0') {
                    (
                        "API3:2023",
                        "Broken Object Property Level Validation",
                        "MEDIUM",
                        2,
                        "Validate all query and body characters against schema specifications, rejecting null bytes and script injection tokens.",
                    )
                } else {
                    (
                        "API10:2023",
                        "Unsafe Consumption of Boundary Values",
                        "LOW",
                        1,
                        "Ensure error handling handles non-standard numerical boundaries (NaN, -1, 64-bit int overflows) gracefully.",
                    )
                };

                if sev_score > highest_severity {
                    highest_severity = sev_score;
                }

                findings.push(OwaspReportFinding {
                    owasp_id: owasp_id.to_string(),
                    title: title.to_string(),
                    severity: severity.to_string(),
                    strategy: "Parameter Mutation Fuzzing".to_string(),
                    target_param: res.target_param.clone(),
                    payload: res.payload.clone(),
                    evidence: format!(
                        "Status: {}, Latency: {}ms, Notes: {}",
                        res.status_code, res.latency_ms, res.notes
                    ),
                    remediation: rem.to_string(),
                });
            }
        }

        let overall_risk = match highest_severity {
            4 => "CRITICAL",
            3 => "HIGH",
            2 => "MEDIUM",
            1 => "LOW",
            _ => "CLEAN",
        };

        let report_id = format!("owasp-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let now = 1700000000;

        let mut md = format!(
            "# OWASP API Security Audit Report\n\n- **Report ID:** `{}`\n- **Target Endpoint:** `{} {}`\n- **Overall Risk:** **{}**\n- **Checks Run:** {}\n- **Anomalies Detected:** {}\n\n| OWASP Category | Severity | Parameter | Payload | Evidence |\n| :--- | :--- | :--- | :--- | :--- |\n",
            report_id,
            base_request.method.as_str(),
            resolved_url,
            overall_risk,
            total_tests,
            anomalies_detected,
        );

        if findings.is_empty() {
            md.push_str(
                "| None | CLEAN | N/A | N/A | All adversarial fuzzing permutations handled safely |\n",
            );
        } else {
            for f in &findings {
                md.push_str(&format!(
                    "| {} - {} | **{}** | `{}` | `{}` | {} |\n",
                    f.owasp_id,
                    f.title,
                    f.severity,
                    f.target_param,
                    f.payload.replace('|', "\\|"),
                    f.evidence
                ));
            }
        }

        OwaspSecurityReport {
            report_id,
            generated_at: now,
            target_url: resolved_url.to_string(),
            method: base_request.method.as_str().to_string(),
            total_tests,
            anomalies_detected,
            overall_risk_score: overall_risk.to_string(),
            findings,
            markdown_summary: md,
        }
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

        let report = ApiFuzzer::generate_owasp_report(&req, &req.url, &results);
        assert_eq!(report.target_url, "http://127.0.0.1:8080/api");
        assert_eq!(report.total_tests, 10);
        assert!(report
            .markdown_summary
            .contains("OWASP API Security Audit Report"));
    }
}
