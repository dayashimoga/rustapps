#![deny(unsafe_code)]
//! Air-Gapped API & Protocol Studio Desktop GUI & CLI Entrypoint

use api_protocol_studio::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "api-protocol-studio",
    app_title: "API Studio — 100% Offline Multi-Protocol Client & Fuzzer",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1480,
    window_height: 920,
};

struct StudioDesktopHandler;

impl DesktopIpcHandler for StudioDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_studio_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "list_collections",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "save_collection",
                payload: json!({
                    "collection": {
                        "id": "self-test-coll",
                        "name": "Integration Test Suite",
                        "description": "Auto-generated collection for self-test verification",
                        "variables": [{ "key": "baseUrl", "value": "http://127.0.0.1:8080", "enabled": true, "description": null }],
                        "created_at": 1700000000,
                        "updated_at": 1700000000
                    }
                }),
            },
            SelfTestCheck {
                command: "save_request",
                payload: json!({
                    "request": {
                        "id": "self-test-req",
                        "collection_id": "self-test-coll",
                        "name": "Healthcheck Endpoint",
                        "protocol": "HttpRest",
                        "method": "GET",
                        "url": "{{baseUrl}}/health",
                        "headers": [],
                        "query_params": [],
                        "auth": { "type": "None" },
                        "body": { "body_type": "none", "raw_content": "", "graphql_query": null, "graphql_variables": null },
                        "timeout_ms": 3000
                    }
                }),
            },
            SelfTestCheck {
                command: "list_requests",
                payload: json!({ "collection_id": "self-test-coll" }),
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
            "Air-Gapped API & Protocol Studio v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: api-protocol-studio [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, StudioDesktopHandler) {
        eprintln!("API Studio desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
