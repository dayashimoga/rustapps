#![deny(unsafe_code)]
//! Markdown Publishing Press Desktop GUI & CLI Entrypoint

use markdown_publishing_press::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "markdown-publishing-press",
    app_title:
        "Markdown Publishing Press — Academic Typst/LaTeX Typesetter & Copyright Certificate Issuer",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct PressDesktopHandler;

impl DesktopIpcHandler for PressDesktopHandler {
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
                command: "list_projects",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_project",
                payload: json!({ "project_id": "PROJECT-QUANTUM-ALGO" }),
            },
            SelfTestCheck {
                command: "get_artifact",
                payload: json!({ "project_id": "PROJECT-QUANTUM-ALGO" }),
            },
            SelfTestCheck {
                command: "compile_project",
                payload: json!({
                    "project_id": "PROJECT-QUANTUM-ALGO",
                    "template": "acm_sigconf",
                    "format": "print_pdf"
                }),
            },
            SelfTestCheck {
                command: "sign_copyright_certificate",
                payload: json!({ "project_id": "PROJECT-QUANTUM-ALGO" }),
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
            "Markdown Publishing Press v{}\n\
             Air-Gapped Academic & Technical Typesetter (IEEE, ACM, Springer, Enterprise SOP) & Copyright Issuer\n\n\
             USAGE:\n\
               markdown-publishing-press [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = PressDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
