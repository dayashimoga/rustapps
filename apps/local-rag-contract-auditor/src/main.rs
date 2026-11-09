#![deny(unsafe_code)]
//! Local RAG Contract Auditor Desktop GUI & CLI Entrypoint

use local_rag_contract_auditor::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "local-rag-contract-auditor",
    app_title:
        "Local RAG Contract Auditor — Offline AI Legal Reviewer, Clause Risk Scorer & Redlining Studio",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct ContractAuditorDesktopHandler;

impl DesktopIpcHandler for ContractAuditorDesktopHandler {
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
                command: "list_contracts",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_contract",
                payload: json!({ "document_id": "DOC-APEX-MSA-32" }),
            },
            SelfTestCheck {
                command: "audit_contract_text",
                payload: json!({
                    "title": "Self-Test Vendor Agreement",
                    "raw_text": "Section 1.0 Liability\nVendor liability shall be capped at zero dollars.",
                    "contract_type": "vendor_procurement"
                }),
            },
            SelfTestCheck {
                command: "compare_contracts",
                payload: json!({
                    "base_id": "DOC-CLEAN-PLAYBOOK-10",
                    "revised_id": "DOC-APEX-MSA-32"
                }),
            },
            SelfTestCheck {
                command: "generate_audit_certificate",
                payload: json!({ "document_id": "DOC-APEX-MSA-32" }),
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
            "Local RAG Contract Auditor v{}\n\
             Air-Gapped Offline Legal & Compliance RAG Contract Reviewer with Clause Risk Scoring, Redlining & Audit Signatures\n\n\
             USAGE:\n\
               local-rag-contract-auditor [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = ContractAuditorDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
