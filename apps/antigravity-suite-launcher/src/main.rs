#![deny(unsafe_code)]
//! Antigravity Suite Launcher Desktop GUI & CLI Entrypoint

use antigravity_suite_launcher::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "antigravity-suite-launcher",
    app_title: "Antigravity — Offline Enterprise Multi-App Suite & Command Center",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct LauncherDesktopHandler;

impl DesktopIpcHandler for LauncherDesktopHandler {
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
                command: "list_apps",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_app",
                payload: json!({ "app_id": "devops-secret-vault" }),
            },
            SelfTestCheck {
                command: "get_app",
                payload: json!({ "app_id": "universal-log-viewer" }),
            },
            SelfTestCheck {
                command: "run_diagnostic",
                payload: json!({ "app_id": "universal-log-viewer" }),
            },
            SelfTestCheck {
                command: "sign_suite_manifest",
                payload: json!({}),
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
            "Antigravity Suite Launcher v{}\n\
             Centralized Command Center & Diagnostic Health Monitor for all 23 Offline Flagship Applications\n\n\
             USAGE:\n\
               antigravity-suite-launcher [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = LauncherDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
