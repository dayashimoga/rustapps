#![deny(unsafe_code)]
//! Offline Digital Forensics & Incident Response (DFIR) Triage Kit Desktop GUI & CLI Entrypoint

use dfir_triage_kit::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "dfir-triage-kit",
    app_title: "DFIR Triage Kit — 100% Offline Host Forensics & Incident Analysis",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1480,
    window_height: 920,
};

struct DfirDesktopHandler;

impl DesktopIpcHandler for DfirDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_dfir_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "list_triage_runs",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "execute_triage",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "scan_directory",
                payload: json!({ "path": ".", "max_depth": 1 }),
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
            "Offline Digital Forensics & Incident Response (DFIR) Triage Kit v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: dfir-triage-kit [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, DfirDesktopHandler) {
        eprintln!("DFIR Triage Kit desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
