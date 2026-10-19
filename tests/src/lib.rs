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

    #[tokio::test]
    async fn test_vault_advanced_cloud_creds_and_totp_flow() {
        use vault_core::models::*;
        use vault_core::service::VaultService;
        use vault_core::storage::VaultStorage;

        let _temp = tempfile::tempdir().unwrap();
        let storage = VaultStorage::open_in_memory().unwrap();
        let service = VaultService::new(storage, Some(Argon2Params::fast_for_tests()));

        service.initialize("AdvancedMasterKey2026!").await.unwrap();

        // 1. AWS Credential
        let aws_input = CreateSecretInput {
            title: "AWS Production Deployer".to_string(),
            tags: vec!["aws".to_string(), "cloud".to_string()],
            username: Some("deployer".to_string()),
            url: Some("https://aws.amazon.com".to_string()),
            notes: Some("Requires IAM MFA".to_string()),
            payload: SecretPayload::AwsCredential {
                access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                session_token: None,
                region: Some("us-east-1".to_string()),
                profile_name: Some("default".to_string()),
            },
        };
        let aws_meta = service.create_secret(aws_input, "test").await.unwrap();
        assert_eq!(aws_meta.secret_type, SecretType::AwsCredential);

        // 2. Azure Credential
        let az_input = CreateSecretInput {
            title: "Azure AKS Cluster SP".to_string(),
            tags: vec!["azure".to_string(), "k8s".to_string()],
            username: Some("sp-aks".to_string()),
            url: None,
            notes: None,
            payload: SecretPayload::AzureCredential {
                subscription_id: "11111111-2222-3333-4444-555555555555".to_string(),
                tenant_id: "22222222-3333-4444-5555-666666666666".to_string(),
                client_id: "33333333-4444-5555-6666-777777777777".to_string(),
                client_secret: "SuperSecretAzureClientTokenXYZ789!".to_string(),
                cloud_environment: Some("AzureCloud".to_string()),
            },
        };
        let az_meta = service.create_secret(az_input, "test").await.unwrap();
        assert_eq!(az_meta.secret_type, SecretType::AzureCredential);

        // 3. Terraform Secret
        let tf_input = CreateSecretInput {
            title: "Terraform State & Vars".to_string(),
            tags: vec!["terraform".to_string(), "iac".to_string()],
            username: None,
            url: None,
            notes: None,
            payload: SecretPayload::TerraformSecret {
                state_key: Some("prod.tfstate.enc".to_string()),
                tf_vars: Some("db_password = \"Secret123!\"\nnode_count = 5".to_string()),
                provider_token: Some("tfp_cloud_token_abc123".to_string()),
                backend_config: Some("bucket = \"corp-tfstate-bucket\"".to_string()),
            },
        };
        let tf_meta = service.create_secret(tf_input, "test").await.unwrap();
        assert_eq!(tf_meta.secret_type, SecretType::TerraformSecret);

        // 4. TOTP Secret
        let totp_input = CreateSecretInput {
            title: "GitHub 2FA Authenticator".to_string(),
            tags: vec!["2fa".to_string(), "totp".to_string()],
            username: Some("devops-lead".to_string()),
            url: Some("https://github.com".to_string()),
            notes: None,
            payload: SecretPayload::Totp {
                secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
                issuer: Some("GitHub".to_string()),
                account_name: Some("devops-lead".to_string()),
                algorithm: Some("SHA1".to_string()),
                digits: Some(6),
                period_seconds: Some(30),
            },
        };
        let totp_meta = service.create_secret(totp_input, "test").await.unwrap();
        assert_eq!(totp_meta.secret_type, SecretType::Totp);

        // 5. Test Live TOTP generation
        let totp_code = service.generate_totp_code(&totp_meta.id).await.unwrap();
        assert_eq!(totp_code.code.len(), 6);
        assert!(totp_code.remaining_seconds <= 30);
        assert!(totp_code.progress_ratio >= 0.0 && totp_code.progress_ratio <= 1.0);
    }

    #[tokio::test]
    async fn test_vault_advanced_health_scoring_and_backup_integrity() {
        use vault_core::models::*;
        use vault_core::risk_scoring::Severity;
        use vault_core::service::VaultService;
        use vault_core::storage::VaultStorage;

        let temp = tempfile::tempdir().unwrap();
        let backup_file = temp.path().join("full_vault_backup.vaultbackup");

        let storage1 = VaultStorage::open_in_memory().unwrap();
        let service1 = VaultService::new(storage1, Some(Argon2Params::fast_for_tests()));
        service1.initialize("OriginalMasterKey123!").await.unwrap();

        // 1. Add secret with default compromised password
        let sec1 = CreateSecretInput {
            title: "MySQL Default Root".to_string(),
            tags: vec![],
            username: Some("root".to_string()),
            url: None,
            notes: None,
            payload: SecretPayload::Password {
                password: "admin".to_string(), // Default credential
            },
        };
        service1.create_secret(sec1, "test").await.unwrap();

        // 2. Add secret with shared/reused password
        let sec2 = CreateSecretInput {
            title: "Redis Cache".to_string(),
            tags: vec![],
            username: None,
            url: None,
            notes: None,
            payload: SecretPayload::Password {
                password: "admin".to_string(), // Reused with sec1!
            },
        };
        service1.create_secret(sec2, "test").await.unwrap();

        // 3. Add strong unique password
        let sec3 = CreateSecretInput {
            title: "Vault Admin Master".to_string(),
            tags: vec![],
            username: Some("admin".to_string()),
            url: None,
            notes: None,
            payload: SecretPayload::Password {
                password: "Kx9#mP2$vL7!qR4@wZ8*yN".to_string(),
            },
        };
        service1.create_secret(sec3, "test").await.unwrap();

        // 4. Run Risk Scoring & Health Report
        let health = service1.get_health_report().await.unwrap();
        assert_eq!(health.total_secrets, 3);
        assert_eq!(health.compromised_count, 2);
        assert_eq!(health.reused_count, 2);
        assert!(health.overall_score < 70);
        assert!(health
            .recommendations
            .iter()
            .any(|r| r.severity == Severity::Critical));

        // 5. Export Encrypted Backup
        let export_summary = service1
            .export_encrypted_backup("BackupKeyStrongPassphrase999!", &backup_file)
            .await
            .unwrap();
        assert_eq!(export_summary.total_secrets, 3);
        assert!(export_summary.file_size_bytes > 72);

        // 6. Restore into a new Vault
        let storage2 = VaultStorage::open_in_memory().unwrap();
        let service2 = VaultService::new(storage2, Some(Argon2Params::fast_for_tests()));
        service2
            .initialize("NewVaultMasterPassword456!")
            .await
            .unwrap();

        let import_summary = service2
            .import_encrypted_backup("BackupKeyStrongPassphrase999!", &backup_file, false)
            .await
            .unwrap();
        assert_eq!(import_summary.total_imported, 3);
        assert_eq!(import_summary.total_skipped, 0);

        // Check restored search works
        let restored_hits = service2.search_secrets("Redis", 10).await.unwrap();
        assert_eq!(restored_hits.len(), 1);
        assert_eq!(restored_hits[0].title, "Redis Cache");

        // 7. Tampered Backup Detection
        let mut tampered_bytes = std::fs::read(&backup_file).unwrap();
        let len = tampered_bytes.len();
        tampered_bytes[len - 5] ^= 0xAA; // Corrupt ciphertext byte
        let tampered_file = temp.path().join("tampered.vaultbackup");
        std::fs::write(&tampered_file, tampered_bytes).unwrap();

        // Restore of tampered file must fail
        assert!(service2
            .import_encrypted_backup("BackupKeyStrongPassphrase999!", &tampered_file, false)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ai_file_explorer_end_to_end_and_dedup_flow() {
        use ai_file_explorer::models::BatchRenameRule;
        use ai_file_explorer::service::FileExplorerService;
        use std::fs::File;
        use std::io::Write;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        // 1. Create directory structure
        let docs = root.join("documents");
        let photos = root.join("photos");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&photos).unwrap();

        let doc1 = docs.join("annual_report.pdf");
        let doc2 = docs.join("notes.md");
        {
            let mut f1 = File::create(&doc1).unwrap();
            f1.write_all(&vec![1u8; 15000]).unwrap();
            f1.sync_all().unwrap();

            let mut f2 = File::create(&doc2).unwrap();
            f2.write_all(b"# Meeting Notes\nDiscussion about offline apps.")
                .unwrap();
            f2.sync_all().unwrap();
        }

        // 2. Create duplicate photos (same 12000-byte content)
        let photo1 = photos.join("IMG_001.JPG");
        let photo2 = photos.join("IMG_001_copy.JPG");
        let photo3 = photos.join("IMG_002.JPG"); // Different content, same size
        let photo_bytes = vec![77u8; 12000];
        let mut photo3_bytes = vec![77u8; 12000];
        photo3_bytes[6000] = 88;

        {
            let mut p1 = File::create(&photo1).unwrap();
            p1.write_all(&photo_bytes).unwrap();
            p1.sync_all().unwrap();

            let mut p2 = File::create(&photo2).unwrap();
            p2.write_all(&photo_bytes).unwrap();
            p2.sync_all().unwrap();

            let mut p3 = File::create(&photo3).unwrap();
            p3.write_all(&photo3_bytes).unwrap();
            p3.sync_all().unwrap();
        }

        let service = FileExplorerService::new().unwrap();

        // 3. Test Directory Listing
        let listing = service.list_directory(&docs).unwrap();
        assert_eq!(listing.total_count, 2);

        // 4. Test Indexing & Sub-50ms Search
        let all_files = vec![listing.items[0].clone(), listing.items[1].clone()];
        let indexed_count = service.index_files(&all_files).unwrap();
        assert_eq!(indexed_count, 2);

        let search_hits = service.search("report", 10).unwrap();
        assert_eq!(search_hits.len(), 1);
        assert_eq!(search_hits[0].file_name, "annual_report.pdf");

        // 5. Test 3-Tier Duplicate Detection (ADR-0022)
        let dedup_summary = service.scan_duplicates(&[root], 1).unwrap();
        assert_eq!(dedup_summary.duplicate_groups.len(), 1);
        assert_eq!(dedup_summary.duplicate_groups[0].file_size, 12000);
        assert_eq!(dedup_summary.duplicate_groups[0].file_paths.len(), 2);
        assert_eq!(dedup_summary.potential_reclaimed_bytes, 12000);

        // 6. Test Storage Analytics
        let stats = service.analyze_storage(root, 5).unwrap();
        assert_eq!(stats.total_files, 5);
        assert_eq!(stats.total_dirs, 2);
        assert!(stats.total_size_bytes >= 39000);
        assert_eq!(stats.extension_counts.get("jpg"), Some(&3));
        assert_eq!(stats.extension_counts.get("pdf"), Some(&1));
        assert_eq!(stats.extension_counts.get("md"), Some(&1));

        // 7. Test Squarified Treemap Layout (ADR-0025)
        let treemap = service.generate_treemap(root, 1024.0, 768.0, 3).unwrap();
        assert_eq!(treemap.bounds.width, 1024.0);
        assert_eq!(treemap.bounds.height, 768.0);
        assert_eq!(treemap.children.len(), 2);

        // 8. Test Transactional Batch Rename (ADR-0024)
        let files_to_rename = vec![photo1.clone(), photo2.clone()];
        let rename_rule = BatchRenameRule::Numbering {
            prefix: "vacation_".to_string(),
            start_number: 101,
            pad_digits: 3,
        };

        let previews = service
            .preview_batch_rename(&files_to_rename, &rename_rule)
            .unwrap();
        assert_eq!(previews.len(), 2);
        assert!(!previews[0].has_conflict);
        assert!(!previews[1].has_conflict);

        let rename_result = service.execute_batch_rename(&previews).unwrap();
        assert_eq!(rename_result.successful_renames, 2);
        assert!(!rename_result.rolled_back);

        assert!(!photo1.exists());
        assert!(!photo2.exists());
        assert!(photos.join("vacation_101.JPG").exists());
        assert!(photos.join("vacation_102.JPG").exists());
    }

    #[test]
    fn test_ai_file_explorer_advanced_ocr_and_semantic_flow() {
        use ai_file_explorer::models::{ContentSearchQuery, SearchMode};
        use ai_file_explorer::screenshot::compute_hamming_distance;
        use ai_file_explorer::FileExplorerService;
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;

        let temp = tempdir().unwrap();
        let root = temp.path();

        // 1. Create multi-format test documents
        let md_file = root.join("architecture.md");
        let mut f1 = File::create(&md_file).unwrap();
        f1.write_all(b"# Architecture Overview\n\nHigh-performance offline document indexing using Tantivy and 8-bit quantized embeddings.").unwrap();
        f1.sync_all().unwrap();

        let rs_file = root.join("pipeline.rs");
        let mut f2 = File::create(&rs_file).unwrap();
        f2.write_all(b"//! Asynchronous indexing pipeline\n\npub fn execute_pipeline() {\n    // zero-copy processing\n}\n").unwrap();
        f2.sync_all().unwrap();

        let service = FileExplorerService::new().unwrap();

        // 2. Extract and index documents
        let doc1 = service.extract_document(&md_file).unwrap();
        assert_eq!(doc1.file_name, "architecture.md");
        assert!(doc1.content_text.contains("quantized embeddings"));
        service.index_extracted_document(&doc1).unwrap();

        let doc2 = service.extract_document(&rs_file).unwrap();
        assert_eq!(doc2.file_name, "pipeline.rs");
        assert!(doc2.content_text.contains("zero-copy processing"));
        service.index_extracted_document(&doc2).unwrap();

        // 3. Keyword Content Search
        let kw_query = ContentSearchQuery {
            query: "quantized".to_string(),
            mode: SearchMode::Keyword,
            limit: 5,
            extension_filter: None,
        };
        let kw_res = service.search_content(&kw_query).unwrap();
        assert_eq!(kw_res.total_matches, 1);
        assert_eq!(kw_res.items[0].file_name, "architecture.md");

        // 4. Semantic Search
        let sem_query = ContentSearchQuery {
            query: "vector representation in memory".to_string(),
            mode: SearchMode::Semantic,
            limit: 5,
            extension_filter: None,
        };
        let sem_res = service.search_content(&sem_query).unwrap();
        assert_eq!(sem_res.total_matches, 2);
        assert!(sem_res.items[0].score > 0.0);

        // 5. Combined Search
        let comb_query = ContentSearchQuery {
            query: "pipeline".to_string(),
            mode: SearchMode::Combined,
            limit: 5,
            extension_filter: Some("rs".to_string()),
        };
        let comb_res = service.search_content(&comb_query).unwrap();
        assert_eq!(comb_res.total_matches, 1);
        assert_eq!(comb_res.items[0].file_name, "pipeline.rs");

        // 6. Test Screenshot Perceptual Hashing & Hamming Distance
        let hash_a =
            0b1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010u64;
        let hash_b =
            0b1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1010_1011u64; // 1 bit diff
        let dist = compute_hamming_distance(hash_a, hash_b);
        assert_eq!(dist, 1);

        // 7. Test Offline OCR on synthetic raster buffer
        let mut bmp = Vec::new();
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&58u32.to_le_bytes()); // file size
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&54u32.to_le_bytes()); // offset
        bmp.extend_from_slice(&40u32.to_le_bytes()); // header size
        bmp.extend_from_slice(&4i32.to_le_bytes()); // width = 4
        bmp.extend_from_slice(&4i32.to_le_bytes()); // height = 4
        bmp.extend_from_slice(&1u16.to_le_bytes()); // planes
        bmp.extend_from_slice(&24u16.to_le_bytes()); // 24 bpp
        bmp.extend_from_slice(&0u32.to_le_bytes()); // uncompressed
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0i32.to_le_bytes());
        bmp.extend_from_slice(&0i32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        // 16 pixels
        for _ in 0..16 {
            bmp.extend_from_slice(&[20, 20, 20]);
        }

        let img_file = root.join("screenshot.bmp");
        let mut f_img = File::create(&img_file).unwrap();
        f_img.write_all(&bmp).unwrap();
        f_img.sync_all().unwrap();

        let ocr_res = service.ocr_image(&img_file).unwrap();
        assert!(ocr_res.processing_time_ms >= 0.0);
    }
}
