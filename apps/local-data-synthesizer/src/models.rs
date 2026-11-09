use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Integer,
    Float,
    Text,
    Boolean,
    Date,
    Uuid,
    Email,
    Name,
    PhoneNumber,
    Category,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyRef {
    pub target_table: String,
    pub target_column: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "strategy", content = "params")]
pub enum AnonymizationStrategy {
    #[serde(rename = "pseudonymize_hmac")]
    PseudonymizeHmac,
    #[serde(rename = "synthetic_gaussian")]
    SyntheticGaussian {
        mean: f64,
        std_dev: f64,
        min: f64,
        max: f64,
    },
    #[serde(rename = "synthetic_categorical")]
    SyntheticCategorical {
        categories: Vec<String>,
        weights: Vec<f64>,
    },
    #[serde(rename = "synthetic_date_range")]
    SyntheticDateRange { start_year: i32, end_year: i32 },
    #[serde(rename = "synthetic_name")]
    SyntheticName,
    #[serde(rename = "synthetic_email")]
    SyntheticEmail,
    #[serde(rename = "synthetic_uuid")]
    SyntheticUuid,
    #[serde(rename = "pass_through")]
    PassThrough,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnRule {
    pub column_name: String,
    pub data_type: DataType,
    pub is_primary_key: bool,
    pub foreign_key_ref: Option<ForeignKeyRef>,
    pub strategy: AnonymizationStrategy,
    pub nullable_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableSchema {
    pub table_name: String,
    pub row_count: usize,
    pub columns: Vec<ColumnRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntheticDatasetConfig {
    pub dataset_id: String,
    pub name: String,
    pub tables: Vec<TableSchema>,
    pub target_rows: usize,
    pub hmac_salt: String,
    pub preserve_referential_integrity: bool,
    pub differential_privacy_epsilon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacyAuditReport {
    pub dataset_id: String,
    pub total_tables: usize,
    pub total_rows_generated: usize,
    pub k_anonymity: usize,
    pub reidentification_risk_score: f64,
    pub epsilon_guarantee: f64,
    pub audit_passed: bool,
    pub signing_key_id: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesizerStats {
    pub total_schemas_analyzed: usize,
    pub total_datasets_generated: usize,
    pub total_rows_synthesized: usize,
    pub avg_k_anonymity: usize,
}
