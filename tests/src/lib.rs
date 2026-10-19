#![forbid(unsafe_code)]
//! Cross-Crate Integration Tests
//!
//! Validates end-to-end integration across all shared subsystems:
//! storage, encryption, search, indexing, telemetry, settings, and updater.

#[cfg(test)]
mod integration_tests {
    use platform_common::PlatformInfo;
    use shared_encryption::{
        decrypt_aes_gcm, derive_key_argon2id, encrypt_aes_gcm, generate_salt, Argon2Params,
    };
    use shared_indexing::{ChunkingPipeline, ConcurrentCrawler, WalkerOptions};
    use shared_search::{SearchDocument, SearchEngine};
    use shared_settings::{AppSettings, SettingsManager};
    use shared_storage::{Database, Migration, MigrationRunner, MmapReader};
    use shared_telemetry::LocalTelemetry;
    use shared_updater::{StandaloneUpdater, UpdateManifest};

    #[tokio::test]
    async fn test_devops_vault_end_to_end_flow() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("vault.db");

        // 1. Key Derivation via Argon2id
        let passphrase = b"MasterPassphrase2026";
        let salt = generate_salt();
        let params = Argon2Params::fast_for_tests();
        let master_key = derive_key_argon2id(passphrase, &salt, &params).unwrap();

        // 2. Encrypt Payload via AES-256-GCM
        let raw_secret = b"postgresql://admin:super_secret_password@localhost:5432/production";
        let encrypted_payload = encrypt_aes_gcm(&master_key, raw_secret).unwrap();

        // 3. Persist in SQLite with WAL Pragmas & Migrations
        let db = Database::open(&db_path).unwrap();
        let migrations = vec![Migration {
            version: 1,
            description: "initial vault schema",
            sql: "CREATE TABLE secrets (id TEXT PRIMARY KEY, title TEXT, category TEXT, payload BLOB);",
        }];
        MigrationRunner::apply_migrations(&db, &migrations).unwrap();

        db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO secrets (id, title, category, payload) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    "sec_pg_01",
                    "Prod PostgreSQL Connection",
                    "database",
                    &encrypted_payload
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // 4. Index in Tantivy Search Engine
        let search_engine = SearchEngine::create_in_memory().unwrap();
        search_engine
            .add_document(SearchDocument {
                id: "sec_pg_01".to_string(),
                title: "Prod PostgreSQL Connection".to_string(),
                content: "database connection credentials postgres".to_string(),
                category: "database".to_string(),
                timestamp: 1729300000,
            })
            .unwrap();
        search_engine.commit().unwrap();

        // 5. Search in Tantivy
        let hits = search_engine.search("PostgreSQL", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "sec_pg_01");

        // 6. Retrieve from SQLite and Decrypt
        let fetched_payload: Vec<u8> = db
            .with_connection(|conn| {
                conn.query_row(
                    "SELECT payload FROM secrets WHERE id = ?1",
                    ["sec_pg_01"],
                    |row| row.get(0),
                )
            })
            .unwrap();

        let decrypted = decrypt_aes_gcm(&master_key, &fetched_payload).unwrap();
        assert_eq!(decrypted.as_slice(), raw_secret);
    }

    #[tokio::test]
    async fn test_ai_explorer_crawl_and_index_flow() {
        let temp = tempfile::tempdir().unwrap();
        let docs_dir = temp.path().join("documents");
        std::fs::create_dir_all(&docs_dir).unwrap();

        let f1 = docs_dir.join("financial_report_2026.pdf");
        let f2 = docs_dir.join("architecture_overview.md");
        std::fs::write(&f1, "Q3 Profit and Balance Sheet").unwrap();
        std::fs::write(&f2, "Offline First System Design").unwrap();

        // Crawl filesystem
        let (_cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let rx = ConcurrentCrawler::crawl_stream(&docs_dir, WalkerOptions::default(), cancel_rx);

        let chunks = ChunkingPipeline::collect_chunks(rx, 10).await;
        assert!(!chunks.is_empty());

        // Index discovered files into Tantivy
        let search_engine = SearchEngine::create_in_memory().unwrap();
        for chunk in chunks {
            for record in chunk {
                if !record.is_dir {
                    search_engine
                        .add_document(SearchDocument {
                            id: record.path.to_string_lossy().to_string(),
                            title: record.file_name.clone(),
                            content: format!("File extension: {:?}", record.extension),
                            category: "file".to_string(),
                            timestamp: record.modified_timestamp_secs as i64,
                        })
                        .unwrap();
                }
            }
        }
        search_engine.commit().unwrap();

        // Query file
        let results = search_engine.search("financial", 5).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("financial_report_2026"));
    }

    #[test]
    fn test_log_viewer_mmap_and_telemetry_flow() {
        let temp = tempfile::tempdir().unwrap();
        let log_file = temp.path().join("server.log");
        let log_data = b"2026-10-19 10:00:00.001 [INFO] Server started\n2026-10-19 10:00:01.042 [ERROR] Database timeout occurred\n";
        std::fs::write(&log_file, log_data).unwrap();

        // Mmap reading
        let reader = MmapReader::open(&log_file).unwrap();
        assert_eq!(reader.len(), log_data.len());
        let slice = reader.slice(0, reader.len()).unwrap();
        let text = String::from_utf8_lossy(slice);
        assert!(text.contains("[ERROR]"));

        // Opt-in telemetry monitoring
        let tel = LocalTelemetry::new(true);
        tel.record_counter("lines_read", 2);
        tel.record_duration_ms("open_latency", 1.2);

        let snap = tel.snapshot().unwrap();
        assert_eq!(*snap.counters.get("lines_read").unwrap(), 2);
    }

    #[tokio::test]
    async fn test_settings_and_updater_flow() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("app_settings.json");

        let manager = SettingsManager::<AppSettings>::load_or_default(&settings_path).unwrap();
        let current = manager.get().await;
        assert_eq!(current.theme, "dark");

        // Verify platform info
        let info = PlatformInfo::current();
        assert!(!info.is_portable || info.is_portable);

        // Verify standalone updater package verification
        let (signing_key, verifying_key) = shared_encryption::generate_ed25519_keypair();
        let payload = b"Universal Log Viewer release binary v1.0.0 payload";
        let hash = StandaloneUpdater::compute_sha256(payload);
        let sig = shared_encryption::sign_ed25519(&signing_key, payload);
        let sig_hex = sig.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        let manifest = UpdateManifest {
            version: "1.0.0".to_string(),
            target_os: "windows".to_string(),
            target_arch: "x86_64".to_string(),
            sha256: hash,
            signature_ed25519: sig_hex,
            release_notes: "Initial MVP release".to_string(),
        };

        let result =
            StandaloneUpdater::verify_package(&manifest, payload, &verifying_key.to_bytes());
        assert!(result.is_ok());
    }
}
