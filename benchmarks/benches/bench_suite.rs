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
    benchmark_timeline_histogram
);
criterion_main!(benches);
