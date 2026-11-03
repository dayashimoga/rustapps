#![forbid(unsafe_code)]
//! Universal Log Viewer Desktop GUI & CLI Executable Entry Point

use platform_common::{
    run_desktop_app, AppPaths, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck,
};
use serde_json::{json, Value};
use shared_logging::{info, init_file_logging, LoggingConfig};
use shared_telemetry::LocalTelemetry;
use std::sync::Arc;
use universal_log_viewer::commands::dispatch_ipc_command;
use universal_log_viewer::{LogQuery, LogViewerService, WorkspaceStore};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "universal-log-viewer",
    app_title: "Universal Log Viewer — 100GB+ Zero-Copy Mmap & Drain AI Log Studio",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1460,
    window_height: 920,
};

struct LogViewerDesktopHandler;

impl DesktopIpcHandler for LogViewerDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "open_file",
                payload: json!({ "path": "default" }),
            },
            SelfTestCheck {
                command: "get_lines_slice",
                payload: json!({ "startLine": 0, "count": 25 }),
            },
            SelfTestCheck {
                command: "get_lines_by_indices",
                payload: json!({ "lineNumbers": [0, 1, 2] }),
            },
            SelfTestCheck {
                command: "get_timeline_histogram",
                payload: json!({ "targetBuckets": 30 }),
            },
            SelfTestCheck {
                command: "filter_by_timerange",
                payload: json!({ "startMs": 0, "endMs": 9999999999999i64 }),
            },
            SelfTestCheck {
                command: "mine_templates",
                payload: json!({ "maxRecords": 5000 }),
            },
            SelfTestCheck {
                command: "detect_anomalies",
                payload: json!({ "targetBuckets": 30, "zThreshold": 2.0 }),
            },
            SelfTestCheck {
                command: "cluster_errors",
                payload: json!({ "maxRecords": 5000 }),
            },
            SelfTestCheck {
                command: "search",
                payload: json!({
                    "query": {
                        "pattern": "ERROR",
                        "is_regex": false,
                        "case_sensitive": true,
                        "limit": 20,
                        "start_line": 0
                    }
                }),
            },
            SelfTestCheck {
                command: "get_system_metrics",
                payload: json!({}),
            },
        ]
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("universal-log-viewer v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Universal Log Viewer v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: universal-log-viewer [OPTIONS]");
        println!("  (no args)                 Launch standalone desktop Universal Log Viewer GUI");
        println!("  --self-test               Verify embedded UI assets and live IPC commands");
        println!(
            "  --headless-server         Start loopback GUI/IPC server without browser window"
        );
        println!("  --file <path>             Open and preview log file in CLI mode");
        println!("  --search <path> <query>   Search log file in CLI mode");
        println!("  --cli                     Show CLI metrics summary");
        println!("  --version, -v             Show version");
        return Ok(());
    }

    // Check CLI modes (--file, --search, --cli)
    let is_cli = args
        .iter()
        .any(|a| a == "--file" || a == "--search" || a == "--cli");

    if is_cli {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        return rt.block_on(async move {
            let paths = AppPaths::from_app_name("universal-log-viewer")?;
            paths.ensure_directories()?;
            let _guard = init_file_logging(&paths.log_dir, "ulv", &LoggingConfig::default())?;
            info!("Starting Universal Log Viewer v0.1.0 CLI...");

            let db_path = paths.data_dir.join("workspaces.db");
            let workspace_store = Arc::new(WorkspaceStore::open(&db_path)?);
            let telemetry = LocalTelemetry::default();
            let service = LogViewerService::new(workspace_store, telemetry);

            if args.len() > 2 && args[1] == "--file" {
                let target_file = &args[2];
                let info = service.open_file(target_file).await?;
                println!(
                    "Opened log: {} | Lines: {} | Size: {} bytes",
                    info.path, info.total_lines, info.file_size_bytes
                );
                let preview = service.get_lines_slice(0, 10).await?;
                for line in preview {
                    println!("{:>6} | {}", line.line_number + 1, line.text);
                }
            } else if args.len() > 3 && args[1] == "--search" {
                let target_file = &args[2];
                let query_str = &args[3];
                service.open_file(target_file).await?;
                let report = service
                    .search(LogQuery {
                        pattern: query_str.to_string(),
                        is_regex: false,
                        case_sensitive: false,
                        limit: 20,
                        start_line: 0,
                    })
                    .await?;
                println!(
                    "Found {} matches in {} µs (SLA < 50,000 µs)",
                    report.matches.len(),
                    report.elapsed_micros
                );
                for m in report.matches {
                    println!("{:>6} | {}", m.line_number + 1, m.line_content);
                }
            } else {
                let metrics = service.get_system_metrics();
                println!(
                    "System metrics: RSS RAM: {:.2} MB, CPU: {:.1}%",
                    metrics.resident_set_size_bytes as f64 / (1024.0 * 1024.0),
                    metrics.cpu_percentage
                );
            }
            Ok(())
        });
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, LogViewerDesktopHandler) {
        anyhow::bail!("Universal Log Viewer desktop runtime error: {}", e);
    }

    Ok(())
}
