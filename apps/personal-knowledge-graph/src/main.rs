#![deny(unsafe_code)]
//! Personal Knowledge Graph & Research Studio Desktop GUI & CLI Entrypoint

use personal_knowledge_graph::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "personal-knowledge-graph",
    app_title: "Research Studio — Offline Personal Knowledge Graph",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct KnowledgeDesktopHandler;

impl DesktopIpcHandler for KnowledgeDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_graph_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_graph_visualization",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "save_note",
                payload: json!({
                    "title": "Quantum Error Correction",
                    "content": "Surface codes and [[Toric Code]] research. #quantum [@kitaev2003]",
                    "tags": ["physics"]
                }),
            },
            SelfTestCheck {
                command: "search_knowledge",
                payload: json!({
                    "query": "quantum",
                    "semantic": false
                }),
            },
            SelfTestCheck {
                command: "search_knowledge",
                payload: json!({
                    "query": "quantum physics",
                    "semantic": true
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
            "Offline Personal Knowledge Graph & Research Studio v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: personal-knowledge-graph [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, KnowledgeDesktopHandler) {
        eprintln!("Personal Knowledge Graph desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
