#![forbid(unsafe_code)]
//! AI File Explorer Desktop Binary Entry Point

use ai_file_explorer::service::FileExplorerService;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let start_time = Instant::now();

    // Initialize structured logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .try_init();

    tracing::info!("Starting AI File Explorer (standalone, offline-first)...");

    let service = FileExplorerService::new()?;
    let elapsed = start_time.elapsed();

    tracing::info!(
        "AI File Explorer service initialized in {:.2}ms (SLA: <2,000ms)",
        elapsed.as_secs_f64() * 1000.0
    );

    // Initial check on current directory
    if let Ok(current_dir) = std::env::current_dir() {
        let listing = service.list_directory(&current_dir)?;
        tracing::info!(
            "Indexed working directory {:?}: {} items found",
            current_dir,
            listing.total_count
        );
    }

    Ok(())
}
