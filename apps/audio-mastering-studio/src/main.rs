#![deny(unsafe_code)]
//! Offline AI Stem Splitter & Audio Mastering Studio Desktop GUI & CLI Entrypoint

use audio_mastering_studio::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "audio-mastering-studio",
    app_title:
        "AI Audio Mastering Studio — 100% Offline 4-Stem Separation, 5-Band EQ & EBU R128 Mastering",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1560,
    window_height: 940,
};

struct MasteringDesktopHandler;

impl DesktopIpcHandler for MasteringDesktopHandler {
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
                command: "list_projects",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_project",
                payload: json!({ "project_id": "proj-demo" }),
            },
            SelfTestCheck {
                command: "remaster_project",
                payload: json!({
                    "project_id": "proj-demo",
                    "config": {
                        "target_lufs": -14.0
                    }
                }),
            },
            SelfTestCheck {
                command: "update_eq_band",
                payload: json!({
                    "project_id": "proj-demo",
                    "band_index": 1,
                    "frequency_hz": 320.0,
                    "gain_db": 1.5,
                    "q_factor": 1.0
                }),
            },
            SelfTestCheck {
                command: "get_eq_curve",
                payload: json!({
                    "project_id": "proj-demo",
                    "num_points": 32
                }),
            },
            SelfTestCheck {
                command: "get_loudness_matched_audio",
                payload: json!({
                    "project_id": "proj-demo"
                }),
            },
            SelfTestCheck {
                command: "export_mastered_wav",
                payload: json!({
                    "project_id": "proj-demo",
                    "config": {
                        "bit_depth": "Bit24",
                        "apply_dither": true,
                        "normalize_true_peak": true
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
            "Offline AI Stem Splitter & Audio Mastering Studio v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: audio-mastering-studio [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, MasteringDesktopHandler) {
        eprintln!("Audio Mastering Studio desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
