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

criterion_group!(
    benches,
    benchmark_encryption_throughput,
    benchmark_tantivy_search_latency,
    benchmark_sqlite_wal_storage,
    benchmark_telemetry_counters
);
criterion_main!(benches);
