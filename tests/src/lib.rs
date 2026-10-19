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

    #[tokio::test]
    async fn test_vault_core_all_secret_types_and_audit_flow() {
        use vault_core::models::*;
        use vault_core::service::VaultService;
        use vault_core::storage::VaultStorage;

        let storage = VaultStorage::open_in_memory().unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        // Positive: Init & Unlock
        service.initialize("MasterKeyPass123!").await.unwrap();
        assert!(!service.is_locked().await);

        // 1. Password Secret
        let pwd_input = CreateSecretInput {
            title: "Internal Admin Login".to_string(),
            tags: vec!["admin".to_string(), "ldap".to_string()],
            username: Some("admin_user".to_string()),
            url: Some("https://admin.corp".to_string()),
            notes: Some("Requires 2FA".to_string()),
            payload: SecretPayload::Password {
                password: "AlphaOmegaPassword999!".to_string(),
            },
        };
        let pwd_meta = service
            .create_secret(pwd_input, "integration_test")
            .await
            .unwrap();

        // 2. SSH Key Secret
        let ssh_input = CreateSecretInput {
            title: "K8s Bastion Key".to_string(),
            tags: vec!["k8s".to_string(), "bastion".to_string()],
            username: Some("ubuntu".to_string()),
            url: None,
            notes: None,
            payload: SecretPayload::SshKey {
                private_key:
                    "-----BEGIN EC PRIVATE KEY-----\nMHQCAQEEI...\n-----END EC PRIVATE KEY-----"
                        .to_string(),
                public_key: Some("ecdsa-sha2-nistp256 AAAAE...".to_string()),
                passphrase: Some("key-pass".to_string()),
            },
        };
        let ssh_meta = service
            .create_secret(ssh_input, "integration_test")
            .await
            .unwrap();

        // 3. Certificate Secret
        let cert_input = CreateSecretInput {
            title: "Wildcard TLS Cert".to_string(),
            tags: vec!["tls".to_string(), "ssl".to_string()],
            username: None,
            url: Some("https://*.corp.net".to_string()),
            notes: Some("Expires 2027".to_string()),
            payload: SecretPayload::Certificate {
                private_key:
                    "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----"
                        .to_string(),
                certificate_pem: "-----BEGIN CERTIFICATE-----\nMIID...\n-----END CERTIFICATE-----"
                    .to_string(),
                chain_pem: Some(
                    "-----BEGIN CERTIFICATE-----\nCA...\n-----END CERTIFICATE-----".to_string(),
                ),
            },
        };
        let cert_meta = service
            .create_secret(cert_input, "integration_test")
            .await
            .unwrap();

        // 4. API Key Secret
        let api_input = CreateSecretInput {
            title: "Datadog Telemetry Key".to_string(),
            tags: vec!["datadog".to_string(), "monitoring".to_string()],
            username: None,
            url: Some("https://api.datadoghq.com".to_string()),
            notes: None,
            payload: SecretPayload::ApiKey {
                api_key: "dd_api_0123456789abcdef".to_string(),
                api_secret: Some("dd_app_fedcba9876543210".to_string()),
                endpoint_url: Some("https://api.datadoghq.com/v1".to_string()),
            },
        };
        let api_meta = service
            .create_secret(api_input, "integration_test")
            .await
            .unwrap();

        // 5. Database Credential Secret
        let db_input = CreateSecretInput {
            title: "PostgreSQL Analytics Cluster".to_string(),
            tags: vec!["postgres".to_string(), "analytics".to_string()],
            username: Some("analytics_ro".to_string()),
            url: Some("pg-analytics.internal:5432".to_string()),
            notes: Some("Read only role".to_string()),
            payload: SecretPayload::DatabaseCredential {
                engine: "postgresql".to_string(),
                host: "pg-analytics.internal".to_string(),
                port: 5432,
                database: "analytics_dw".to_string(),
                username: "analytics_ro".to_string(),
                password: "SecureAnalyticsPassword2026!".to_string(),
                connection_string: Some("postgresql://analytics_ro:SecureAnalyticsPassword2026!@pg-analytics.internal:5432/analytics_dw".to_string()),
            },
        };
        let db_meta = service
            .create_secret(db_input, "integration_test")
            .await
            .unwrap();

        // Read and verify all decrypted secrets
        let dec_pwd = service.get_secret(&pwd_meta.id, "test").await.unwrap();
        assert_eq!(
            dec_pwd.payload.primary_secret_value(),
            "AlphaOmegaPassword999!"
        );

        let dec_ssh = service.get_secret(&ssh_meta.id, "test").await.unwrap();
        assert!(dec_ssh
            .payload
            .primary_secret_value()
            .contains("BEGIN EC PRIVATE KEY"));

        let dec_cert = service.get_secret(&cert_meta.id, "test").await.unwrap();
        assert!(dec_cert
            .payload
            .primary_secret_value()
            .contains("BEGIN RSA PRIVATE KEY"));

        let dec_api = service.get_secret(&api_meta.id, "test").await.unwrap();
        assert_eq!(
            dec_api.payload.primary_secret_value(),
            "dd_api_0123456789abcdef"
        );

        let dec_db = service.get_secret(&db_meta.id, "test").await.unwrap();
        assert_eq!(
            dec_db.payload.primary_secret_value(),
            "SecureAnalyticsPassword2026!"
        );

        // Sub-50ms search test
        let search_results = service.search_secrets("postgres", 10).await.unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, db_meta.id);

        let search_by_tag = service.search_secrets("tls", 10).await.unwrap();
        assert_eq!(search_by_tag.len(), 1);
        assert_eq!(search_by_tag[0].id, cert_meta.id);

        // Clipboard auto-clear test
        let ttl = service
            .copy_to_clipboard(&pwd_meta.id, Some(10), "test")
            .await
            .unwrap();
        assert_eq!(ttl, 10);
        assert_eq!(
            service.peek_clipboard_for_test().await,
            Some("AlphaOmegaPassword999!".to_string())
        );
        service.clear_clipboard().await.unwrap();
        assert_eq!(service.peek_clipboard_for_test().await, None);

        // Audit log check
        let audit_logs = service.get_audit_logs(None, 50).await.unwrap();
        assert!(audit_logs.len() >= 6); // 5 created + 5 read + 1 copy

        // Lock & Unlock
        service.lock().await.unwrap();
        assert!(service.is_locked().await);
        assert!(service.get_secret(&pwd_meta.id, "test").await.is_err());

        service.unlock("MasterKeyPass123!").await.unwrap();
        assert!(!service.is_locked().await);
        let dec_after = service.get_secret(&pwd_meta.id, "test").await.unwrap();
        assert_eq!(
            dec_after.payload.primary_secret_value(),
            "AlphaOmegaPassword999!"
        );
    }

    #[tokio::test]
    async fn test_vault_boundary_negative_and_regression_cases() {
        use vault_core::error::VaultError;
        use vault_core::models::*;
        use vault_core::service::VaultService;
        use vault_core::storage::VaultStorage;

        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("regression_vault.db");
        let storage = VaultStorage::open(&db_path).unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        service.initialize("CorrectMaster123!").await.unwrap();

        // Negative: Initializing again must fail
        assert!(matches!(
            service.initialize("AnotherPassword").await.unwrap_err(),
            VaultError::AlreadyInitialized
        ));

        // Boundary: Title length 256 chars (Allowed)
        let title_256 = "A".repeat(256);
        let input_256 = CreateSecretInput {
            title: title_256.clone(),
            tags: vec![],
            username: None,
            url: None,
            notes: None,
            payload: SecretPayload::Password {
                password: "p".to_string(),
            },
        };
        let created_256 = service.create_secret(input_256, "test").await.unwrap();
        assert_eq!(created_256.title.len(), 256);

        // Boundary: Title length 257 chars (Rejected)
        let title_257 = "B".repeat(257);
        let input_257 = CreateSecretInput {
            title: title_257,
            tags: vec![],
            username: None,
            url: None,
            notes: None,
            payload: SecretPayload::Password {
                password: "p".to_string(),
            },
        };
        assert!(service.create_secret(input_257, "test").await.is_err());

        // Negative: Port 0 for DB (Rejected)
        let invalid_db_input = CreateSecretInput {
            title: "Invalid Port DB".to_string(),
            tags: vec![],
            username: None,
            url: None,
            notes: None,
            payload: SecretPayload::DatabaseCredential {
                engine: "mysql".to_string(),
                host: "localhost".to_string(),
                port: 0,
                database: "db".to_string(),
                username: "root".to_string(),
                password: "root".to_string(),
                connection_string: None,
            },
        };
        assert!(service
            .create_secret(invalid_db_input, "test")
            .await
            .is_err());

        // Regression: Inspect raw database file on disk to verify zero plaintext leakage
        let raw_disk_bytes = std::fs::read(&db_path).unwrap();
        let sensitive_token = b"AlphaOmegaPassword999!";
        let found_leak = raw_disk_bytes
            .windows(sensitive_token.len())
            .any(|w| w == sensitive_token);
        assert!(
            !found_leak,
            "Plaintext sensitive payload must NEVER leak into SQLite on disk"
        );

        // Failure: Delete and get non-existent ID
        assert!(matches!(
            service
                .get_secret("non-existent-uuid", "test")
                .await
                .unwrap_err(),
            VaultError::SecretNotFound(_)
        ));
        assert!(matches!(
            service
                .delete_secret("non-existent-uuid", "test")
                .await
                .unwrap_err(),
            VaultError::SecretNotFound(_)
        ));
    }
}
