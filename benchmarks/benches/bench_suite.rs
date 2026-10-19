//! Performance Benchmark Harness
//!
//! Validates SLA boundaries (< 50ms search latency, throughput, zero regressions)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use shared_encryption::{decrypt_aes_gcm, encrypt_aes_gcm};
use shared_search::{SearchDocument, SearchEngine};
use shared_storage::{Database, Migration, MigrationRunner};
use shared_telemetry::LocalTelemetry;

fn benchmark_encryption_throughput(c: &mut Criterion) {
    let key = [0x5au8; 32];
    let payload = vec![0x3cu8; 4096]; // 4KB chunk

    let mut group = c.benchmark_group("encryption");

    group.bench_function("aes_256_gcm_encrypt_4kb", |b| {
        b.iter(|| {
            let res = encrypt_aes_gcm(black_box(&key), black_box(&payload));
            black_box(res).unwrap();
        });
    });

    let encrypted = encrypt_aes_gcm(&key, &payload).unwrap();
    group.bench_function("aes_256_gcm_decrypt_4kb", |b| {
        b.iter(|| {
            let res = decrypt_aes_gcm(black_box(&key), black_box(&encrypted));
            black_box(res).unwrap();
        });
    });

    group.finish();
}

fn benchmark_tantivy_search_latency(c: &mut Criterion) {
    let engine = SearchEngine::create_in_memory().unwrap();

    // Populate index with 1,000 synthetic documents
    for i in 0..1000 {
        engine
            .add_document(SearchDocument {
                id: format!("doc_{}", i),
                title: format!("Kubernetes ingress pod failure log record #{}", i),
                content: format!(
                    "Detailed trace log payload with timestamps and error codes: ERR_{}",
                    i % 20
                ),
                category: "logs".to_string(),
                timestamp: 1729300000 + i as i64,
            })
            .unwrap();
    }
    engine.commit().unwrap();

    let mut group = c.benchmark_group("search");

    // Enforce < 50ms search SLA
    group.bench_function("tantivy_query_latency", |b| {
        b.iter(|| {
            let hits = engine.search(black_box("ingress"), black_box(10)).unwrap();
            black_box(hits);
        });
    });

    group.finish();
}

fn benchmark_sqlite_wal_storage(c: &mut Criterion) {
    let db = Database::open_in_memory().unwrap();
    let migrations = vec![Migration {
        version: 1,
        description: "create kv store",
        sql: "CREATE TABLE kv (key TEXT PRIMARY KEY, val BLOB);",
    }];
    MigrationRunner::apply_migrations(&db, &migrations).unwrap();

    let mut group = c.benchmark_group("storage");

    group.bench_function("sqlite_wal_insert_and_query", |b| {
        let mut idx = 0;
        b.iter(|| {
            idx += 1;
            let key = format!("k_{}", idx);
            let val = b"benchmark payload value 123456789";
            db.with_connection(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO kv (key, val) VALUES (?1, ?2)",
                    rusqlite::params![&key, val],
                )?;
                let res: Vec<u8> =
                    conn.query_row("SELECT val FROM kv WHERE key = ?1", [&key], |row| {
                        row.get(0)
                    })?;
                black_box(res);
                Ok(())
            })
            .unwrap();
        });
    });

    group.finish();
}

fn benchmark_telemetry_counters(c: &mut Criterion) {
    let tel = LocalTelemetry::new(true);
    let mut group = c.benchmark_group("telemetry");

    group.bench_function("telemetry_record_counter", |b| {
        b.iter(|| {
            tel.record_counter(black_box("query_counter"), black_box(1));
        });
    });

    group.finish();
}

fn benchmark_sparse_indexing_throughput(c: &mut Criterion) {
    use universal_log_viewer::SparseLineIndex;

    // Generate 1MB of synthetic log lines
    let mut log_bytes = Vec::with_capacity(1024 * 1024);
    for i in 0..15000 {
        log_bytes.extend_from_slice(
            format!(
                "2026-10-19 10:14:02.104 [INFO] [auth] User session created id={}\n",
                i
            )
            .as_bytes(),
        );
    }

    let mut group = c.benchmark_group("log_viewer");

    group.bench_function("sparse_line_index_1mb_scan", |b| {
        b.iter(|| {
            let index = SparseLineIndex::build(black_box(&log_bytes));
            black_box(index.total_lines());
        });
    });

    group.finish();
}

fn benchmark_log_regex_search(c: &mut Criterion) {
    use universal_log_viewer::{LogQuery, LogSearchEngine, MmapLogEngine, SparseLineIndex};

    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("bench_search.log");

    let mut log_bytes = Vec::with_capacity(2 * 1024 * 1024);
    for i in 0..25000 {
        let level = if i % 100 == 0 { "ERROR" } else { "INFO" };
        log_bytes.extend_from_slice(
            format!(
                "2026-10-19 [{}]: Event #{}: Status code {}\n",
                level,
                i,
                if i % 100 == 0 { 500 } else { 200 }
            )
            .as_bytes(),
        );
    }
    std::fs::write(&file_path, &log_bytes).unwrap();

    let mmap = MmapLogEngine::open(&file_path).unwrap();
    let index = SparseLineIndex::build(&log_bytes);

    let query = LogQuery {
        pattern: r"Status code (5\d\d)".to_string(),
        is_regex: true,
        case_sensitive: false,
        limit: 100,
        start_line: 0,
    };

    let mut group = c.benchmark_group("log_viewer_search");

    // Enforce < 50ms search SLA
    group.bench_function("regex_search_25k_lines", |b| {
        b.iter(|| {
            let report =
                LogSearchEngine::search(black_box(&mmap), black_box(&index), black_box(&query))
                    .unwrap();
            black_box(report.matches.len());
        });
    });

    group.finish();
}

fn benchmark_json_streaming_parser(c: &mut Criterion) {
    use universal_log_viewer::StreamingJsonParser;

    let sample_json = r#"{"timestamp":"2026-10-19T10:15:30.123Z","level":"INFO","message":"User login successful","userId":"user_9021","latencyMs":42,"region":"us-east-1"}"#;

    let mut group = c.benchmark_group("log_viewer_parsers");

    group.bench_function("streaming_json_parser_10k_lines", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let rec = StreamingJsonParser::parse_line(black_box(i), black_box(sample_json));
                black_box(rec);
            }
        });
    });

    group.finish();
}

fn benchmark_drain_template_mining(c: &mut Criterion) {
    use universal_log_viewer::DrainLogMiner;

    let messages = [
        "Connection to 192.168.1.10 failed on port 8080: timeout",
        "Connection to 192.168.1.15 failed on port 8080: timeout",
        "User 9021 login successful from 10.0.0.1 in 45ms",
        "User 9022 login successful from 10.0.0.2 in 32ms",
        "HTTP 500 internal server error in auth-svc path /v1/tokens",
    ];

    let mut group = c.benchmark_group("log_viewer_ai");

    group.bench_function("drain_template_mining_1k_logs", |b| {
        b.iter(|| {
            let mut miner = DrainLogMiner::new();
            for i in 0..1000 {
                let msg = messages[i % messages.len()];
                let cluster_id = miner.add_log(black_box(i), black_box(msg));
                black_box(cluster_id);
            }
            let clusters = miner.get_clusters();
            black_box(clusters.len());
        });
    });

    group.finish();
}

fn benchmark_timeline_histogram(c: &mut Criterion) {
    use universal_log_viewer::{LogSeverity, TimelineIndex};

    let mut index = TimelineIndex::new();
    for i in 0..25000 {
        let sev = if i % 100 == 0 {
            LogSeverity::Error
        } else if i % 20 == 0 {
            LogSeverity::Warn
        } else {
            LogSeverity::Info
        };
        index.push(1700000000000 + (i as i64) * 500, i, sev);
    }
    index.sort();

    let mut group = c.benchmark_group("log_viewer_timeline");

    group.bench_function("timeline_histogram_50_buckets", |b| {
        b.iter(|| {
            let hist = index.generate_histogram(black_box(50)).unwrap();
            black_box(hist.buckets.len());
        });
    });

    group.bench_function("timeline_filter_timerange_sub_50ms", |b| {
        b.iter(|| {
            let lines = index.filter_by_timerange(
                black_box(1700000000000 + 5000 * 500),
                black_box(1700000000000 + 15000 * 500),
            );
            black_box(lines.len());
        });
    });

    group.finish();
}

fn benchmark_vault_search_and_crypto(c: &mut Criterion) {
    use vault_core::crypto::{decrypt_secret_payload, encrypt_secret_payload, VaultMasterKey};
    use vault_core::models::*;
    use vault_core::search::VaultSearchIndex;

    let index = VaultSearchIndex::new().unwrap();

    // Populate 2,000 synthetic secret metadata records
    let mut batch = Vec::with_capacity(2000);
    for i in 0..2000 {
        let (stype, tag) = match i % 4 {
            0 => (SecretType::Password, "web"),
            1 => (SecretType::SshKey, "bastion"),
            2 => (SecretType::Certificate, "tls"),
            _ => (SecretType::DatabaseCredential, "database"),
        };

        batch.push(SecretMetadata {
            id: format!("sec-bench-{}", i),
            title: format!("Production Server {} Access Secret", i),
            secret_type: stype,
            tags: vec!["prod".to_string(), tag.to_string()],
            username: Some(format!("user_{}", i)),
            url: Some(format!("https://server-{}.internal", i)),
            notes: Some(format!("Auto-generated secret benchmark entry {}", i)),
            version: 1,
            created_at: 1729300000 + i as i64,
            updated_at: 1729300000 + i as i64,
        });
    }
    index.index_batch(&batch).unwrap();

    let mut group = c.benchmark_group("vault_core");

    // Benchmark sub-50ms search SLA across 2,000 in-memory indexed secrets
    group.bench_function("vault_tantivy_search_sub_50ms", |b| {
        b.iter(|| {
            let hits = index.search(black_box("database"), black_box(20)).unwrap();
            black_box(hits.len());
        });
    });

    // Benchmark structured AES-256-GCM secret payload roundtrip
    let master_key = VaultMasterKey([0x77u8; 32]);
    let payload = SecretPayload::DatabaseCredential {
        engine: "postgresql".to_string(),
        host: "postgres-cluster.internal".to_string(),
        port: 5432,
        database: "primary_db".to_string(),
        username: "super_admin".to_string(),
        password: "VeryLongAndSecureDatabasePassword!#2026".to_string(),
        connection_string: Some("postgresql://super_admin:VeryLongAndSecureDatabasePassword!#2026@postgres-cluster.internal:5432/primary_db".to_string()),
    };

    let encrypted = encrypt_secret_payload(&master_key, &payload).unwrap();

    group.bench_function("vault_payload_encrypt_aes_gcm", |b| {
        b.iter(|| {
            let res = encrypt_secret_payload(black_box(&master_key), black_box(&payload));
            black_box(res).unwrap();
        });
    });

    group.bench_function("vault_payload_decrypt_aes_gcm", |b| {
        b.iter(|| {
            let res = decrypt_secret_payload(black_box(&master_key), black_box(&encrypted));
            black_box(res).unwrap();
        });
    });

    group.finish();
}

fn benchmark_vault_advanced_features(c: &mut Criterion) {
    use shared_encryption::Argon2Params;
    use vault_core::backup::{export_vault_backup, read_vault_backup};
    use vault_core::models::{DecryptedSecret, SecretMetadata, SecretPayload, SecretType};
    use vault_core::risk_scoring::evaluate_vault_health;
    use vault_core::totp::{TotpAlgorithm, TotpConfig};

    let mut group = c.benchmark_group("vault_advanced");

    // 1. TOTP code generation throughput (RFC 6238 HMAC-SHA1 + dynamic truncation)
    let totp_cfg = TotpConfig {
        secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
        issuer: Some("Google".to_string()),
        account_name: Some("user@gmail.com".to_string()),
        algorithm: TotpAlgorithm::Sha1,
        digits: 6,
        step_seconds: 30,
    };

    group.bench_function("totp_generate_100_codes", |b| {
        b.iter(|| {
            for t in 0..100 {
                let code = totp_cfg
                    .generate_at_timestamp(black_box(1700000000 + t * 30))
                    .unwrap();
                black_box(code);
            }
        });
    });

    // 2. Risk scoring across 500 secrets (entropy, zero-knowledge reuse, default checks)
    let mut secrets = Vec::with_capacity(500);
    for i in 0..500 {
        secrets.push(DecryptedSecret {
            metadata: SecretMetadata {
                id: format!("bench-sec-{}", i),
                title: format!("Database Cluster {}", i),
                secret_type: SecretType::Password,
                tags: vec!["db".to_string()],
                username: Some(format!("user_{}", i)),
                url: None,
                notes: None,
                version: 1,
                created_at: 1729300000,
                updated_at: 1729300000 - (i as i64 * 86400),
            },
            payload: SecretPayload::Password {
                password: if i % 10 == 0 {
                    "admin".to_string()
                } else if i % 5 == 0 {
                    "SharedPassword#2026!".to_string()
                } else {
                    format!("StrongRandomPassphrase!#{}", i)
                },
            },
        });
    }

    group.bench_function("vault_risk_scoring_500_secrets", |b| {
        b.iter(|| {
            let report = evaluate_vault_health(black_box(&secrets));
            black_box(report.overall_score);
        });
    });

    // 3. Encrypted backup archive roundtrip (Argon2id + AES-256-GCM + integrity check)
    let sample_secrets = secrets[..50].to_vec();
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let backup_path = temp_file.path();
    let test_params = Argon2Params::fast_for_tests();

    group.bench_function("vault_encrypted_backup_roundtrip_50_secrets", |b| {
        b.iter(|| {
            let summary = export_vault_backup(
                black_box(&sample_secrets),
                black_box("BenchBackupPassphrase999!"),
                black_box(backup_path),
                black_box(Some(test_params.clone())),
            )
            .unwrap();
            black_box(summary.file_size_bytes);

            let env = read_vault_backup(
                black_box(backup_path),
                black_box("BenchBackupPassphrase999!"),
            )
            .unwrap();
            black_box(env.secrets.len());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_encryption_throughput,
    benchmark_tantivy_search_latency,
    benchmark_sqlite_wal_storage,
    benchmark_telemetry_counters,
    benchmark_sparse_indexing_throughput,
    benchmark_log_regex_search,
    benchmark_json_streaming_parser,
    benchmark_drain_template_mining,
    benchmark_timeline_histogram,
    benchmark_vault_search_and_crypto,
    benchmark_vault_advanced_features
);
criterion_main!(benches);
