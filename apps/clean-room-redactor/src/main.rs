#![deny(unsafe_code)]
//! Offline Clean-Room Document Redactor Desktop GUI & CLI Entrypoint

use clean_room_redactor::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "clean-room-redactor",
    app_title: "Clean-Room Document Redactor — 100% Offline Stream Burnout, PII Scrubber & Ed25519 Certificates",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct CleanRoomDesktopHandler;

impl DesktopIpcHandler for CleanRoomDesktopHandler {
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
                command: "get_document",
                payload: json!({ "document_id": "doc-demo-settlement" }),
            },
            SelfTestCheck {
                command: "add_redaction",
                payload: json!({
                    "document_id": "doc-demo-settlement",
                    "page_index": 0,
                    "label": "Settlement Amount Self-Test",
                    "burn_style": "exemption_code",
                    "exemption_code": "(b)(4)"
                }),
            },
            SelfTestCheck {
                command: "apply_clean_room_redaction",
                payload: json!({
                    "document_id": "doc-demo-settlement",
                    "bates_config": {
                        "prefix": "SELFTEST-",
                        "suffix": "",
                        "start_number": 1,
                        "digits": 6,
                        "position": "bottom_right",
                        "font_size": 10,
                        "include_date": false
                    }
                }),
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
            "Clean-Room Document Redactor v{}\n\
             100% Offline Deep Stream Burnout, PII Scrubber & Ed25519 Sanitization Certificates\n\n\
             USAGE:\n\
               clean-room-redactor [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = CleanRoomDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
