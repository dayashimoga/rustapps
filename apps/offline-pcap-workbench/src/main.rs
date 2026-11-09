#![deny(unsafe_code)]
//! High-Performance Air-Gapped Network Packet Analyzer Desktop GUI & CLI Entrypoint

use offline_pcap_workbench::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "offline-pcap-workbench",
    app_title:
        "Offline PCAP Workbench — High-Performance Zero-Copy Packet Analyzer & Stream Reassembly",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct PcapDesktopHandler;

impl DesktopIpcHandler for PcapDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_capture_summary",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_packets",
                payload: json!({ "filter": "", "offset": 0, "limit": 10 }),
            },
            SelfTestCheck {
                command: "get_tcp_flows",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_forensic_report",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_license_status",
                payload: json!({}),
            },
        ]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Offline PCAP Workbench v{}\n\
             100% Air-Gapped High-Performance Packet Analyzer, Stream Reassembly & JA3 TLS Fingerprinting\n\n\
             USAGE:\n\
               offline-pcap-workbench [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = PcapDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
