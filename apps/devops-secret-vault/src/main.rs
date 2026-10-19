#![forbid(unsafe_code)]
//! DevOps Secret Vault - Standalone Application Entry Point

use std::path::PathBuf;
use tracing::info;
use vault_core::models::calculate_entropy;
use vault_core::service::VaultService;
use vault_core::storage::VaultStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!(
        "DevOps Secret Vault v{} initializing...",
        env!("CARGO_PKG_VERSION")
    );

    let db_path = std::env::var("VAULT_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("devops_vault.db"));

    info!("Using vault database path: {:?}", db_path);
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

    // Demonstration of entropy calculation
    let sample_strength = calculate_entropy("Sample-P@ssw0rd-2026!");
    info!(
        "Sample password strength rating: {:?}",
        sample_strength.rating
    );

    info!("DevOps Secret Vault operational. Ready for desktop UI & FFI callers.");
    Ok(())
}
