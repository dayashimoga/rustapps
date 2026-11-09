//! Service Layer for Visual Offline SQL Schema & Migration Diff Studio.
//!
//! Stores parsed schemas, generates differential migration plans,
//! and issues cryptographic migration integrity manifests.

use crate::diff_engine::DiffEngine;
use crate::models::{
    DatabaseSchema, MigrationIntegrityManifest, MigrationPlan, SchemaSummary, SqlDialect,
    StudioStats,
};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct SchemaService {
    schemas: RwLock<HashMap<String, DatabaseSchema>>,
    plans: RwLock<HashMap<String, MigrationPlan>>,
    signing_key: SigningKey,
}

impl Default for SchemaService {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaService {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let service = Self {
            schemas: RwLock::new(HashMap::new()),
            plans: RwLock::new(HashMap::new()),
            signing_key,
        };

        service.seed_sample_schemas();
        service
    }

    fn seed_sample_schemas(&self) {
        let v1_ddl = r#"
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    full_name VARCHAR(100),
    legacy_status VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_users_email ON users (email);

CREATE TABLE organizations (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(150) NOT NULL,
    slug VARCHAR(60) NOT NULL,
    plan_tier VARCHAR(30) DEFAULT 'free'
);

CREATE UNIQUE INDEX idx_orgs_slug ON organizations (slug);

CREATE TABLE deprecated_tokens (
    id BIGSERIAL PRIMARY KEY,
    token_str TEXT NOT NULL,
    revoked BOOLEAN DEFAULT false
);
"#;

        let v2_ddl = r#"
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    full_name VARCHAR(255),
    phone_number VARCHAR(30),
    is_mfa_enabled BOOLEAN DEFAULT false NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_users_email ON users (email);
CREATE INDEX idx_users_phone ON users (phone_number);

CREATE TABLE organizations (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(150) NOT NULL,
    slug VARCHAR(60) NOT NULL,
    plan_tier VARCHAR(30) DEFAULT 'free',
    emergency_contact VARCHAR(100) NOT NULL
);

CREATE UNIQUE INDEX idx_orgs_slug ON organizations (slug);

CREATE TABLE api_credentials (
    id BIGSERIAL PRIMARY KEY,
    org_id BIGINT NOT NULL,
    key_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_api_creds_org ON api_credentials (org_id);
"#;

        let s1 = DiffEngine::parse_ddl(
            "SCHEMA-PROD-V1",
            "Production Cloud Backend v1.4 (Baseline)",
            SqlDialect::PostgreSql,
            v1_ddl,
        );

        let s2 = DiffEngine::parse_ddl(
            "SCHEMA-STAGING-V2",
            "Staging Microservices v2.0 (Target)",
            SqlDialect::PostgreSql,
            v2_ddl,
        );

        let initial_plan = DiffEngine::compare_schemas(&s1, &s2);

        if let Ok(mut lock) = self.schemas.write() {
            lock.insert(s1.schema_id.clone(), s1);
            lock.insert(s2.schema_id.clone(), s2);
        }

        if let Ok(mut p_lock) = self.plans.write() {
            p_lock.insert(initial_plan.plan_id.clone(), initial_plan);
        }
    }

    /// Evaluates metrics across all registered schemas and migration plans
    pub fn get_stats(&self) -> StudioStats {
        let s_lock = match self.schemas.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let p_lock = match self.plans.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut breaking_count = 0;
        let mut safe_count = 0;

        for plan in p_lock.values() {
            breaking_count += plan.breaking_changes_count;
            safe_count += plan.safe_changes_count;
        }

        StudioStats {
            schemas_analyzed: s_lock.len(),
            migrations_generated: p_lock.len(),
            breaking_hazards_prevented: breaking_count,
            safe_online_deployments: safe_count,
        }
    }

    /// Returns list of summaries for all parsed database schemas
    pub fn list_schemas(&self) -> Vec<SchemaSummary> {
        let s_lock = match self.schemas.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut list: Vec<SchemaSummary> = s_lock
            .values()
            .map(|s| {
                let total_cols: usize = s.tables.iter().map(|t| t.columns.len()).sum();
                let total_idx: usize = s.tables.iter().map(|t| t.indexes.len()).sum();

                SchemaSummary {
                    schema_id: s.schema_id.clone(),
                    title: s.title.clone(),
                    dialect: s.dialect,
                    total_tables: s.tables.len(),
                    total_columns: total_cols,
                    total_indexes: total_idx,
                    created_at_epoch_ms: s.created_at_epoch_ms,
                }
            })
            .collect();

        list.sort_by_key(|b| std::cmp::Reverse(b.created_at_epoch_ms));
        list
    }

    /// Retrieves full parsed database schema by ID
    pub fn get_schema(&self, schema_id: &str) -> Option<DatabaseSchema> {
        let s_lock = match self.schemas.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if schema_id.is_empty() {
            return s_lock.values().next().cloned();
        }

        s_lock.get(schema_id).cloned()
    }

    /// Parses and registers a new DDL schema
    pub fn parse_schema(&self, title: &str, dialect: SqlDialect, raw_ddl: &str) -> DatabaseSchema {
        let schema_id = format!(
            "SCHEMA-{}",
            blake3::hash(format!("{}:{}", title, raw_ddl).as_bytes()).to_hex()[0..10]
                .to_uppercase()
        );

        let schema = DiffEngine::parse_ddl(&schema_id, title, dialect, raw_ddl);

        if let Ok(mut s_lock) = self.schemas.write() {
            s_lock.insert(schema.schema_id.clone(), schema.clone());
        }

        schema
    }

    /// Compares two schemas and synthesizes a reversible migration plan
    pub fn compare_schemas(&self, base_id: &str, target_id: &str) -> Result<MigrationPlan, String> {
        let s_lock = match self.schemas.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let base = s_lock
            .get(base_id)
            .ok_or_else(|| format!("Baseline schema '{}' not found", base_id))?;
        let target = s_lock
            .get(target_id)
            .ok_or_else(|| format!("Target schema '{}' not found", target_id))?;

        let plan = DiffEngine::compare_schemas(base, target);

        if let Ok(mut p_lock) = self.plans.write() {
            p_lock.insert(plan.plan_id.clone(), plan.clone());
        }

        Ok(plan)
    }

    /// Retrieves an existing migration plan
    pub fn get_migration_plan(&self, plan_id: &str) -> Option<MigrationPlan> {
        let p_lock = match self.plans.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if plan_id.is_empty() {
            return p_lock.values().next().cloned();
        }

        p_lock.get(plan_id).cloned()
    }

    /// Signs an Ed25519 Migration Integrity Manifest
    pub fn sign_migration_manifest(
        &self,
        plan_id: &str,
    ) -> Result<MigrationIntegrityManifest, String> {
        let p_lock = match self.plans.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let s_lock = match self.schemas.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let plan = if plan_id.is_empty() {
            p_lock
                .values()
                .next()
                .cloned()
                .ok_or_else(|| "No migration plans available".to_string())?
        } else {
            p_lock
                .get(plan_id)
                .cloned()
                .ok_or_else(|| format!("Migration plan '{}' not found", plan_id))?
        };

        let base_ddl = s_lock
            .get(&plan.baseline_schema_id)
            .map(|s| s.raw_ddl.as_str())
            .unwrap_or("");
        let target_ddl = s_lock
            .get(&plan.target_schema_id)
            .map(|s| s.raw_ddl.as_str())
            .unwrap_or("");

        Ok(DiffEngine::sign_manifest(
            &plan,
            base_ddl,
            target_ddl,
            &self.signing_key,
        ))
    }
}
