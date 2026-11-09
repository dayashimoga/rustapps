use crate::models::{
    AnonymizationStrategy, ColumnRule, DataType, ForeignKeyRef, PrivacyAuditReport,
    SynthesizerStats, SyntheticDatasetConfig, TableSchema,
};
use crate::synth_engine::SynthEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SynthError {
    #[error("Dataset not found: {0}")]
    NotFound(String),
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("Lock error: {0}")]
    Lock(String),
}

type TableRowMap = HashMap<String, Vec<Map<String, Value>>>;
type DatasetCache = Arc<RwLock<HashMap<String, TableRowMap>>>;

pub struct SynthesizerService {
    datasets: Arc<RwLock<HashMap<String, SyntheticDatasetConfig>>>,
    generated_cache: DatasetCache,
    audit_reports: Arc<RwLock<HashMap<String, PrivacyAuditReport>>>,
    stats: Arc<RwLock<SynthesizerStats>>,
    master_key: SigningKey,
}

impl Default for SynthesizerService {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthesizerService {
    pub fn new() -> Self {
        let service = Self {
            datasets: Arc::new(RwLock::new(HashMap::new())),
            generated_cache: Arc::new(RwLock::new(HashMap::new())),
            audit_reports: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SynthesizerStats {
                total_schemas_analyzed: 0,
                total_datasets_generated: 0,
                total_rows_synthesized: 0,
                avg_k_anonymity: 5,
            })),
            master_key: SigningKey::generate(&mut OsRng),
        };

        service.create_demo_schema();
        service
    }

    /// Seeds realistic E-Commerce / FinTech schema with referential integrity
    pub fn create_demo_schema(&self) {
        let users_table = TableSchema {
            table_name: "customers".to_string(),
            row_count: 20,
            columns: vec![
                ColumnRule {
                    column_name: "customer_id".to_string(),
                    data_type: DataType::Uuid,
                    is_primary_key: true,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticUuid,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "full_name".to_string(),
                    data_type: DataType::Name,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticName,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "email_address".to_string(),
                    data_type: DataType::Email,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticEmail,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "risk_tier".to_string(),
                    data_type: DataType::Category,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticCategorical {
                        categories: vec![
                            "STANDARD".to_string(),
                            "PREFERRED".to_string(),
                            "HIGH_ROLLER".to_string(),
                        ],
                        weights: vec![0.7, 0.25, 0.05],
                    },
                    nullable_probability: 0.0,
                },
            ],
        };

        let transactions_table = TableSchema {
            table_name: "financial_transactions".to_string(),
            row_count: 50,
            columns: vec![
                ColumnRule {
                    column_name: "tx_id".to_string(),
                    data_type: DataType::Uuid,
                    is_primary_key: true,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticUuid,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "customer_ref".to_string(),
                    data_type: DataType::Uuid,
                    is_primary_key: false,
                    foreign_key_ref: Some(ForeignKeyRef {
                        target_table: "customers".to_string(),
                        target_column: "customer_id".to_string(),
                    }),
                    strategy: AnonymizationStrategy::SyntheticUuid,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "amount_usd".to_string(),
                    data_type: DataType::Float,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticGaussian {
                        mean: 145.50,
                        std_dev: 48.0,
                        min: 5.0,
                        max: 950.0,
                    },
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "tx_date".to_string(),
                    data_type: DataType::Date,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticDateRange {
                        start_year: 2024,
                        end_year: 2026,
                    },
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "payment_status".to_string(),
                    data_type: DataType::Category,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticCategorical {
                        categories: vec![
                            "SETTLED".to_string(),
                            "PENDING".to_string(),
                            "FLAGGED".to_string(),
                        ],
                        weights: vec![0.85, 0.12, 0.03],
                    },
                    nullable_probability: 0.0,
                },
            ],
        };

        let config = SyntheticDatasetConfig {
            dataset_id: "ds-fintech-demo".to_string(),
            name: "FinTech Banking & Ledger Sandbox".to_string(),
            tables: vec![users_table, transactions_table],
            target_rows: 70,
            hmac_salt: "salt-demo-2026".to_string(),
            preserve_referential_integrity: true,
            differential_privacy_epsilon: 1.0,
        };

        if let Ok(mut map) = self.datasets.write() {
            map.insert(config.dataset_id.clone(), config);
        }

        if let Ok(mut s) = self.stats.write() {
            s.total_schemas_analyzed += 1;
        }
    }

    /// Retrieves dataset schema config
    pub fn get_dataset_config(
        &self,
        dataset_id: &str,
    ) -> Result<SyntheticDatasetConfig, SynthError> {
        let map = self
            .datasets
            .read()
            .map_err(|e| SynthError::Lock(e.to_string()))?;
        map.get(dataset_id)
            .cloned()
            .ok_or_else(|| SynthError::NotFound(dataset_id.to_string()))
    }

    /// Generates synthetic dataset rows and evaluates privacy audit report
    pub fn synthesize_dataset(
        &self,
        dataset_id: &str,
        target_rows_override: Option<usize>,
    ) -> Result<PrivacyAuditReport, SynthError> {
        let config = self.get_dataset_config(dataset_id)?;

        let synthesized =
            SynthEngine::synthesize_tables(&config.tables, target_rows_override, &config.hmac_salt);

        let report = SynthEngine::evaluate_privacy_guarantee(
            dataset_id,
            &synthesized,
            config.differential_privacy_epsilon,
            &self.master_key,
        );

        // Cache generated data
        if let Ok(mut cache) = self.generated_cache.write() {
            cache.insert(dataset_id.to_string(), synthesized);
        }

        // Cache audit report
        if let Ok(mut reports) = self.audit_reports.write() {
            reports.insert(dataset_id.to_string(), report.clone());
        }

        // Update stats
        if let Ok(mut s) = self.stats.write() {
            s.total_datasets_generated += 1;
            s.total_rows_synthesized += report.total_rows_generated;
            s.avg_k_anonymity = report.k_anonymity;
        }

        Ok(report)
    }

    /// Retrieves preview rows for a table
    pub fn get_table_preview(
        &self,
        dataset_id: &str,
        table_name: &str,
        limit: usize,
    ) -> Result<Vec<Map<String, Value>>, SynthError> {
        let cache = self
            .generated_cache
            .read()
            .map_err(|e| SynthError::Lock(e.to_string()))?;
        let ds = cache
            .get(dataset_id)
            .ok_or_else(|| SynthError::NotFound(dataset_id.to_string()))?;
        let table_rows = ds
            .get(table_name)
            .ok_or_else(|| SynthError::TableNotFound(table_name.to_string()))?;

        Ok(table_rows.iter().take(limit).cloned().collect())
    }

    /// Exports table as CSV string
    pub fn export_table_csv(
        &self,
        dataset_id: &str,
        table_name: &str,
    ) -> Result<String, SynthError> {
        let rows = self.get_table_preview(dataset_id, table_name, usize::MAX)?;
        if rows.is_empty() {
            return Ok(String::new());
        }

        let headers: Vec<String> = rows[0].keys().cloned().collect();
        let mut csv = headers.join(",") + "\n";

        for r in rows {
            let line: Vec<String> = headers
                .iter()
                .map(|h| {
                    let v = r.get(h).unwrap_or(&Value::Null);
                    match v {
                        Value::String(s) => format!("\"{}\"", s.replace('"', "\"\"")),
                        Value::Null => "".to_string(),
                        other => other.to_string(),
                    }
                })
                .collect();
            csv.push_str(&line.join(","));
            csv.push('\n');
        }

        Ok(csv)
    }

    /// Returns aggregate metrics
    pub fn get_stats(&self) -> SynthesizerStats {
        self.stats
            .read()
            .map(|s| s.clone())
            .unwrap_or(SynthesizerStats {
                total_schemas_analyzed: 1,
                total_datasets_generated: 0,
                total_rows_synthesized: 0,
                avg_k_anonymity: 5,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_initialization_and_demo_synthesis() {
        let service = SynthesizerService::new();
        let report = service.synthesize_dataset("ds-fintech-demo", None).unwrap();

        assert_eq!(report.total_tables, 2);
        assert_eq!(report.total_rows_generated, 70); // 20 + 50
        assert!(report.audit_passed);

        let preview = service
            .get_table_preview("ds-fintech-demo", "customers", 5)
            .unwrap();
        assert_eq!(preview.len(), 5);
        assert!(preview[0].get("full_name").is_some());

        let csv = service
            .export_table_csv("ds-fintech-demo", "financial_transactions")
            .unwrap();
        assert!(csv.contains("tx_id"));
        assert!(csv.contains("amount_usd"));
    }
}
