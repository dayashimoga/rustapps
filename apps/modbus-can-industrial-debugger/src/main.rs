#![deny(unsafe_code)]
//! Industrial Fieldbus Protocol Debugger Desktop GUI & CLI Entrypoint

use modbus_can_industrial_debugger::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "modbus-can-industrial-debugger",
    app_title:
        "Industrial Fieldbus Protocol Debugger — Modbus RTU/TCP & CAN-FD Dissector & Security Monitor",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct DebuggerDesktopHandler;

impl DesktopIpcHandler for DebuggerDesktopHandler {
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
                command: "list_sessions",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_session",
                payload: json!({ "session_id": "SESSION-MODBUS-FACTORY" }),
            },
            SelfTestCheck {
                command: "get_session",
                payload: json!({ "session_id": "SESSION-CAN-AUTOMOTIVE" }),
            },
            SelfTestCheck {
                command: "decode_raw_frame",
                payload: json!({
                    "protocol": "modbus_rtu",
                    "raw_hex": "01 03 00 00 00 02 C4 0B",
                    "is_request": true
                }),
            },
            SelfTestCheck {
                command: "generate_audit_manifest",
                payload: json!({ "session_id": "SESSION-MODBUS-FACTORY" }),
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
            "Industrial Fieldbus Protocol Debugger v{}\n\
             Air-Gapped Modbus RTU/TCP, CAN 2.0B / CAN-FD Dissector & IEC 62443 Security Anomaly Monitor\n\n\
             USAGE:\n\
               modbus-can-industrial-debugger [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = DebuggerDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
