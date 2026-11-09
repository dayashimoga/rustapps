#![deny(unsafe_code)]
//! Offline QR Code Scanner & Generator Studio Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use qr_code_studio::commands::dispatch_ipc_command;
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "qr-code-studio",
    app_title: "QR Code Studio — 100% Offline Generator, Scanner & Anti-Quishing Inspector",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct QrStudioDesktopHandler;

impl DesktopIpcHandler for QrStudioDesktopHandler {
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
                command: "generate_qr",
                payload: json!({
                    "raw_text": "https://antigravity.internal/self-test",
                    "label": "Self-Test Verification QR",
                    "style": {
                        "ecc_level": "M",
                        "module_shape": "Rounded",
                        "finder_style": "Rounded",
                        "foreground_color": "#06b6d4",
                        "background_color": "#0a0e17",
                        "quiet_zone": 4
                    }
                }),
            },
            SelfTestCheck {
                command: "analyze_payload_security",
                payload: json!({
                    "payload": "https://paypal.com@phish.internal/login"
                }),
            },
            SelfTestCheck {
                command: "list_history",
                payload: json!({
                    "search_query": null,
                    "favorites_only": false,
                    "limit": 10
                }),
            },
            SelfTestCheck {
                command: "get_license_status",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "batch_generate_qr",
                payload: json!({
                    "items": [
                        {
                            "raw_text": "https://alpha.internal",
                            "label": "Self-Test Alpha",
                            "ecc": "M",
                            "color": "#06b6d4"
                        },
                        {
                            "raw_text": "WIFI:S:Guest;T:WPA;P:secret123;;",
                            "label": "Self-Test Wi-Fi",
                            "ecc": "H",
                            "color": "#10b981"
                        }
                    ]
                }),
            },
            SelfTestCheck {
                command: "export_label_sheet",
                payload: json!({
                    "items": [
                        { "payload": "https://label.internal/1", "label": "Label 1" }
                    ],
                    "format": "pdf"
                }),
            },
        ]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Offline QR Code Scanner & Generator Studio v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: qr-code-studio [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, QrStudioDesktopHandler) {
        eprintln!("QR Code Studio desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
