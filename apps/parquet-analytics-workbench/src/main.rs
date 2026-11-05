#![deny(unsafe_code)]
//! Local Database & Parquet Analytics Workbench Desktop GUI & CLI Entrypoint

use parquet_analytics_workbench::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "parquet-analytics-workbench",
    app_title: "Parquet Workbench — 100% Offline Local Data Query & Analytics",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1480,
    window_height: 920,
};

struct WorkbenchDesktopHandler;

impl DesktopIpcHandler for WorkbenchDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_workbench_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "seed_demo_dataset",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "list_datasets",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "query_dataset",
                payload: json!({
                    "query": {
                        "dataset_id": "demo-telemetry",
                        "filters": [],
                        "sort": null,
                        "search_query": null,
                        "limit": 10,
                        "offset": 0
                    }
                }),
            },
            SelfTestCheck {
                command: "compute_chart_data",
                payload: json!({
                    "chart": {
                        "dataset_id": "demo-telemetry",
                        "chart_type": "Bar",
                        "x_column": "datacenter",
                        "y_column": "cpu_percent",
                        "aggregation": "Avg",
                        "limit": 5
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
            "Local Database & Parquet Analytics Workbench v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: parquet-analytics-workbench [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, WorkbenchDesktopHandler) {
        eprintln!("Parquet Workbench desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
