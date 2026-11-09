//! Data models for Visual Offline SQL Schema & Migration Diff Studio.
//!
//! Models SQL dialects, tables, columns, indexes, foreign keys,
//! AST diff operations, safety linter risk levels, and Ed25519 migration manifests.

use serde::{Deserialize, Serialize};

/// Supported SQL Database Dialects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqlDialect {
    PostgreSql,
    MySql,
    Sqlite,
    Oracle,
    MsSqlServer,
}

impl SqlDialect {
    pub fn as_str(&self) -> &'static str {
        match self {
            SqlDialect::PostgreSql => "PostgreSQL",
            SqlDialect::MySql => "MySQL",
            SqlDialect::Sqlite => "SQLite",
            SqlDialect::Oracle => "Oracle",
            SqlDialect::MsSqlServer => "Microsoft SQL Server",
        }
    }
}

/// Column Definition in a Relational Table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_autoincrement: bool,
}

/// Foreign Key Relationship Definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignKeyDefinition {
    pub name: String,
    pub constrained_columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Index Definition on Table Columns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub index_type: String, // "BTREE", "GIN", "HASH"
}

/// Relational Table Schema
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKeyDefinition>,
    pub indexes: Vec<IndexDefinition>,
}

/// Full Database Schema snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub schema_id: String,
    pub title: String,
    pub dialect: SqlDialect,
    pub tables: Vec<TableDefinition>,
    pub raw_ddl: String,
    pub created_at_epoch_ms: i64,
}

/// Schema Summary for Navigation List
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSummary {
    pub schema_id: String,
    pub title: String,
    pub dialect: SqlDialect,
    pub total_tables: usize,
    pub total_columns: usize,
    pub total_indexes: usize,
    pub created_at_epoch_ms: i64,
}

/// Migration Operation Risk Severity Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationRiskLevel {
    SafeOnline,
    Warning,
    HazardousBreaking,
}

impl MigrationRiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            MigrationRiskLevel::SafeOnline => "SAFE_ONLINE",
            MigrationRiskLevel::Warning => "WARNING",
            MigrationRiskLevel::HazardousBreaking => "HAZARDOUS_BREAKING",
        }
    }
}

/// Granular Schema Difference Operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchemaDiffOperation {
    CreateTable {
        table_name: String,
        table: TableDefinition,
    },
    DropTable {
        table_name: String,
    },
    AddColumn {
        table_name: String,
        column: ColumnDefinition,
    },
    DropColumn {
        table_name: String,
        column_name: String,
        data_type: String,
    },
    AlterColumnType {
        table_name: String,
        column_name: String,
        old_type: String,
        new_type: String,
    },
    AlterColumnNullability {
        table_name: String,
        column_name: String,
        old_nullable: bool,
        new_nullable: bool,
    },
    AlterColumnDefault {
        table_name: String,
        column_name: String,
        old_default: Option<String>,
        new_default: Option<String>,
    },
    AddIndex {
        table_name: String,
        index: IndexDefinition,
    },
    DropIndex {
        table_name: String,
        index_name: String,
    },
    AddForeignKey {
        table_name: String,
        fk: ForeignKeyDefinition,
    },
    DropForeignKey {
        table_name: String,
        fk_name: String,
    },
}

/// Evaluated Single Diff Change with UP/DOWN SQL and Safety Linter Note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffChangeItem {
    pub operation: SchemaDiffOperation,
    pub risk_level: MigrationRiskLevel,
    pub summary: String,
    pub up_sql: String,
    pub down_sql: String,
    pub safety_guidance: String,
}

/// Complete Reversible Database Migration Plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub plan_id: String,
    pub baseline_schema_id: String,
    pub target_schema_id: String,
    pub dialect: SqlDialect,
    pub total_operations: usize,
    pub breaking_changes_count: usize,
    pub warning_changes_count: usize,
    pub safe_changes_count: usize,
    pub overall_risk: MigrationRiskLevel,
    pub changes: Vec<DiffChangeItem>,
    pub up_script: String,
    pub down_script: String,
    pub generated_at_epoch_ms: i64,
}

/// Cryptographically Anchored Migration Integrity Certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationIntegrityManifest {
    pub manifest_id: String,
    pub plan_id: String,
    pub baseline_blake3_hash: String,
    pub target_blake3_hash: String,
    pub migration_blake3_hash: String,
    pub ed25519_signature_hex: String,
    pub signer_public_key_hex: String,
    pub issued_at_epoch_ms: i64,
}

/// Executive Workbench Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioStats {
    pub schemas_analyzed: usize,
    pub migrations_generated: usize,
    pub breaking_hazards_prevented: usize,
    pub safe_online_deployments: usize,
}
