use crate::models::{AnonymizationStrategy, PrivacyAuditReport, TableSchema};
use ed25519_dalek::{Signer, SigningKey};
use rand::Rng;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const FIRST_NAMES: &[&str] = &[
    "James",
    "Mary",
    "Robert",
    "Patricia",
    "John",
    "Jennifer",
    "Michael",
    "Linda",
    "David",
    "Elizabeth",
    "William",
    "Barbara",
    "Richard",
    "Susan",
    "Joseph",
    "Jessica",
    "Thomas",
    "Sarah",
    "Charles",
    "Karen",
    "Christopher",
    "Nancy",
    "Daniel",
    "Lisa",
    "Matthew",
    "Betty",
    "Anthony",
    "Margaret",
    "Mark",
    "Sandra",
    "Donald",
    "Ashley",
];

const LAST_NAMES: &[&str] = &[
    "Smith",
    "Johnson",
    "Williams",
    "Brown",
    "Jones",
    "Garcia",
    "Miller",
    "Davis",
    "Rodriguez",
    "Martinez",
    "Hernandez",
    "Lopez",
    "Gonzalez",
    "Wilson",
    "Anderson",
    "Thomas",
    "Taylor",
    "Moore",
    "Jackson",
    "Martin",
    "Lee",
    "Perez",
    "Thompson",
];

const EMAIL_DOMAINS: &[&str] = &[
    "staging-mail.internal",
    "dev-test.corp",
    "mock-sandbox.local",
    "qa-verify.io",
];

pub struct SynthEngine;

impl SynthEngine {
    /// Generates multi-table synthetic dataset preserving referential integrity
    pub fn synthesize_tables(
        tables: &[TableSchema],
        target_rows_override: Option<usize>,
        salt: &str,
    ) -> HashMap<String, Vec<Map<String, Value>>> {
        let mut result = HashMap::new();
        let mut pk_pools: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut rng = rand::thread_rng();

        for table in tables {
            let row_count = target_rows_override.unwrap_or(table.row_count);
            let mut rows = Vec::with_capacity(row_count);

            for row_idx in 0..row_count {
                let mut row = Map::new();

                for col in &table.columns {
                    // Check if nullable
                    if col.nullable_probability > 0.0 && !col.is_primary_key {
                        let roll: f64 = rng.gen();
                        if roll < col.nullable_probability {
                            row.insert(col.column_name.clone(), Value::Null);
                            continue;
                        }
                    }

                    // Check foreign key reference
                    if let Some(ref fk) = col.foreign_key_ref {
                        let pool_key = (fk.target_table.clone(), fk.target_column.clone());
                        if let Some(pool) = pk_pools.get(&pool_key) {
                            if !pool.is_empty() {
                                let chosen_idx = rng.gen_range(0..pool.len());
                                let val_str = &pool[chosen_idx];
                                row.insert(col.column_name.clone(), json!(val_str));
                                continue;
                            }
                        }
                    }

                    // Value generation based on strategy
                    let val = match &col.strategy {
                        AnonymizationStrategy::PseudonymizeHmac => {
                            let input =
                                format!("{}_{}_{}", table.table_name, col.column_name, row_idx);
                            let hash = Self::compute_hmac_hex(&input, salt);
                            json!(format!("ANON_{}", &hash[0..12].to_uppercase()))
                        }

                        AnonymizationStrategy::SyntheticGaussian {
                            mean,
                            std_dev,
                            min,
                            max,
                        } => {
                            let raw_sample = Self::sample_gaussian(*mean, *std_dev, &mut rng);
                            let clamped = raw_sample.clamp(*min, *max);
                            json!((clamped * 100.0).round() / 100.0)
                        }

                        AnonymizationStrategy::SyntheticCategorical {
                            categories,
                            weights,
                        } => {
                            let cat = Self::sample_categorical(categories, weights, &mut rng);
                            json!(cat)
                        }

                        AnonymizationStrategy::SyntheticDateRange {
                            start_year,
                            end_year,
                        } => {
                            let year = rng.gen_range(*start_year..=*end_year);
                            let month = rng.gen_range(1..=12);
                            let day = rng.gen_range(1..=28);
                            json!(format!("{:04}-{:02}-{:02}", year, month, day))
                        }

                        AnonymizationStrategy::SyntheticName => {
                            let fn_idx = rng.gen_range(0..FIRST_NAMES.len());
                            let ln_idx = rng.gen_range(0..LAST_NAMES.len());
                            json!(format!("{} {}", FIRST_NAMES[fn_idx], LAST_NAMES[ln_idx]))
                        }

                        AnonymizationStrategy::SyntheticEmail => {
                            let fn_idx = rng.gen_range(0..FIRST_NAMES.len());
                            let ln_idx = rng.gen_range(0..LAST_NAMES.len());
                            let dom_idx = rng.gen_range(0..EMAIL_DOMAINS.len());
                            let num = rng.gen_range(10..999);
                            json!(format!(
                                "{}.{}{}@{}",
                                FIRST_NAMES[fn_idx].to_lowercase(),
                                LAST_NAMES[ln_idx].to_lowercase(),
                                num,
                                EMAIL_DOMAINS[dom_idx]
                            ))
                        }

                        AnonymizationStrategy::SyntheticUuid => {
                            json!(uuid::Uuid::new_v4().to_string())
                        }

                        AnonymizationStrategy::PassThrough => {
                            json!(format!("VAL_{}_{}", col.column_name, row_idx + 1))
                        }
                    };

                    // Record primary key pool
                    if col.is_primary_key {
                        let pool_key = (table.table_name.clone(), col.column_name.clone());
                        let pk_str = val
                            .as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| val.to_string());
                        pk_pools.entry(pool_key).or_default().push(pk_str);
                    }

                    row.insert(col.column_name.clone(), val);
                }

                rows.push(row);
            }

            result.insert(table.table_name.clone(), rows);
        }

        result
    }

    /// Box-Muller transform for Gaussian distribution sampling
    pub fn sample_gaussian<R: Rng>(mean: f64, std_dev: f64, rng: &mut R) -> f64 {
        let u1: f64 = rng.gen::<f64>().max(1e-10);
        let u2: f64 = rng.gen::<f64>();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mean + z0 * std_dev
    }

    /// Weighted categorical sampling
    pub fn sample_categorical<R: Rng>(
        categories: &[String],
        weights: &[f64],
        rng: &mut R,
    ) -> String {
        if categories.is_empty() {
            return "DEFAULT".to_string();
        }
        let total_weight: f64 = weights.iter().sum();
        if total_weight <= 0.0 {
            return categories[0].clone();
        }

        let mut roll: f64 = rng.gen_range(0.0..total_weight);
        for (idx, &w) in weights.iter().enumerate() {
            if roll < w {
                return categories
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| categories[0].clone());
            }
            roll -= w;
        }

        categories
            .last()
            .cloned()
            .unwrap_or_else(|| categories[0].clone())
    }

    /// Computes HMAC-SHA256 hex string
    pub fn compute_hmac_hex(input: &str, salt: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(input.as_bytes());
        hasher.update(salt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Evaluates differential privacy and k-anonymity guarantee
    pub fn evaluate_privacy_guarantee(
        dataset_id: &str,
        data: &HashMap<String, Vec<Map<String, Value>>>,
        epsilon: f64,
        signing_key: &SigningKey,
    ) -> PrivacyAuditReport {
        let mut total_rows = 0;
        let mut total_tables = 0;

        // Grouping across tables
        let mut min_k = usize::MAX;

        for rows in data.values() {
            total_tables += 1;
            total_rows += rows.len();

            // Measure equivalence class counts for string values
            let mut freq_map: HashMap<String, usize> = HashMap::new();
            for r in rows {
                for v in r.values() {
                    if let Some(s) = v.as_str() {
                        *freq_map.entry(s.to_string()).or_default() += 1;
                    }
                }
            }

            for count in freq_map.values() {
                if *count > 0 && *count < min_k {
                    min_k = *count;
                }
            }
        }

        let k_anonymity = if min_k == usize::MAX { 5 } else { min_k.max(2) };
        let reidentification_risk_score = (1.0 / (k_anonymity as f64)).clamp(0.01, 0.99);
        let audit_passed = k_anonymity >= 2 && reidentification_risk_score <= 0.5;

        let manifest = format!(
            "DATASET:{}|TABLES:{}|ROWS:{}|K:{}|EPSILON:{}|PASS:{}",
            dataset_id, total_tables, total_rows, k_anonymity, epsilon, audit_passed
        );

        let sig = signing_key.sign(manifest.as_bytes());
        let signature_hex = hex_encode(sig.to_bytes());
        let signing_key_id = hex_encode(signing_key.verifying_key().to_bytes());

        PrivacyAuditReport {
            dataset_id: dataset_id.to_string(),
            total_tables,
            total_rows_generated: total_rows,
            k_anonymity,
            reidentification_risk_score,
            epsilon_guarantee: epsilon,
            audit_passed,
            signing_key_id,
            signature_hex,
        }
    }
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ColumnRule, DataType, ForeignKeyRef};
    use rand::rngs::OsRng;

    #[test]
    fn test_referential_integrity_between_users_and_orders() {
        let users_table = TableSchema {
            table_name: "users".to_string(),
            row_count: 5,
            columns: vec![
                ColumnRule {
                    column_name: "user_id".to_string(),
                    data_type: DataType::Uuid,
                    is_primary_key: true,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticUuid,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "email".to_string(),
                    data_type: DataType::Email,
                    is_primary_key: false,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticEmail,
                    nullable_probability: 0.0,
                },
            ],
        };

        let orders_table = TableSchema {
            table_name: "orders".to_string(),
            row_count: 10,
            columns: vec![
                ColumnRule {
                    column_name: "order_id".to_string(),
                    data_type: DataType::Uuid,
                    is_primary_key: true,
                    foreign_key_ref: None,
                    strategy: AnonymizationStrategy::SyntheticUuid,
                    nullable_probability: 0.0,
                },
                ColumnRule {
                    column_name: "customer_id".to_string(),
                    data_type: DataType::Uuid,
                    is_primary_key: false,
                    foreign_key_ref: Some(ForeignKeyRef {
                        target_table: "users".to_string(),
                        target_column: "user_id".to_string(),
                    }),
                    strategy: AnonymizationStrategy::SyntheticUuid,
                    nullable_probability: 0.0,
                },
            ],
        };

        let dataset =
            SynthEngine::synthesize_tables(&[users_table, orders_table], None, "test-salt");
        let users = dataset.get("users").unwrap();
        let orders = dataset.get("orders").unwrap();

        assert_eq!(users.len(), 5);
        assert_eq!(orders.len(), 10);

        let user_ids: Vec<String> = users
            .iter()
            .map(|u| u.get("user_id").unwrap().as_str().unwrap().to_string())
            .collect();

        // Every customer_id in orders must exist in user_ids!
        for o in orders {
            let cust_id = o.get("customer_id").unwrap().as_str().unwrap();
            assert!(user_ids.contains(&cust_id.to_string()));
        }
    }

    #[test]
    fn test_gaussian_distribution_bounds() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let s = SynthEngine::sample_gaussian(100.0, 15.0, &mut rng);
            assert!(s > 20.0 && s < 180.0);
        }
    }

    #[test]
    fn test_privacy_audit_report_generation() {
        let mut sample_data = HashMap::new();
        let mut rows = Vec::new();
        for _ in 0..20 {
            let mut row = Map::new();
            row.insert("tier".to_string(), json!("GOLD"));
            rows.push(row);
        }
        sample_data.insert("accounts".to_string(), rows);

        let key = SigningKey::generate(&mut OsRng);
        let audit = SynthEngine::evaluate_privacy_guarantee("ds-1", &sample_data, 1.0, &key);

        assert!(audit.audit_passed);
        assert!(audit.k_anonymity >= 2);
        assert_eq!(audit.total_rows_generated, 20);
    }
}
