#![forbid(unsafe_code)]
//! AI File Explorer Desktop GUI & CLI Binary Entry Point

use ai_file_explorer::{commands::dispatch_ipc_command, service::FileExplorerService};
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};
use std::time::Instant;

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "ai-file-explorer",
    app_title: "AI File Explorer — Offline Semantic Search, OCR & Compliance Studio",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 920,
};

struct ExplorerDesktopHandler;

impl DesktopIpcHandler for ExplorerDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        let base = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let target_dir = if base.join("docs").is_dir() {
            base.join("docs")
        } else {
            base
        };
        let cwd = target_dir.to_string_lossy().to_string();
        vec![
            SelfTestCheck {
                command: "list_directory_cmd",
                payload: json!({ "path": cwd }),
            },
            SelfTestCheck {
                command: "search_files_cmd",
                payload: json!({
                    "query": {
                        "query": "Cargo",
                        "mode": "Keyword",
                        "limit": 10
                    }
                }),
            },
            SelfTestCheck {
                command: "search_content_cmd",
                payload: json!({
                    "query": {
                        "query": "encryption",
                        "mode": "Combined",
                        "limit": 10,
                        "extension_filter": null
                    }
                }),
            },
            SelfTestCheck {
                command: "analyze_storage_cmd",
                payload: json!({ "path": cwd, "top_limit": 5 }),
            },
            SelfTestCheck {
                command: "get_treemap_cmd",
                payload: json!({ "path": cwd, "width": 800.0, "height": 600.0, "max_depth": 1 }),
            },
            SelfTestCheck {
                command: "scan_compliance_cmd",
                payload: json!({ "path": cwd }),
            },
            SelfTestCheck {
                command: "scan_screenshots_dir_cmd",
                payload: json!({ "path": cwd }),
            },
            SelfTestCheck {
                command: "search_screenshots_by_text_cmd",
                payload: json!({ "query": "error", "limit": 5 }),
            },
        ]
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("ai-file-explorer v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("AI File Explorer v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: ai-file-explorer [OPTIONS]");
        println!("  (no args)           Launch standalone desktop AI File Explorer GUI");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --cli               Execute headless CLI indexing diagnostics");
        println!("  --version, -v       Show version");
        return Ok(());
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .try_init();

    if args.iter().any(|a| a == "--cli") {
        let start_time = Instant::now();
        tracing::info!("Starting AI File Explorer CLI diagnostics (standalone, offline-first)...");
        let service = FileExplorerService::new()?;
        let elapsed = start_time.elapsed();
        tracing::info!(
            "AI File Explorer service initialized in {:.2}ms (SLA: <2,000ms)",
            elapsed.as_secs_f64() * 1000.0
        );
        if let Ok(current_dir) = std::env::current_dir() {
            let listing = service.list_directory(&current_dir)?;
            tracing::info!(
                "Indexed working directory {:?}: {} items found",
                current_dir,
                listing.total_count
            );
        }
        return Ok(());
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, ExplorerDesktopHandler) {
        anyhow::bail!("AI File Explorer desktop runtime error: {}", e);
    }

    Ok(())
}
