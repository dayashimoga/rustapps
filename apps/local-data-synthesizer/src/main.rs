#![deny(unsafe_code)]
//! Offline Local Synthetic Data & Anonymization Studio Desktop GUI & CLI Entrypoint

use local_data_synthesizer::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "local-data-synthesizer",
    app_title: "Local Synthetic Data Studio — 100% Offline Schema Cloning, Referential Integrity & Differential Privacy",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct SynthesizerDesktopHandler;

impl DesktopIpcHandler for SynthesizerDesktopHandler {
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
                command: "get_dataset_config",
                payload: json!({ "dataset_id": "ds-fintech-demo" }),
            },
            SelfTestCheck {
                command: "synthesize_dataset",
                payload: json!({ "dataset_id": "ds-fintech-demo" }),
            },
            SelfTestCheck {
                command: "get_table_preview",
                payload: json!({ "table_name": "customers", "limit": 5 }),
            },
            SelfTestCheck {
                command: "export_table_csv",
                payload: json!({ "table_name": "customers" }),
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
            "Local Synthetic Data & Anonymization Studio v{}\n\
             100% Offline Schema Cloning, Referential Integrity & Differential Privacy\n\n\
             USAGE:\n\
               local-data-synthesizer [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = SynthesizerDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
