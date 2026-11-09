#![deny(unsafe_code)]
//! Air-Gapped SBOM & Supply-Chain Auditor Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use sbom_supply_chain_auditor::commands::dispatch_ipc_command;
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "sbom-supply-chain-auditor",
    app_title: "SBOM & Supply-Chain Auditor — 100% Offline CycloneDX/SPDX Analysis, CVE Hunter & NTIA Compliance",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct SbomDesktopHandler;

impl DesktopIpcHandler for SbomDesktopHandler {
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
                command: "get_sbom",
                payload: json!({ "document_id": "doc-demo-cyclonedx" }),
            },
            SelfTestCheck {
                command: "get_audit_report",
                payload: json!({ "document_id": "doc-demo-cyclonedx" }),
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
            "Air-Gapped SBOM & Supply-Chain Auditor v{}\n\
             100% Offline CycloneDX/SPDX Analysis, Copyleft Risk Detection & NTIA Compliance Verification\n\n\
             USAGE:\n\
               sbom-supply-chain-auditor [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = SbomDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
