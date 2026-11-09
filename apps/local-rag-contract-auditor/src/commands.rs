//! IPC Command Dispatcher for Offline Local RAG Contract & Legal Compliance Auditor.

use crate::models::ContractType;
use crate::service::ContractService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<ContractService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_service() -> Arc<ContractService> {
    SERVICE
        .get_or_init(|| Arc::new(ContractService::new()))
        .clone()
}

pub fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("local-rag-contract-auditor")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::LocalRagContractAuditor, &data_dir)
    })
}

/// Dispatches IPC command payload strings from desktop GUI / Tauri / FFI
pub fn dispatch_ipc_command(action: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match action {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "list_contracts" => {
            let list = service.list_contracts();
            Ok(json!(list))
        }

        "get_contract" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let doc = service.get_contract(doc_id);
            Ok(json!(doc))
        }

        "audit_contract_text" => {
            let title = payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Agreement");
            let raw_text = payload
                .get("raw_text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing raw_text payload".to_string())?;

            let contract_type = payload
                .get("contract_type")
                .and_then(|v| v.as_str())
                .map(|t| match t {
                    "saas_master_services_agreement" => ContractType::SaaSMasterServicesAgreement,
                    "non_disclosure_agreement" => ContractType::NonDisclosureAgreement,
                    "data_processing_agreement" => ContractType::DataProcessingAgreement,
                    "vendor_procurement" => ContractType::VendorProcurement,
                    "employment_agreement" => ContractType::EmploymentAgreement,
                    _ => ContractType::Other,
                })
                .unwrap_or(ContractType::SaaSMasterServicesAgreement);

            let doc = service.audit_contract_text(title, contract_type, raw_text);
            Ok(json!(doc))
        }

        "compare_contracts" => {
            let base_id = payload
                .get("base_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing base_id".to_string())?;
            let revised_id = payload
                .get("revised_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing revised_id".to_string())?;

            let diff = service.compare_contracts(base_id, revised_id)?;
            Ok(json!(diff))
        }

        "generate_audit_certificate" => {
            let doc_id = payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing document_id".to_string())?;

            let cert = service.sign_audit_certificate(doc_id)?;
            Ok(json!(cert))
        }

        "get_license_status" => {
            let report: LicenseStatusReport = licensing.status_report();
            Ok(json!(report))
        }

        "apply_license_key" => {
            let key = payload
                .get("license_key")
                .or_else(|| payload.get("key"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing license_key".to_string())?;
            let report = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        _ => Err(format!(
            "Unknown contract auditor IPC command: '{}'",
            action
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_commands_end_to_end() {
        // 1. Get stats
        let stats_val = dispatch_ipc_command("get_stats", json!({})).expect("get_stats");
        assert!(stats_val.get("total_contracts_audited").is_some());
        assert!(stats_val["total_contracts_audited"].as_u64().unwrap() >= 2);

        // 2. List contracts
        let list_val = dispatch_ipc_command("list_contracts", json!({})).expect("list_contracts");
        assert!(list_val.as_array().unwrap().len() >= 2);

        // 3. Get contract
        let doc_val =
            dispatch_ipc_command("get_contract", json!({ "document_id": "DOC-APEX-MSA-32" }))
                .expect("get_contract");
        assert_eq!(doc_val["document_id"], "DOC-APEX-MSA-32");
        assert!(doc_val["clauses"].as_array().unwrap().len() > 5);

        // 4. Audit new text
        let sample =
            "Section 1.0 Non-Compete\nCustomer shall never compete with Vendor in perpetuity.";
        let new_doc = dispatch_ipc_command(
            "audit_contract_text",
            json!({
                "title": "Vendor Non-Compete Addendum",
                "raw_text": sample,
                "contract_type": "vendor_procurement"
            }),
        )
        .expect("audit_contract_text");
        assert_eq!(new_doc["title"], "Vendor Non-Compete Addendum");

        // 5. Compare contracts
        let diff_val = dispatch_ipc_command(
            "compare_contracts",
            json!({
                "base_id": "DOC-CLEAN-PLAYBOOK-10",
                "revised_id": "DOC-APEX-MSA-32"
            }),
        )
        .expect("compare_contracts");
        assert!(!diff_val["diff_items"].as_array().unwrap().is_empty());

        // 6. Sign Certificate
        let cert_val = dispatch_ipc_command(
            "generate_audit_certificate",
            json!({ "document_id": "DOC-APEX-MSA-32" }),
        )
        .expect("generate_audit_certificate");
        assert!(cert_val["ed25519_signature_hex"].as_str().is_some());

        // 7. License Status
        let lic_val =
            dispatch_ipc_command("get_license_status", json!({})).expect("get_license_status");
        assert!(lic_val.get("tier").is_some());
    }
}
