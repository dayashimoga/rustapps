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
                command: "add_zoom_keyframe",
                payload: json!({
                    "session_id": "rec-demo",
                    "keyframe": {
                        "start_ms": 11_000,
                        "end_ms": 11_800,
                        "focus_x": 0.5,
                        "focus_y": 0.5,
                        "scale": 1.7,
                        "easing": "ease-in-out",
                        "label": "Self-Test Zoom"
                    }
                }),
            },
            SelfTestCheck {
                command: "delete_zoom_keyframe",
                payload: json!({
                    "session_id": "rec-demo",
                    "start_ms": 11_000
                }),
            },
            SelfTestCheck {
                command: "detect_pii_redactions",
                payload: json!({
                    "session_id": "rec-demo",
                    "text_regions": [
                        {
                            "text": "AWS Key: AKIAIOSFODNN7EXAMPLE",
                            "x": 0.15,
                            "y": 0.25,
                            "width": 0.35,
                            "height": 0.05
                        }
                    ]
                }),
            },
            SelfTestCheck {
                command: "add_redaction_region",
                payload: json!({
                    "session_id": "rec-demo",
                    "region": {
                        "id": "redact-self-test-1",
                        "label": "Self-Test Redact",
                        "x": 0.3,
                        "y": 0.3,
                        "width": 0.2,
                        "height": 0.1,
                        "pattern_type": "custom"
                    }
                }),
            },
            SelfTestCheck {
                command: "delete_redaction_region",
                payload: json!({
                    "session_id": "rec-demo",
                    "region_id": "redact-self-test-1"
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
