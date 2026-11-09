#![deny(unsafe_code)]
//! Visual Air-Gapped X.509 PKI Studio Desktop GUI & CLI Entrypoint

use crypto_pki_ca_studio::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "crypto-pki-ca-studio",
    app_title:
        "Crypto PKI & CA Studio — Visual Air-Gapped X.509 CA Builder & Shamir M-of-N Key Ceremony",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct PkiDesktopHandler;

impl DesktopIpcHandler for PkiDesktopHandler {
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
                command: "get_hierarchy",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_certificate_detail",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_crls",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_shamir_ceremonies",
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
            "Crypto PKI & CA Studio v{}\n\
             Visual Air-Gapped X.509 Certificate Authority Builder, Shamir M-of-N Ceremony & CRL Signer\n\n\
             USAGE:\n\
               crypto-pki-ca-studio [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = PkiDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
