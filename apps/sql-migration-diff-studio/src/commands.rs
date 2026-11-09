//! IPC Command Dispatcher for Visual SQL Schema & Migration Diff Studio.

use crate::models::SqlDialect;
use crate::service::SchemaService;
use serde_json::{json, Value};
use shared_licensing::{AppId, LicenseStatusReport, LicensingManager};
use std::sync::{Arc, OnceLock};

static SERVICE: OnceLock<Arc<SchemaService>> = OnceLock::new();
static LICENSING: OnceLock<LicensingManager> = OnceLock::new();

pub fn get_service() -> Arc<SchemaService> {
    SERVICE
        .get_or_init(|| Arc::new(SchemaService::new()))
        .clone()
}

pub fn get_licensing() -> &'static LicensingManager {
    LICENSING.get_or_init(|| {
        let data_dir = platform_common::AppPaths::from_app_name("sql-migration-diff-studio")
            .map(|p| p.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("data"));
        LicensingManager::new(AppId::SqlMigrationDiffStudio, &data_dir)
    })
}

/// Dispatches an IPC command formatted as JSON
pub fn dispatch_ipc_command(action: &str, payload: Value) -> Result<Value, String> {
    let service = get_service();
    let licensing = get_licensing();

    match action {
        "get_stats" => {
            let stats = service.get_stats();
            Ok(json!(stats))
        }

        "list_schemas" => {
            let list = service.list_schemas();
            Ok(json!(list))
        }

        "get_schema" => {
            let schema_id = payload
                .get("schema_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let schema = service.get_schema(schema_id);
            Ok(json!(schema))
        }

        "parse_schema" => {
            let title = payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Schema");
            let raw_ddl = payload
                .get("raw_ddl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing raw_ddl payload".to_string())?;

            let dialect = payload
                .get("dialect")
                .and_then(|v| v.as_str())
                .map(|d| match d {
                    "mysql" => SqlDialect::MySql,
                    "sqlite" => SqlDialect::Sqlite,
                    "oracle" => SqlDialect::Oracle,
                    "mssql" => SqlDialect::MsSqlServer,
                    _ => SqlDialect::PostgreSql,
                })
                .unwrap_or(SqlDialect::PostgreSql);

            let schema = service.parse_schema(title, dialect, raw_ddl);
            Ok(json!(schema))
        }

        "compare_schemas" => {
            let base_id = payload
                .get("base_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing base_id".to_string())?;
            let target_id = payload
                .get("target_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing target_id".to_string())?;

            let plan = service.compare_schemas(base_id, target_id)?;
            Ok(json!(plan))
        }

        "get_migration_plan" => {
            let plan_id = payload
                .get("plan_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let plan = service.get_migration_plan(plan_id);
            Ok(json!(plan))
        }

        "generate_integrity_manifest" => {
            let plan_id = payload
                .get("plan_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let manifest = service.sign_migration_manifest(plan_id)?;
            Ok(json!(manifest))
        }

        "get_license_status" => {
            let report: LicenseStatusReport = licensing.status_report();
            Ok(json!(report))
        }

        "apply_license_key" => {
            let key = payload
                .get("license_key")
                .or_else(|| payload.get("key"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing license_key".to_string())?;
            let report = licensing.activate(key).map_err(|e| e.to_string())?;
            Ok(json!(report))
        }

        _ => Err(format!("Unknown SQL diff studio IPC command: '{}'", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_commands_end_to_end() {
        // 1. Get stats
        let stats_val = dispatch_ipc_command("get_stats", json!({})).expect("get_stats");
        assert!(stats_val.get("schemas_analyzed").is_some());
        assert!(stats_val["schemas_analyzed"].as_u64().unwrap() >= 2);

        // 2. List schemas
        let list_val = dispatch_ipc_command("list_schemas", json!({})).expect("list_schemas");
        assert!(list_val.as_array().unwrap().len() >= 2);

        // 3. Get schema
        let schema_val =
            dispatch_ipc_command("get_schema", json!({ "schema_id": "SCHEMA-PROD-V1" }))
                .expect("get_schema");
        assert_eq!(schema_val["schema_id"], "SCHEMA-PROD-V1");
        assert!(schema_val["tables"].as_array().unwrap().len() >= 2);

        // 4. Parse new DDL schema
        let ddl = "CREATE TABLE products ( id BIGSERIAL PRIMARY KEY, name VARCHAR(100) NOT NULL );";
        let new_schema = dispatch_ipc_command(
            "parse_schema",
            json!({
                "title": "Inventory Microservice",
                "dialect": "postgresql",
                "raw_ddl": ddl
            }),
        )
        .expect("parse_schema");
        assert_eq!(new_schema["title"], "Inventory Microservice");

        // 5. Compare schemas
        let plan_val = dispatch_ipc_command(
            "compare_schemas",
            json!({
                "base_id": "SCHEMA-PROD-V1",
                "target_id": "SCHEMA-STAGING-V2"
            }),
        )
        .expect("compare_schemas");
        assert!(!plan_val["changes"].as_array().unwrap().is_empty());
        assert!(plan_val["up_script"].as_str().unwrap().contains("BEGIN;"));

        // 6. Sign integrity manifest
        let manifest_val = dispatch_ipc_command(
            "generate_integrity_manifest",
            json!({ "plan_id": plan_val["plan_id"].as_str().unwrap() }),
        )
        .expect("generate_integrity_manifest");
        assert!(manifest_val["ed25519_signature_hex"].as_str().is_some());

        // 7. License Status
        let lic_val =
            dispatch_ipc_command("get_license_status", json!({})).expect("get_license_status");
        assert!(lic_val.get("tier").is_some());
    }
}
