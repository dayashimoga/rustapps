export type SqlDialect = 'postgresql' | 'mysql' | 'sqlite' | 'oracle' | 'mssql';

export type MigrationRiskLevel = 'safe_online' | 'warning' | 'hazardous_breaking';

export interface ColumnDefinition {
  name: string;
  data_type: string;
  is_nullable: boolean;
  default_value?: string | null;
  is_primary_key: boolean;
  is_autoincrement: boolean;
}

export interface ForeignKeyDefinition {
  name: string;
  constrained_columns: string[];
  referenced_table: string;
  referenced_columns: string[];
  on_delete?: string | null;
  on_update?: string | null;
}

export interface IndexDefinition {
  name: string;
  table_name: string;
  columns: string[];
  is_unique: boolean;
  index_type: string;
}

export interface TableDefinition {
  name: string;
  columns: ColumnDefinition[];
  primary_key: string[];
  foreign_keys: ForeignKeyDefinition[];
  indexes: IndexDefinition[];
}

export interface DatabaseSchema {
  schema_id: string;
  title: string;
  dialect: SqlDialect;
  tables: TableDefinition[];
  raw_ddl: string;
  created_at_epoch_ms: number;
}

export interface SchemaSummary {
  schema_id: string;
  title: string;
  dialect: SqlDialect;
  total_tables: number;
  total_columns: number;
  total_indexes: number;
  created_at_epoch_ms: number;
}

export interface DiffChangeItem {
  operation: any;
  risk_level: MigrationRiskLevel;
  summary: string;
  up_sql: string;
  down_sql: string;
  safety_guidance: string;
}

export interface MigrationPlan {
  plan_id: string;
  baseline_schema_id: string;
  target_schema_id: string;
  dialect: SqlDialect;
  total_operations: number;
  breaking_changes_count: number;
  warning_changes_count: number;
  safe_changes_count: number;
  overall_risk: MigrationRiskLevel;
  changes: DiffChangeItem[];
  up_script: string;
  down_script: string;
  generated_at_epoch_ms: number;
}

export interface MigrationIntegrityManifest {
  manifest_id: string;
  plan_id: string;
  baseline_blake3_hash: string;
  target_blake3_hash: string;
  migration_blake3_hash: string;
  ed25519_signature_hex: string;
  signer_public_key_hex: string;
  issued_at_epoch_ms: number;
}

export interface StudioStats {
  schemas_analyzed: number;
  migrations_generated: number;
  breaking_hazards_prevented: number;
  safe_online_deployments: number;
}

export interface LicenseStatusReport {
  tier: string;
  is_valid: boolean;
  features: string[];
  days_remaining?: number | null;
}
