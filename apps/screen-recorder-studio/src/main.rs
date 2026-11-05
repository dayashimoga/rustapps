#![deny(unsafe_code)]
//! Offline AI Screen Recorder & Interactive Tutorial Builder Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use screen_recorder_studio::commands::dispatch_ipc_command;
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "screen-recorder-studio",
    app_title:
        "Screen Recorder Studio — 100% Offline AI Capture, Auto-Zoom & Interactive Tutorials",
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
                command: "list_recordings",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_recording",
                payload: json!({ "session_id": "rec-demo" }),
            },
            SelfTestCheck {
                command: "adjust_auto_zooms",
                payload: json!({ "session_id": "rec-demo", "scale": 1.6 }),
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
            "Offline AI Screen Recorder & Interactive Tutorial Builder v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: screen-recorder-studio [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, StudioDesktopHandler) {
        eprintln!("Screen Recorder Studio desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
