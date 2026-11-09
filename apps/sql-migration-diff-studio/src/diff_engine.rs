//! Visual SQL Schema Parsing, Differential Comparison & Safe Migration Generator.
//!
//! Provides AST-level table, column, and index extraction,
//! detects backward-incompatible breaking changes, assigns migration risk grades,
//! and synthesizes transactional reversible UP/DOWN scripts.

use crate::models::{
    ColumnDefinition, DatabaseSchema, DiffChangeItem, ForeignKeyDefinition, IndexDefinition,
    MigrationIntegrityManifest, MigrationPlan, MigrationRiskLevel, SchemaDiffOperation, SqlDialect,
    TableDefinition,
};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::{HashMap, HashSet};

pub struct DiffEngine;

impl DiffEngine {
    /// Parses raw SQL DDL text into a structured DatabaseSchema
    pub fn parse_ddl(
        schema_id: &str,
        title: &str,
        dialect: SqlDialect,
        raw_ddl: &str,
    ) -> DatabaseSchema {
        let tables = Self::extract_tables(raw_ddl);

        DatabaseSchema {
            schema_id: schema_id.to_string(),
            title: title.to_string(),
            dialect,
            tables,
            raw_ddl: raw_ddl.to_string(),
            created_at_epoch_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Extracts tables, columns, primary keys, and indexes from DDL statements
    fn extract_tables(raw: &str) -> Vec<TableDefinition> {
        let mut tables = Vec::new();

        // Split statements by semicolon
        let statements: Vec<&str> = raw
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Pass 1: Parse CREATE TABLE statements
        let create_table_re = regex::Regex::new(
            r"(?is)CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([`\[\x22]?([a-zA-Z0-9_\.]+)[`\]\x22]?)\s*\((.*)\)",
        )
        .unwrap();

        for stmt in &statements {
            if let Some(caps) = create_table_re.captures(stmt) {
                let table_name = caps
                    .get(2)
                    .map(|m| m.as_str().split('.').next_back().unwrap_or(m.as_str()))
                    .unwrap_or("unknown_table")
                    .to_string();

                let body = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let (columns, pk, fks) = Self::parse_table_body(body);

                tables.push(TableDefinition {
                    name: table_name,
                    columns,
                    primary_key: pk,
                    foreign_keys: fks,
                    indexes: Vec::new(),
                });
            }
        }

        // Pass 2: Parse standalone CREATE [UNIQUE] INDEX statements
        let index_re = regex::Regex::new(
            r"(?is)CREATE\s+(UNIQUE\s+)?INDEX\s+(?:IF\s+NOT\s+EXISTS\s+)?([`\[\x22]?([a-zA-Z0-9_]+)[`\]\x22]?)\s+ON\s+([`\[\x22]?([a-zA-Z0-9_]+)[`\]\x22]?)\s*(?:USING\s+([a-zA-Z0-9_]+))?\s*\((.*?)\)",
        )
        .unwrap();

        for stmt in &statements {
            if let Some(caps) = index_re.captures(stmt) {
                let is_unique = caps.get(1).is_some();
                let idx_name = caps
                    .get(3)
                    .map(|m| m.as_str())
                    .unwrap_or("idx_unnamed")
                    .to_string();
                let table_name = caps.get(5).map(|m| m.as_str()).unwrap_or("").to_string();
                let idx_type = caps
                    .get(6)
                    .map(|m| m.as_str().to_uppercase())
                    .unwrap_or_else(|| "BTREE".to_string());
                let cols_raw = caps.get(7).map(|m| m.as_str()).unwrap_or("");

                let cols: Vec<String> = cols_raw
                    .split(',')
                    .map(|c| {
                        c.trim()
                            .trim_matches(|ch| ch == '"' || ch == '`' || ch == '[' || ch == ']')
                            .to_string()
                    })
                    .filter(|c| !c.is_empty())
                    .collect();

                if let Some(t) = tables
                    .iter_mut()
                    .find(|t| t.name.eq_ignore_ascii_case(&table_name))
                {
                    t.indexes.push(IndexDefinition {
                        name: idx_name,
                        table_name: t.name.clone(),
                        columns: cols,
                        is_unique,
                        index_type: idx_type,
                    });
                }
            }
        }

        tables
    }

    /// Parses columns, constraints, and inline definitions inside CREATE TABLE (...)
    fn parse_table_body(
        body: &str,
    ) -> (
        Vec<ColumnDefinition>,
        Vec<String>,
        Vec<ForeignKeyDefinition>,
    ) {
        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();
        let mut foreign_keys = Vec::new();

        // Split by top-level commas (avoid splitting commas within parentheses, e.g. DECIMAL(10, 2))
        let items = Self::split_top_level_commas(body);
        let fk_re = regex::Regex::new(r"(?is)(?:CONSTRAINT\s+[`\[\x22]?([a-zA-Z0-9_]+)[`\]\x22]?\s+)?FOREIGN\s+KEY\s*\((.*?)\)\s*REFERENCES\s*([`\[\x22]?([a-zA-Z0-9_]+)[`\]\x22]?)\s*\((.*?)\)").unwrap();

        for raw_item in items {
            let item = raw_item.trim();
            if item.is_empty() {
                continue;
            }

            let upper = item.to_uppercase();

            // Table-level PRIMARY KEY constraint
            if upper.starts_with("PRIMARY KEY")
                || upper.starts_with("CONSTRAINT") && upper.contains("PRIMARY KEY")
            {
                if let Some(open) = item.find('(') {
                    if let Some(close) = item.rfind(')') {
                        let pk_str = &item[open + 1..close];
                        for pk in pk_str.split(',') {
                            let clean = pk
                                .trim()
                                .trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']');
                            if !clean.is_empty() {
                                primary_keys.push(clean.to_string());
                            }
                        }
                    }
                }
                continue;
            }

            // Table-level FOREIGN KEY constraint
            if upper.starts_with("FOREIGN KEY")
                || (upper.starts_with("CONSTRAINT") && upper.contains("FOREIGN KEY"))
            {
                if let Some(caps) = fk_re.captures(item) {
                    let fk_name = caps
                        .get(1)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| format!("fk_{}", foreign_keys.len() + 1));
                    let src_cols: Vec<String> = caps
                        .get(2)
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .split(',')
                        .map(|c| {
                            c.trim()
                                .trim_matches(|ch| ch == '"' || ch == '`' || ch == '[' || ch == ']')
                                .to_string()
                        })
                        .collect();
                    let ref_tbl = caps
                        .get(4)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    let ref_cols: Vec<String> = caps
                        .get(5)
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .split(',')
                        .map(|c| {
                            c.trim()
                                .trim_matches(|ch| ch == '"' || ch == '`' || ch == '[' || ch == ']')
                                .to_string()
                        })
                        .collect();

                    let on_delete = if upper.contains("ON DELETE CASCADE") {
                        Some("CASCADE".to_string())
                    } else if upper.contains("ON DELETE SET NULL") {
                        Some("SET NULL".to_string())
                    } else {
                        None
                    };

                    foreign_keys.push(ForeignKeyDefinition {
                        name: fk_name,
                        constrained_columns: src_cols,
                        referenced_table: ref_tbl,
                        referenced_columns: ref_cols,
                        on_delete,
                        on_update: None,
                    });
                }
                continue;
            }

            // Otherwise, it's a Column Definition
            let tokens: Vec<&str> = item.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let col_name = tokens[0]
                .trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']')
                .to_string();
            let mut data_type = if tokens.len() > 1 {
                tokens[1].to_string()
            } else {
                "TEXT".to_string()
            };

            // If type has attached size like VARCHAR ( 255 )
            if data_type.contains('(') && !data_type.contains(')') && tokens.len() > 2 {
                for t in &tokens[2..] {
                    data_type.push_str(t);
                    if t.contains(')') {
                        break;
                    }
                }
            }

            let is_primary_key = upper.contains("PRIMARY KEY");
            let is_autoincrement = upper.contains("AUTO_INCREMENT")
                || upper.contains("SERIAL")
                || upper.contains("BIGSERIAL")
                || upper.contains("IDENTITY");

            let is_nullable = !upper.contains("NOT NULL") && !is_primary_key;

            let default_val = if let Some(def_idx) = upper.find("DEFAULT") {
                let after = &item[def_idx + 7..].trim();
                let def_token = after
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(',');
                Some(def_token.to_string())
            } else {
                None
            };

            if is_primary_key && !primary_keys.contains(&col_name) {
                primary_keys.push(col_name.clone());
            }

            columns.push(ColumnDefinition {
                name: col_name,
                data_type: data_type.to_uppercase(),
                is_nullable,
                default_value: default_val,
                is_primary_key,
                is_autoincrement,
            });
        }

        (columns, primary_keys, foreign_keys)
    }

    /// Splits column/constraint tokens by commas not enclosed in parentheses
    fn split_top_level_commas(input: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for ch in input.chars() {
            match ch {
                '(' => {
                    depth += 1;
                    current.push(ch);
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    result.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }

    /// Compares two database schemas and generates reversible UP/DOWN operations with risk grades
    pub fn compare_schemas(baseline: &DatabaseSchema, target: &DatabaseSchema) -> MigrationPlan {
        let mut changes = Vec::new();
        let dialect = target.dialect;

        let base_tables: HashMap<String, &TableDefinition> = baseline
            .tables
            .iter()
            .map(|t| (t.name.to_lowercase(), t))
            .collect();

        let target_tables: HashMap<String, &TableDefinition> = target
            .tables
            .iter()
            .map(|t| (t.name.to_lowercase(), t))
            .collect();

        // 1. Table Additions (in Target, not in Baseline)
        for (name, target_tbl) in &target_tables {
            if !base_tables.contains_key(name) {
                let up_sql = Self::generate_create_table_sql(target_tbl, dialect);
                let down_sql = format!("DROP TABLE IF EXISTS {} CASCADE;", target_tbl.name);

                changes.push(DiffChangeItem {
                    operation: SchemaDiffOperation::CreateTable {
                        table_name: target_tbl.name.clone(),
                        table: (*target_tbl).clone(),
                    },
                    risk_level: MigrationRiskLevel::SafeOnline,
                    summary: format!("Create new table '{}' with {} columns", target_tbl.name, target_tbl.columns.len()),
                    up_sql,
                    down_sql,
                    safety_guidance: "SAFE: Adding a new isolated table does not block existing production queries or require locks.".to_string(),
                });
            }
        }

        // 2. Table Modifications (in both)
        for (name, target_tbl) in &target_tables {
            if let Some(base_tbl) = base_tables.get(name) {
                // Check column changes
                let base_cols: HashMap<String, &ColumnDefinition> = base_tbl
                    .columns
                    .iter()
                    .map(|c| (c.name.to_lowercase(), c))
                    .collect();

                let target_cols: HashMap<String, &ColumnDefinition> = target_tbl
                    .columns
                    .iter()
                    .map(|c| (c.name.to_lowercase(), c))
                    .collect();

                // 2a. Added Columns
                for (c_name, t_col) in &target_cols {
                    if !base_cols.contains_key(c_name) {
                        let (risk, guidance) = if t_col.is_nullable {
                            (MigrationRiskLevel::SafeOnline, "SAFE: Adding a nullable column is metadata-only in modern RDBMS without table locks.")
                        } else if t_col.default_value.is_some() {
                            (MigrationRiskLevel::Warning, "WARNING: Adding NOT NULL column with DEFAULT is metadata-only in PostgreSQL 11+, but may trigger table rewrites on older engines.")
                        } else {
                            (MigrationRiskLevel::HazardousBreaking, "HAZARDOUS BREAKING: Adding a NOT NULL column without a DEFAULT value will immediately FAIL on existing populated tables. Use a 3-step migration: 1. Add NULL, 2. Backfill rows, 3. Set NOT NULL.")
                        };

                        let null_clause = if t_col.is_nullable { "" } else { " NOT NULL" };
                        let def_clause = t_col
                            .default_value
                            .as_ref()
                            .map(|d| format!(" DEFAULT {}", d))
                            .unwrap_or_default();

                        let up_sql = format!(
                            "ALTER TABLE {} ADD COLUMN {} {}{}{};",
                            target_tbl.name, t_col.name, t_col.data_type, def_clause, null_clause
                        );
                        let down_sql = format!(
                            "ALTER TABLE {} DROP COLUMN {};",
                            target_tbl.name, t_col.name
                        );

                        changes.push(DiffChangeItem {
                            operation: SchemaDiffOperation::AddColumn {
                                table_name: target_tbl.name.clone(),
                                column: (*t_col).clone(),
                            },
                            risk_level: risk,
                            summary: format!(
                                "Add column '{}.{}' ({})",
                                target_tbl.name, t_col.name, t_col.data_type
                            ),
                            up_sql,
                            down_sql,
                            safety_guidance: guidance.to_string(),
                        });
                    }
                }

                // 2b. Dropped Columns
                for (c_name, b_col) in &base_cols {
                    if !target_cols.contains_key(c_name) {
                        let up_sql = format!(
                            "ALTER TABLE {} DROP COLUMN {};",
                            target_tbl.name, b_col.name
                        );
                        let down_sql = format!(
                            "ALTER TABLE {} ADD COLUMN {} {};",
                            target_tbl.name, b_col.name, b_col.data_type
                        );

                        changes.push(DiffChangeItem {
                            operation: SchemaDiffOperation::DropColumn {
                                table_name: target_tbl.name.clone(),
                                column_name: b_col.name.clone(),
                                data_type: b_col.data_type.clone(),
                            },
                            risk_level: MigrationRiskLevel::HazardousBreaking,
                            summary: format!("Drop column '{}.{}' (destructive data loss)", target_tbl.name, b_col.name),
                            up_sql,
                            down_sql,
                            safety_guidance: "HAZARDOUS BREAKING: Dropping a column causes immediate, irreversible data deletion and will crash application queries still referencing it. Ensure code deprecation period before executing.".to_string(),
                        });
                    }
                }

                // 2c. Altered Columns (Type & Nullability)
                for (c_name, t_col) in &target_cols {
                    if let Some(b_col) = base_cols.get(c_name) {
                        if b_col.data_type != t_col.data_type {
                            let (risk, guidance) = if Self::is_widening_cast(
                                &b_col.data_type,
                                &t_col.data_type,
                            ) {
                                (MigrationRiskLevel::Warning, "WARNING: Widening type requires validation scan but generally avoids data truncation.")
                            } else {
                                (MigrationRiskLevel::HazardousBreaking, "HAZARDOUS BREAKING: Altering column type to incompatible or narrower type triggers table rewrite, exclusive locks, or silent data truncation.")
                            };

                            let up_sql = format!(
                                "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
                                target_tbl.name, t_col.name, t_col.data_type
                            );
                            let down_sql = format!(
                                "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
                                target_tbl.name, b_col.name, b_col.data_type
                            );

                            changes.push(DiffChangeItem {
                                operation: SchemaDiffOperation::AlterColumnType {
                                    table_name: target_tbl.name.clone(),
                                    column_name: t_col.name.clone(),
                                    old_type: b_col.data_type.clone(),
                                    new_type: t_col.data_type.clone(),
                                },
                                risk_level: risk,
                                summary: format!(
                                    "Alter column '{}.{}' type from {} to {}",
                                    target_tbl.name, t_col.name, b_col.data_type, t_col.data_type
                                ),
                                up_sql,
                                down_sql,
                                safety_guidance: guidance.to_string(),
                            });
                        }

                        if b_col.is_nullable != t_col.is_nullable {
                            let (risk, guidance, up_action, down_action) = if !t_col.is_nullable {
                                (MigrationRiskLevel::HazardousBreaking, "HAZARDOUS BREAKING: Setting NOT NULL requires verifying zero existing NULL values; blocks concurrently without proper checks.", "SET NOT NULL", "DROP NOT NULL")
                            } else {
                                (MigrationRiskLevel::SafeOnline, "SAFE: Relaxing NOT NULL constraint to nullable is safe and non-blocking.", "DROP NOT NULL", "SET NOT NULL")
                            };

                            let up_sql = format!(
                                "ALTER TABLE {} ALTER COLUMN {} {};",
                                target_tbl.name, t_col.name, up_action
                            );
                            let down_sql = format!(
                                "ALTER TABLE {} ALTER COLUMN {} {};",
                                target_tbl.name, b_col.name, down_action
                            );

                            changes.push(DiffChangeItem {
                                operation: SchemaDiffOperation::AlterColumnNullability {
                                    table_name: target_tbl.name.clone(),
                                    column_name: t_col.name.clone(),
                                    old_nullable: b_col.is_nullable,
                                    new_nullable: t_col.is_nullable,
                                },
                                risk_level: risk,
                                summary: format!(
                                    "Alter nullability on '{}.{}' (now {})",
                                    target_tbl.name,
                                    t_col.name,
                                    if t_col.is_nullable {
                                        "NULL"
                                    } else {
                                        "NOT NULL"
                                    }
                                ),
                                up_sql,
                                down_sql,
                                safety_guidance: guidance.to_string(),
                            });
                        }
                    }
                }

                // 2d. Index Additions & Removals
                let base_idx_names: HashSet<String> = base_tbl
                    .indexes
                    .iter()
                    .map(|i| i.name.to_lowercase())
                    .collect();
                let target_idx_names: HashSet<String> = target_tbl
                    .indexes
                    .iter()
                    .map(|i| i.name.to_lowercase())
                    .collect();

                for t_idx in &target_tbl.indexes {
                    if !base_idx_names.contains(&t_idx.name.to_lowercase()) {
                        let uniq_str = if t_idx.is_unique { "UNIQUE " } else { "" };
                        let cols_str = t_idx.columns.join(", ");
                        let up_sql = format!(
                            "CREATE {}INDEX CONCURRENTLY IF NOT EXISTS {} ON {} ({});",
                            uniq_str, t_idx.name, target_tbl.name, cols_str
                        );
                        let down_sql = format!("DROP INDEX CONCURRENTLY IF EXISTS {};", t_idx.name);

                        changes.push(DiffChangeItem {
                            operation: SchemaDiffOperation::AddIndex {
                                table_name: target_tbl.name.clone(),
                                index: t_idx.clone(),
                            },
                            risk_level: MigrationRiskLevel::SafeOnline,
                            summary: format!("Add index '{}' on {} ({})", t_idx.name, target_tbl.name, cols_str),
                            up_sql,
                            down_sql,
                            safety_guidance: "SAFE ONLINE: Creating index CONCURRENTLY avoids table locking in production environments.".to_string(),
                        });
                    }
                }

                for b_idx in &base_tbl.indexes {
                    if !target_idx_names.contains(&b_idx.name.to_lowercase()) {
                        let up_sql = format!("DROP INDEX CONCURRENTLY IF EXISTS {};", b_idx.name);
                        let down_sql = format!(
                            "CREATE {}INDEX CONCURRENTLY IF NOT EXISTS {} ON {} ({});",
                            if b_idx.is_unique { "UNIQUE " } else { "" },
                            b_idx.name,
                            base_tbl.name,
                            b_idx.columns.join(", ")
                        );

                        changes.push(DiffChangeItem {
                            operation: SchemaDiffOperation::DropIndex {
                                table_name: base_tbl.name.clone(),
                                index_name: b_idx.name.clone(),
                            },
                            risk_level: MigrationRiskLevel::Warning,
                            summary: format!("Drop index '{}' from {}", b_idx.name, base_tbl.name),
                            up_sql,
                            down_sql,
                            safety_guidance: "WARNING: Dropping an index will affect query planner execution plans and may cause slow query performance regressions.".to_string(),
                        });
                    }
                }
            }
        }

        // 3. Table Removals (in Baseline, not in Target)
        for (name, base_tbl) in &base_tables {
            if !target_tables.contains_key(name) {
                let up_sql = format!("DROP TABLE IF EXISTS {} CASCADE;", base_tbl.name);
                let down_sql = Self::generate_create_table_sql(base_tbl, dialect);

                changes.push(DiffChangeItem {
                    operation: SchemaDiffOperation::DropTable {
                        table_name: base_tbl.name.clone(),
                    },
                    risk_level: MigrationRiskLevel::HazardousBreaking,
                    summary: format!("Drop table '{}' (destructive table removal)", base_tbl.name),
                    up_sql,
                    down_sql,
                    safety_guidance: "HAZARDOUS BREAKING: Dropping a table permanently deletes all rows and related foreign keys. Verify application no longer queries this table.".to_string(),
                });
            }
        }

        // Calculate risk metrics
        let mut breaking = 0;
        let mut warnings = 0;
        let mut safe = 0;

        for c in &changes {
            match c.risk_level {
                MigrationRiskLevel::HazardousBreaking => breaking += 1,
                MigrationRiskLevel::Warning => warnings += 1,
                MigrationRiskLevel::SafeOnline => safe += 1,
            }
        }

        let overall_risk = if breaking > 0 {
            MigrationRiskLevel::HazardousBreaking
        } else if warnings > 0 {
            MigrationRiskLevel::Warning
        } else {
            MigrationRiskLevel::SafeOnline
        };

        // Synthesize Reversible UP & DOWN Scripts
        let up_script = Self::synthesize_up_script(&changes, dialect);
        let down_script = Self::synthesize_down_script(&changes, dialect);

        let plan_id = format!(
            "MIG-{}",
            blake3::hash(format!("{}:{}", baseline.schema_id, target.schema_id).as_bytes())
                .to_hex()[0..10]
                .to_uppercase()
        );

        MigrationPlan {
            plan_id,
            baseline_schema_id: baseline.schema_id.clone(),
            target_schema_id: target.schema_id.clone(),
            dialect,
            total_operations: changes.len(),
            breaking_changes_count: breaking,
            warning_changes_count: warnings,
            safe_changes_count: safe,
            overall_risk,
            changes,
            up_script,
            down_script,
            generated_at_epoch_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn is_widening_cast(from: &str, to: &str) -> bool {
        let f = from.to_uppercase();
        let t = to.to_uppercase();
        (f.contains("INT") && t.contains("BIGINT"))
            || (f.contains("VARCHAR(50)") && t.contains("VARCHAR(255)"))
            || (f.contains("VARCHAR") && t.contains("TEXT"))
            || (f.contains("FLOAT") && t.contains("DOUBLE"))
    }

    fn generate_create_table_sql(tbl: &TableDefinition, _dialect: SqlDialect) -> String {
        let mut col_defs = Vec::new();

        for c in &tbl.columns {
            let mut line = format!("    {} {}", c.name, c.data_type);
            if let Some(ref def) = c.default_value {
                line.push_str(&format!(" DEFAULT {}", def));
            }
            if !c.is_nullable {
                line.push_str(" NOT NULL");
            }
            col_defs.push(line);
        }

        if !tbl.primary_key.is_empty() {
            col_defs.push(format!("    PRIMARY KEY ({})", tbl.primary_key.join(", ")));
        }

        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n{}\n);",
            tbl.name,
            col_defs.join(",\n")
        )
    }

    fn synthesize_up_script(changes: &[DiffChangeItem], dialect: SqlDialect) -> String {
        let mut script = String::new();
        script.push_str(&format!(
            "-- ============================================================================\n\
             -- AIR-GAPPED SAFE DATABASE MIGRATION SCRIPT (UP)\n\
             -- Dialect: {}\n\
             -- Total Operations: {}\n\
             -- Safety Guard: Reversible Transaction with Statement Timeouts\n\
             -- ============================================================================\n\n",
            dialect.as_str(),
            changes.len()
        ));

        script.push_str("BEGIN;\n");
        script.push_str("SET statement_timeout = '5s';\n");
        script.push_str("SET lock_timeout = '2s';\n\n");

        for (idx, c) in changes.iter().enumerate() {
            script.push_str(&format!(
                "-- Step {}: [{}] {}\n\
                 -- Guidance: {}\n\
                 {}\n\n",
                idx + 1,
                c.risk_level.as_str(),
                c.summary,
                c.safety_guidance,
                c.up_sql
            ));
        }

        script.push_str("COMMIT;\n");
        script
    }

    fn synthesize_down_script(changes: &[DiffChangeItem], dialect: SqlDialect) -> String {
        let mut script = String::new();
        script.push_str(&format!(
            "-- ============================================================================\n\
             -- AIR-GAPPED SAFE DATABASE ROLLBACK SCRIPT (DOWN)\n\
             -- Dialect: {}\n\
             -- Total Operations: {}\n\
             -- ============================================================================\n\n",
            dialect.as_str(),
            changes.len()
        ));

        script.push_str("BEGIN;\n");
        script.push_str("SET statement_timeout = '5s';\n");
        script.push_str("SET lock_timeout = '2s';\n\n");

        // Rollback operations in reverse order
        for (idx, c) in changes.iter().rev().enumerate() {
            script.push_str(&format!(
                "-- Rollback Step {}: Revert {}\n\
                 {}\n\n",
                idx + 1,
                c.summary,
                c.down_sql
            ));
        }

        script.push_str("COMMIT;\n");
        script
    }

    /// Generates an immutable, Ed25519-signed Migration Integrity Manifest
    pub fn sign_manifest(
        plan: &MigrationPlan,
        base_ddl: &str,
        target_ddl: &str,
        signing_key: &SigningKey,
    ) -> MigrationIntegrityManifest {
        let base_hash = blake3::hash(base_ddl.as_bytes()).to_hex().to_string();
        let target_hash = blake3::hash(target_ddl.as_bytes()).to_hex().to_string();
        let mig_hash = blake3::hash(plan.up_script.as_bytes()).to_hex().to_string();

        let manifest_id = format!(
            "CERT-MIG-{}",
            blake3::hash(plan.plan_id.as_bytes()).to_hex()[0..12].to_uppercase()
        );

        let manifest_payload = format!(
            "MIGRATION-MANIFEST:{}|PLAN:{}|BASE:{}|TARGET:{}|MIG:{}|OPS:{}|BREAKING:{}",
            manifest_id,
            plan.plan_id,
            base_hash,
            target_hash,
            mig_hash,
            plan.total_operations,
            plan.breaking_changes_count
        );

        let sig = signing_key.sign(manifest_payload.as_bytes());
        let ed25519_signature_hex = hex::encode(sig.to_bytes());
        let signer_public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());

        MigrationIntegrityManifest {
            manifest_id,
            plan_id: plan.plan_id.clone(),
            baseline_blake3_hash: base_hash,
            target_blake3_hash: target_hash,
            migration_blake3_hash: mig_hash,
            ed25519_signature_hex,
            signer_public_key_hex,
            issued_at_epoch_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use rand::rngs::OsRng;

    #[test]
    fn test_ddl_parsing() {
        let ddl = r#"
CREATE TABLE accounts (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL,
    balance NUMERIC(10, 2) DEFAULT 0.00,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_accounts_username ON accounts (username);
"#;

        let schema = DiffEngine::parse_ddl("TEST-1", "Test Accounts", SqlDialect::PostgreSql, ddl);
        assert_eq!(schema.tables.len(), 1);
        let table = &schema.tables[0];
        assert_eq!(table.name, "accounts");
        assert_eq!(table.columns.len(), 4);
        assert_eq!(table.primary_key, vec!["id".to_string()]);
        assert_eq!(table.indexes.len(), 1);
        assert!(table.indexes[0].is_unique);
        assert_eq!(table.indexes[0].columns, vec!["username".to_string()]);
    }

    #[test]
    fn test_breaking_change_detection() {
        let v1 = r#"
CREATE TABLE customers (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    deprecated_notes TEXT
);
"#;

        let v2 = r#"
CREATE TABLE customers (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    tier VARCHAR(20) NOT NULL
);
"#;

        let s1 = DiffEngine::parse_ddl("V1", "Base", SqlDialect::PostgreSql, v1);
        let s2 = DiffEngine::parse_ddl("V2", "Target", SqlDialect::PostgreSql, v2);

        let plan = DiffEngine::compare_schemas(&s1, &s2);

        // Should detect dropped column (deprecated_notes) as HazardousBreaking
        assert!(plan
            .changes
            .iter()
            .any(|c| c.risk_level == MigrationRiskLevel::HazardousBreaking
                && c.summary.contains("deprecated_notes")));

        // Should detect added NOT NULL column without DEFAULT (tier) as HazardousBreaking
        assert!(plan
            .changes
            .iter()
            .any(|c| c.risk_level == MigrationRiskLevel::HazardousBreaking
                && c.summary.contains("tier")));

        assert!(plan.breaking_changes_count >= 2);
        assert_eq!(plan.overall_risk, MigrationRiskLevel::HazardousBreaking);
    }

    #[test]
    fn test_manifest_signature_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let ddl1 = "CREATE TABLE t1 ( id INT PRIMARY KEY );";
        let ddl2 = "CREATE TABLE t1 ( id INT PRIMARY KEY, name TEXT );";

        let s1 = DiffEngine::parse_ddl("V1", "S1", SqlDialect::PostgreSql, ddl1);
        let s2 = DiffEngine::parse_ddl("V2", "S2", SqlDialect::PostgreSql, ddl2);

        let plan = DiffEngine::compare_schemas(&s1, &s2);
        let manifest = DiffEngine::sign_manifest(&plan, ddl1, ddl2, &signing_key);

        let manifest_payload = format!(
            "MIGRATION-MANIFEST:{}|PLAN:{}|BASE:{}|TARGET:{}|MIG:{}|OPS:{}|BREAKING:{}",
            manifest.manifest_id,
            manifest.plan_id,
            manifest.baseline_blake3_hash,
            manifest.target_blake3_hash,
            manifest.migration_blake3_hash,
            plan.total_operations,
            plan.breaking_changes_count
        );

        let sig_bytes = hex::decode(&manifest.ed25519_signature_hex).unwrap();
        let sig = Signature::from_bytes(sig_bytes[..].try_into().unwrap());
        let pubkey_bytes = hex::decode(&manifest.signer_public_key_hex).unwrap();
        let pubkey = VerifyingKey::from_bytes(pubkey_bytes[..].try_into().unwrap()).unwrap();

        assert!(pubkey.verify(manifest_payload.as_bytes(), &sig).is_ok());
    }
}
