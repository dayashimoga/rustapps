//! Data Models for Antigravity Offline Enterprise Multi-App Suite Launcher.

use serde::{Deserialize, Serialize};

/// Application Category Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    /// High-Security DevOps, Data & Forensics Core
    EnterpriseCore,
    /// High-Velocity Prosumer & Creator Tools
    CreatorMedia,
    /// Deep-Tech Fieldbus, RAG Legal & Protocol Engines
    IndustrialSpecialized,
}

impl AppCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::EnterpriseCore => "DevOps, Data & Security Core",
            Self::CreatorMedia => "Creator & Scientific Media",
            Self::IndustrialSpecialized => "Specialized Industrial & Legal",
        }
    }
}

/// Metadata Specification for a Managed Offline Application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub category: AppCategory,
    pub binary_name: String,
    pub version: String,
    pub installed: bool,
    pub cold_start_target_ms: u64,
    pub ram_idle_budget_mb: u64,
    pub air_gap_status: String,
    pub icon_glyph: String,
}

/// Executive Suite-Level Diagnostics & Inventory Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteStats {
    pub total_apps_registered: usize,
    pub total_apps_installed: usize,
    pub suite_license_tier: String,
    pub air_gap_egress_blocks: usize,
}

/// Diagnostic Self-Test Execution Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSelfTestResult {
    pub app_id: String,
    pub passed: bool,
    pub ipc_checks_passed: usize,
    pub ipc_checks_total: usize,
    pub message: String,
}

/// Cryptographically Signed Master Suite Integrity Manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteIntegrityManifest {
    pub manifest_id: String,
    pub suite_blake3_hash: String,
    pub total_apps: usize,
    pub signer_public_key_hex: String,
    pub ed25519_signature_hex: String,
    pub issued_at_epoch_ms: i64,
}
