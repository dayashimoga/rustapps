//! Service Layer for Antigravity Offline Enterprise Multi-App Suite Launcher.

use crate::launcher_engine::LauncherEngine;
use crate::models::{
    AppCategory, AppMetadata, AppSelfTestResult, SuiteIntegrityManifest, SuiteStats,
};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::sync::RwLock;

pub struct LauncherService {
    catalog: RwLock<Vec<AppMetadata>>,
    signing_key: SigningKey,
}

impl Default for LauncherService {
    fn default() -> Self {
        Self::new()
    }
}

impl LauncherService {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let mut catalog = LauncherEngine::get_catalog();
        // Check actual disk presence for each app
        for app in &mut catalog {
            app.installed = LauncherEngine::check_binary_installed(&app.binary_name);
        }

        Self {
            catalog: RwLock::new(catalog),
            signing_key,
        }
    }

    /// Evaluates metrics across all registered applications
    pub fn get_stats(&self) -> SuiteStats {
        let lock = match self.catalog.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let installed_count = lock.iter().filter(|a| a.installed).count();

        SuiteStats {
            total_apps_registered: lock.len(),
            total_apps_installed: installed_count,
            suite_license_tier: "Enterprise Air-Gap".to_string(),
            air_gap_egress_blocks: 0,
        }
    }

    /// Lists applications optionally filtered by category
    pub fn list_apps(&self, category: Option<AppCategory>) -> Vec<AppMetadata> {
        let lock = match self.catalog.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        match category {
            Some(cat) => lock.iter().filter(|a| a.category == cat).cloned().collect(),
            None => lock.clone(),
        }
    }

    /// Retrieves app metadata by ID
    pub fn get_app(&self, app_id: &str) -> Option<AppMetadata> {
        let lock = match self.catalog.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        lock.iter().find(|a| a.id == app_id).cloned()
    }

    /// Executes diagnostic self-test check against an app
    pub fn run_diagnostic(&self, app_id: &str) -> AppSelfTestResult {
        LauncherEngine::run_app_diagnostic(app_id)
    }

    /// Issues an Ed25519-signed Suite Integrity Manifest
    pub fn sign_suite_manifest(&self) -> SuiteIntegrityManifest {
        let lock = match self.catalog.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        LauncherEngine::sign_suite_manifest(&lock, &self.signing_key)
    }
}
