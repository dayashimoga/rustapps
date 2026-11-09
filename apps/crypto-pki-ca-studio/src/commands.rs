//! IPC Command Dispatcher for Crypto PKI & CA Studio.

use crate::models::CertType;
use crate::pki_service::PkiService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<PkiService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<PkiService> {
    SERVICE.get_or_init(|| Arc::new(PkiService::new())).clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("crypto-pki-ca-studio")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::CryptoPkiCaStudio, &data_dir)
    })
}

/// Dispatches an IPC command formatted as JSON
pub fn dispatch_ipc_command(command: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match command {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "get_hierarchy" => {
            let list = service.get_hierarchy();
            Ok(json!(list))
        }

        "get_certificate_detail" => {
            let cert_id = payload
                .get("certificate_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match service.get_certificate_detail(cert_id) {
                Some(cert) => Ok(json!(cert)),
                None => Err("Certificate not found".to_string()),
            }
        }

        "sign_csr" => {
            let ca_id = payload
                .get("ca_id")
                .and_then(|v| v.as_str())
                .unwrap_or("ca-sub-apex-1a");
            let subject_cn = payload
                .get("common_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing common_name".to_string())?;

            let cert_type_str = payload
                .get("cert_type")
                .and_then(|v| v.as_str())
                .unwrap_or("tls_server");
            let cert_type = match cert_type_str {
                "tls_client" => CertType::TlsClient,
                "code_signing" => CertType::CodeSigning,
                _ => CertType::TlsServer,
            };

            let sans: Vec<String> = payload
                .get("sans")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(|v| v.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let validity_days = payload
                .get("validity_days")
                .and_then(|v| v.as_u64())
                .unwrap_or(365) as u32;

            let cert = service.sign_csr(ca_id, subject_cn, cert_type, &sans, validity_days)?;
            Ok(json!(cert))
        }

        "revoke_certificate" => {
            let cert_id = payload
                .get("certificate_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing certificate_id".to_string())?;
            let reason = payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("KeyCompromise");

            let cert = service.revoke_certificate(cert_id, reason)?;
            Ok(json!(cert))
        }

        "get_crls" => {
            let list = service.get_crls();
            Ok(json!(list))
        }

        "get_shamir_ceremonies" => {
            let list = service.get_shamir_ceremonies();
            Ok(json!(list))
        }

        "reconstruct_shamir_key" => {
            let raw_shards = payload
                .get("shards")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Missing 'shards' array".to_string())?;

            let threshold_m = payload
                .get("threshold_m")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as usize;

            let mut shards = Vec::new();
            for s in raw_shards {
                let idx = s
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| "Missing shard index".to_string())?
                    as u8;
                let hex_str = s
                    .get("hex")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing shard hex".to_string())?
                    .to_string();
                shards.push((idx, hex_str));
            }

            let reconstructed_hex = service.reconstruct_shamir_key(&shards, threshold_m)?;
            Ok(json!({
                "success": true,
                "reconstructed_key_hex": reconstructed_hex
            }))
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

        _ => Err(format!("Unknown crypto-pki-ca-studio command: {}", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_workflow_end_to_end() {
        // 1. Stats
        let stats_val = dispatch_ipc_command("get_stats", json!({})).expect("get_stats");
        assert!(stats_val.get("total_cas").is_some());

        // 2. Hierarchy
        let hier_val = dispatch_ipc_command("get_hierarchy", json!({})).expect("get_hierarchy");
        assert!(hier_val.as_array().unwrap().len() >= 3);

        // 3. Detail
        let detail_val = dispatch_ipc_command("get_certificate_detail", json!({})).expect("detail");
        assert!(detail_val.get("serial_number_hex").is_some());

        // 4. CRLs
        let crls_val = dispatch_ipc_command("get_crls", json!({})).expect("crls");
        assert!(!crls_val.as_array().unwrap().is_empty());

        // 5. Shamir Ceremonies
        let shamir_val = dispatch_ipc_command("get_shamir_ceremonies", json!({})).expect("shamir");
        assert!(!shamir_val.as_array().unwrap().is_empty());

        // 6. Sign CSR
        let sign_val = dispatch_ipc_command(
            "sign_csr",
            json!({
                "ca_id": "ca-sub-apex-1a",
                "common_name": "vault.internal.corp",
                "cert_type": "tls_server",
                "sans": ["DNS:vault.internal.corp"],
                "validity_days": 180
            }),
        )
        .expect("sign_csr");
        assert_eq!(sign_val["subject"]["common_name"], "vault.internal.corp");

        // 7. Revoke
        let rev_val = dispatch_ipc_command(
            "revoke_certificate",
            json!({
                "certificate_id": sign_val["certificate_id"].as_str().unwrap(),
                "reason": "Superseded"
            }),
        )
        .expect("revoke");
        assert!(rev_val["is_revoked"].as_bool().unwrap());
    }
}
