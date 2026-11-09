//! IPC Command Dispatcher for Air-Gapped Industrial Fieldbus Protocol Debugger.

use crate::models::BusProtocol;
use crate::service::DebuggerService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<DebuggerService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_service() -> Arc<DebuggerService> {
    SERVICE
        .get_or_init(|| Arc::new(DebuggerService::new()))
        .clone()
}

pub fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("modbus-can-industrial-debugger")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::ModbusCanIndustrialDebugger, &data_dir)
    })
}

/// Dispatches an IPC command formatted as JSON
pub fn dispatch_ipc_command(action: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match action {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "list_sessions" => {
            let list = service.list_sessions();
            Ok(json!(list))
        }

        "get_session" => {
            let session_id = payload
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = service.get_session(session_id);
            Ok(json!(session))
        }

        "decode_raw_frame" => {
            let proto_str = payload
                .get("protocol")
                .and_then(|v| v.as_str())
                .unwrap_or("modbus_rtu");
            let protocol = match proto_str {
                "modbus_tcp" => BusProtocol::ModbusTcp,
                "can_20a" => BusProtocol::Can20A,
                "can_20b" => BusProtocol::Can20B,
                "can_fd" => BusProtocol::CanFd,
                _ => BusProtocol::ModbusRtu,
            };

            let raw_hex = payload
                .get("raw_hex")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing raw_hex".to_string())?;

            let is_request = payload
                .get("is_request")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let frame = service.decode_raw_frame(protocol, raw_hex, is_request)?;
            Ok(json!(frame))
        }

        "generate_audit_manifest" => {
            let session_id = payload
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let manifest = service.sign_audit_manifest(session_id)?;
            Ok(json!(manifest))
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

        _ => Err(format!("Unknown action: {}", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_get_stats() {
        let res = dispatch_ipc_command("get_stats", json!({})).unwrap();
        assert!(res["total_traces_captured"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn test_ipc_list_and_get_session() {
        let list_res = dispatch_ipc_command("list_sessions", json!({})).unwrap();
        let arr = list_res.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        let s_id = arr[0]["session_id"].as_str().unwrap();
        let get_res = dispatch_ipc_command("get_session", json!({ "session_id": s_id })).unwrap();
        assert_eq!(get_res["session_id"].as_str().unwrap(), s_id);
    }

    #[test]
    fn test_ipc_decode_raw_frame() {
        let res = dispatch_ipc_command(
            "decode_raw_frame",
            json!({
                "protocol": "modbus_rtu",
                "raw_hex": "01 03 00 00 00 02 C4 0B",
                "is_request": true
            }),
        )
        .unwrap();
        assert_eq!(res["protocol"].as_str().unwrap(), "modbus_rtu");
        assert!(res["modbus"].is_object());
    }

    #[test]
    fn test_ipc_generate_audit_manifest() {
        let res = dispatch_ipc_command(
            "generate_audit_manifest",
            json!({ "session_id": "SESSION-MODBUS-FACTORY" }),
        )
        .unwrap();
        assert_eq!(
            res["session_id"].as_str().unwrap(),
            "SESSION-MODBUS-FACTORY"
        );
        assert!(!res["ed25519_signature_hex"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_ipc_license_status() {
        let res = dispatch_ipc_command("get_license_status", json!({})).unwrap();
        assert!(res["tier"].is_string());
    }
}
