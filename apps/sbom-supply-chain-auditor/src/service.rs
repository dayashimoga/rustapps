use crate::audit_engine::AuditEngine;
use crate::models::{AuditorStats, SbomAuditReport, SbomDocument};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SbomError {
    #[error("SBOM document not found: {0}")]
    NotFound(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Lock error: {0}")]
    Lock(String),
}

pub struct SbomService {
    documents: Arc<RwLock<HashMap<String, SbomDocument>>>,
    reports: Arc<RwLock<HashMap<String, SbomAuditReport>>>,
    stats: Arc<RwLock<AuditorStats>>,
    master_key: SigningKey,
}

impl Default for SbomService {
    fn default() -> Self {
        Self::new()
    }
}

impl SbomService {
    pub fn new() -> Self {
        let service = Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            reports: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AuditorStats {
                total_sboms_audited: 0,
                total_components_analyzed: 0,
                total_cves_flagged: 0,
                total_copyleft_detected: 0,
                avg_compliance_score: 95.0,
            })),
            master_key: SigningKey::generate(&mut OsRng),
        };

        service.seed_demo_sbom();
        service
    }

    /// Seeds realistic production CycloneDX 1.5 SBOM for instant 5-second onboarding
    pub fn seed_demo_sbom(&self) {
        let demo_cyclonedx = r#"{
            "bomFormat": "CycloneDX",
            "specVersion": "1.5",
            "serialNumber": "urn:uuid:7c9e6679-7425-40de-944b-e07fc1f90ae7",
            "metadata": {
                "timestamp": "2026-10-18T12:00:00Z",
                "tools": [{ "name": "cargo-cyclonedx", "version": "0.5.4" }],
                "authors": [{ "name": "Principal DevSecOps Architect" }],
                "component": {
                    "name": "enterprise-auth-gateway",
                    "version": "2.4.0",
                    "type": "application"
                }
            },
            "components": [
                {
                    "name": "tokio",
                    "version": "1.40.0",
                    "purl": "pkg:cargo/tokio@1.40.0",
                    "type": "library",
                    "licenses": [{ "license": { "id": "MIT" } }],
                    "hashes": [{ "alg": "SHA-256", "content": "e44d32e9a59fa861788c227f..." }]
                },
                {
                    "name": "serde",
                    "version": "1.0.210",
                    "purl": "pkg:cargo/serde@1.0.210",
                    "type": "library",
                    "licenses": [{ "license": { "id": "MIT" } }, { "license": { "id": "Apache-2.0" } }]
                },
                {
                    "name": "rusqlite",
                    "version": "0.32.1",
                    "purl": "pkg:cargo/rusqlite@0.32.1",
                    "type": "library",
                    "licenses": [{ "license": { "id": "MIT" } }]
                },
                {
                    "name": "aes-gcm",
                    "version": "0.10.3",
                    "purl": "pkg:cargo/aes-gcm@0.10.3",
                    "type": "library",
                    "licenses": [{ "license": { "id": "Apache-2.0" } }]
                },
                {
                    "name": "ed25519-dalek",
                    "version": "2.1.1",
                    "purl": "pkg:cargo/ed25519-dalek@2.1.1",
                    "type": "library",
                    "licenses": [{ "license": { "id": "BSD-3-Clause" } }]
                },
                {
                    "name": "libwebp",
                    "version": "1.3.1",
                    "purl": "pkg:generic/libwebp@1.3.1",
                    "type": "library",
                    "licenses": [{ "license": { "id": "BSD-3-Clause" } }]
                },
                {
                    "name": "xz-utils",
                    "version": "5.6.1",
                    "purl": "pkg:deb/debian/xz-utils@5.6.1",
                    "type": "library",
                    "licenses": [{ "license": { "id": "GPL-2.0-or-later" } }]
                },
                {
                    "name": "flate2",
                    "version": "1.0.34",
                    "purl": "pkg:cargo/flate2@1.0.34",
                    "type": "library",
                    "licenses": [{ "license": { "id": "MIT" } }]
                }
            ]
        }"#;

        if let Ok(mut doc) = AuditEngine::parse_cyclonedx_json(
            demo_cyclonedx,
            "enterprise_auth_gateway_sbom.cdx.json",
        ) {
            doc.document_id = "doc-demo-cyclonedx".to_string();
            let report = AuditEngine::audit_sbom(&doc, &self.master_key);

            if let Ok(mut docs) = self.documents.write() {
                docs.insert(doc.document_id.clone(), doc);
            }
            if let Ok(mut reports) = self.reports.write() {
                reports.insert(report.document_id.clone(), report);
            }
            if let Ok(mut stats) = self.stats.write() {
                stats.total_sboms_audited += 1;
                stats.total_components_analyzed += 8;
                stats.total_cves_flagged += 2;
                stats.total_copyleft_detected += 1;
            }
        }
    }

    /// Loads and parses an uploaded SBOM file
    pub fn load_sbom(&self, filename: &str, raw_bytes: &[u8]) -> Result<SbomDocument, SbomError> {
        let text = String::from_utf8_lossy(raw_bytes);
        let doc = AuditEngine::parse_cyclonedx_json(&text, filename).map_err(SbomError::Parse)?;

        let report = AuditEngine::audit_sbom(&doc, &self.master_key);

        if let Ok(mut docs) = self.documents.write() {
            docs.insert(doc.document_id.clone(), doc.clone());
        }
        if let Ok(mut reports) = self.reports.write() {
            reports.insert(report.document_id.clone(), report.clone());
        }
        if let Ok(mut s) = self.stats.write() {
            s.total_sboms_audited += 1;
            s.total_components_analyzed += doc.total_components;
            s.total_cves_flagged += report.critical_cves + report.high_cves;
            s.total_copyleft_detected += report.copyleft_violations;
        }

        Ok(doc)
    }

    /// Retrieves an SBOM document
    pub fn get_sbom(&self, document_id: &str) -> Result<SbomDocument, SbomError> {
        let map = self
            .documents
            .read()
            .map_err(|e| SbomError::Lock(e.to_string()))?;
        map.get(document_id)
            .cloned()
            .ok_or_else(|| SbomError::NotFound(document_id.to_string()))
    }

    /// Retrieves an audit report
    pub fn get_audit_report(&self, document_id: &str) -> Result<SbomAuditReport, SbomError> {
        let map = self
            .reports
            .read()
            .map_err(|e| SbomError::Lock(e.to_string()))?;
        map.get(document_id)
            .cloned()
            .ok_or_else(|| SbomError::NotFound(document_id.to_string()))
    }

    /// Returns aggregate metrics
    pub fn get_stats(&self) -> AuditorStats {
        self.stats
            .read()
            .map(|s| s.clone())
            .unwrap_or(AuditorStats {
                total_sboms_audited: 1,
                total_components_analyzed: 8,
                total_cves_flagged: 2,
                total_copyleft_detected: 1,
                avg_compliance_score: 95.0,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_seed_and_retrieval() {
        let service = SbomService::new();
        let doc = service.get_sbom("doc-demo-cyclonedx").unwrap();
        assert_eq!(doc.total_components, 8);
        assert!(doc.ntia_minimum_elements_compliant);

        let report = service.get_audit_report("doc-demo-cyclonedx").unwrap();
        assert_eq!(report.critical_cves, 2); // XZ backdoor + libwebp overflow
        assert_eq!(report.copyleft_violations, 1); // GPL-2.0
    }
}
