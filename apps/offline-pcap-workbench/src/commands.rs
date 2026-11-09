//! IPC Command Dispatcher for Offline PCAP Workbench.

use crate::service::PcapService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<PcapService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

fn get_service() -> Arc<PcapService> {
    SERVICE.get_or_init(|| Arc::new(PcapService::new())).clone()
}

fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("offline-pcap-workbench")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::OfflinePcapWorkbench, &data_dir)
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

        "get_capture_summary" => {
            let capture_id = payload
                .get("capture_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match service.get_capture_summary(capture_id) {
                Some(s) => Ok(json!(s)),
                None => Err("No capture found".to_string()),
            }
        }

        "get_packets" => {
            let capture_id = payload
                .get("capture_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let filter = payload.get("filter").and_then(|v| v.as_str());
            let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let limit = payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

            let (packets, total) = service.get_packets(capture_id, filter, offset, limit);
            Ok(json!({
                "packets": packets,
                "total": total,
                "offset": offset,
                "limit": limit
            }))
        }

        "get_packet_detail" => {
            let capture_id = payload
                .get("capture_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let packet_index = payload
                .get("packet_index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Missing packet_index".to_string())?
                as usize;

            match service.get_packet_detail(capture_id, packet_index) {
                Some((packet, hex_dump)) => Ok(json!({
                    "packet": packet,
                    "hex_dump": hex_dump
                })),
                None => Err("Packet not found".to_string()),
            }
        }

        "get_tcp_flows" => {
            let capture_id = payload
                .get("capture_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let flows = service.get_tcp_flows(capture_id);
            Ok(json!(flows))
        }

        "get_stream_waterfall" => {
            let capture_id = payload
                .get("capture_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let stream_id = payload
                .get("stream_id")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;

            let events = service.get_stream_waterfall(capture_id, stream_id);
            Ok(json!(events))
        }

        "get_forensic_report" => {
            let capture_id = payload
                .get("capture_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match service.get_forensic_report(capture_id) {
                Some(r) => Ok(json!(r)),
                None => Err("No report available".to_string()),
            }
        }

        "parse_pcap_file" => {
            let filename = payload
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("uploaded.pcap");

            let raw_bytes =
                if let Some(hex_str) = payload.get("content_hex").and_then(|v| v.as_str()) {
                    hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?
                } else if let Some(b64) = payload.get("content_base64").and_then(|v| v.as_str()) {
                    hex::decode(b64).unwrap_or_else(|_| b64.as_bytes().to_vec())
                } else {
                    return Err("Missing 'content_hex' or 'content_base64'".to_string());
                };

            let summary = service.parse_uploaded_capture(filename, &raw_bytes)?;
            Ok(json!(summary))
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
                .ok_or_else(|| "Missing 'license_key' field".to_string())?;

            let report = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        _ => Err(format!(
            "Unknown offline-pcap-workbench command: {}",
            command
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_workflow_end_to_end() {
        // 1. Stats
        let stats_val = dispatch_ipc_command("get_stats", json!({})).expect("get_stats");
        assert!(stats_val.get("total_packets_decoded").is_some());

        // 2. Summary
        let sum_val = dispatch_ipc_command("get_capture_summary", json!({})).expect("summary");
        assert_eq!(sum_val["total_packets"], 7);

        // 3. Packets
        let pkts_val = dispatch_ipc_command(
            "get_packets",
            json!({
                "filter": "DNS",
                "offset": 0,
                "limit": 10
            }),
        )
        .expect("get_packets");
        assert_eq!(pkts_val["packets"].as_array().unwrap().len(), 2);

        // 4. Packet Detail
        let detail_val = dispatch_ipc_command(
            "get_packet_detail",
            json!({
                "packet_index": 1
            }),
        )
        .expect("detail");
        assert!(detail_val.get("hex_dump").is_some());

        // 5. TCP flows
        let flows_val = dispatch_ipc_command("get_tcp_flows", json!({})).expect("flows");
        assert!(!flows_val.as_array().unwrap().is_empty());

        // 6. Forensic Report
        let report_val = dispatch_ipc_command("get_forensic_report", json!({})).expect("forensic");
        assert_eq!(report_val["cleartext_credentials_found"], 1);
        assert!(!report_val["signature_hex"].as_str().unwrap().is_empty());
    }
}
