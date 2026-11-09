#![deny(unsafe_code)]
//! Visual SQL Schema & Migration Diff Studio Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};
use sql_migration_diff_studio::commands::dispatch_ipc_command;

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "sql-migration-diff-studio",
    app_title:
        "SQL Migration & Diff Studio — Visual Air-Gapped Database Schema Differ & Safe Migration Generator",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1440,
    window_height: 900,
};

struct SqlDiffDesktopHandler;

impl DesktopIpcHandler for SqlDiffDesktopHandler {
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
                command: "list_schemas",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_schema",
                payload: json!({ "schema_id": "SCHEMA-PROD-V1" }),
            },
            SelfTestCheck {
                command: "parse_schema",
                payload: json!({
                    "title": "Self-Test Users Schema",
                    "dialect": "postgresql",
                    "raw_ddl": "CREATE TABLE test_users ( id BIGSERIAL PRIMARY KEY, name VARCHAR(50) NOT NULL );"
                }),
            },
            SelfTestCheck {
                command: "compare_schemas",
                payload: json!({
                    "base_id": "SCHEMA-PROD-V1",
                    "target_id": "SCHEMA-STAGING-V2"
                }),
            },
            SelfTestCheck {
                command: "generate_integrity_manifest",
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
            "SQL Migration & Diff Studio v{}\n\
             Air-Gapped Visual Database Schema Differ, Safe Migration Generator & Backward Compatibility Linter\n\n\
             USAGE:\n\
               sql-migration-diff-studio [FLAGS]\n\n\
             FLAGS:\n\
               --self-test    Run automated IPC integration self-test and verify embedded UI bundle\n\
               --help, -h     Show this help screen\n",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let handler = SqlDiffDesktopHandler;
    let _ = run_desktop_app(UI_BUNDLE, handler);
}
