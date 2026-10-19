#![forbid(unsafe_code)]
//! Universal Log Viewer Executable Entry Point

use platform_common::AppPaths;
use shared_logging::{info, init_file_logging, LoggingConfig};
use shared_telemetry::LocalTelemetry;
use std::sync::Arc;
use universal_log_viewer::{LogQuery, LogViewerService, WorkspaceStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Resolve application paths safely
    let paths = AppPaths::from_app_name("universal-log-viewer")?;
    paths.ensure_directories()?;

    // 2. Initialize non-blocking file logging
    let _guard = init_file_logging(&paths.log_dir, "ulv", &LoggingConfig::default())?;

    info!("Starting Universal Log Viewer v0.1.0...");

    // 3. Initialize workspace database and telemetry
    let db_path = paths.data_dir.join("workspaces.db");
    let workspace_store = Arc::new(WorkspaceStore::open(&db_path)?);
    let telemetry = LocalTelemetry::default(); // Telemetry disabled by default

    let service = LogViewerService::new(workspace_store, telemetry);

    // 4. Check CLI arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--file" && args.len() > 2 {
        let target_file = &args[2];
        info!("Opening target log file: {}", target_file);
        let info = service.open_file(target_file).await?;
        println!(
            "Opened log: {} | Lines: {} | Size: {} bytes",
            info.path, info.total_lines, info.file_size_bytes
        );

        // Preview initial 10 lines
        let preview = service.get_lines_slice(0, 10).await?;
        for line in preview {
            println!("{:>6} | {}", line.line_number + 1, line.text);
        }
    } else if args.len() > 1 && args[1] == "--search" && args.len() > 3 {
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
        println!("Universal Log Viewer v0.1.0 (Standalone Desktop Engine)");
        println!("Usage: universal-log-viewer --file <path> | --search <path> <query>");
        let metrics = service.get_system_metrics();
        println!(
            "System metrics: RSS RAM: {:.2} MB, CPU: {:.1}%",
            metrics.resident_set_size_bytes as f64 / (1024.0 * 1024.0),
            metrics.cpu_percentage
        );
    }

    Ok(())
}
