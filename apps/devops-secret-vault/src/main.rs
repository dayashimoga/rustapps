#![forbid(unsafe_code)]
//! DevOps Secret Vault - Standalone Desktop GUI & CLI Application Entry Point

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::info;
use vault_core::commands::dispatch_ipc_command;
use vault_core::models::calculate_entropy;
use vault_core::service::VaultService;
use vault_core::storage::VaultStorage;

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "devops-secret-vault",
    app_title: "DevOps Secret Vault — Zero-Knowledge AES-256-GCM & Shamir Key Vault",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1400,
    window_height: 880,
};

struct VaultDesktopHandler;

impl DesktopIpcHandler for VaultDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_vault_status",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "check_password_strength",
                payload: json!({ "password": "V@ult-Master-Key-2026!#Secure" }),
            },
            SelfTestCheck {
                command: "is_biometric_available",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_clipboard_remaining_seconds",
                payload: json!({}),
            },
        ]
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("devops-secret-vault v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("DevOps Secret Vault v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: devops-secret-vault [OPTIONS]");
        println!("  (no args)           Launch standalone desktop DevOps Secret Vault GUI");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --cli               Execute headless CLI vault diagnostics");
        println!("  --version, -v       Show version");
        return Ok(());
    }

    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    if args.iter().any(|a| a == "--cli") {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            info!(
                "DevOps Secret Vault v{} CLI diagnostics initializing...",
                env!("CARGO_PKG_VERSION")
            );
            let db_path = std::env::var("VAULT_DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("devops_vault.db"));
            let storage = VaultStorage::open(&db_path)?;
            let service = VaultService::new(storage, None);
            let is_initialized = service.is_initialized()?;
            info!(
                "Vault initialization status: {}",
                if is_initialized {
                    "INITIALIZED"
                } else {
                    "UNINITIALIZED"
                }
            );
            let sample_strength = calculate_entropy("Sample-P@ssw0rd-2026!");
            info!(
                "Sample password strength rating: {:?}",
                sample_strength.rating
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
        return Ok(());
    }

    run_desktop_app(UI_BUNDLE, VaultDesktopHandler)?;
    Ok(())
}
