#![deny(unsafe_code)]
//! Forensic Media Privacy Sanitizer Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use privacy_media_sanitizer::commands::dispatch_ipc_command;
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "privacy-media-sanitizer",
    app_title:
        "Privacy Media Sanitizer — Forensic Air-Gapped EXIF, GPS & Document Metadata Scrubber",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct SanitizerDesktopHandler;

impl DesktopIpcHandler for SanitizerDesktopHandler {
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
                command: "list_media",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_media_report",
                payload: json!({ "file_id": "FILE-IPHONE-PHOTO" }),
            },
            SelfTestCheck {
                command: "sanitize_file",
                payload: json!({ "file_id": "FILE-LEGAL-BRIEF" }),
            },
            SelfTestCheck {
                command: "get_sanitized_result",
                payload: json!({ "file_id": "FILE-LEGAL-BRIEF" }),
            },
            SelfTestCheck {
                command: "generate_privacy_certificate",
                payload: json!({ "file_id": "FILE-LEGAL-BRIEF" }),
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
            "Privacy Media Sanitizer v{}\n\
             Forensic Air-Gapped Media & Document Privacy Sanitizer: Strips EXIF/XMP/GPS, PDF metadata, and issues Ed25519 audit certificates\n\n\
             USAGE:\n\
               privacy-media-sanitizer [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = SanitizerDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
