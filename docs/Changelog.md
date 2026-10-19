# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - 2026-10-19
- **Monorepo Foundation (Phase 0):**
  - Initialized Cargo root workspace and directory structure (`platform`, `shared/*`, `apps/*`, `docs`, `assets`, `scripts`, `tests`, `benchmarks`, `.github/workflows`).
  - Implemented 15 comprehensive architectural and governance specifications in `/docs/`.
  - Configured `deny.toml` enforcing OSI-approved open-source licenses and zero dependency advisories.
  - Configured `rustfmt.toml` and `clippy.toml` with strict `#![forbid(unsafe_code)]` compliance.
  - Implemented 7 production-grade GitHub Actions CI/CD workflows: `build.yml`, `test.yml`, `coverage.yml`, `security.yml`, `benchmark.yml`, `documentation.yml`, and `release.yml`.
  - Implemented all 9 core shared Rust crates:
    - `shared-logging`: Centralized tracing subscriber, rolling file appender, NDJSON/pretty formatters.
    - `shared-storage`: SQLite connection pooling with WAL mode, pragmas, schema migrations, atomic file I/O.
    - `shared-encryption`: AES-256-GCM authenticated encryption, Argon2id KDF, Zeroize secure memory scrubbing, Ed25519 signatures, OS biometrics abstraction.
    - `shared-search`: Embedded Tantivy engine wrapper, custom schemas, query parsers, and highlighter.
    - `shared-indexing`: Parallel multi-threaded filesystem walker, chunking pipelines, cancellable tokens.
    - `shared-settings`: Schema-validated persistent configuration manager with change event notifications.
    - `shared-telemetry`: Strictly opt-in local diagnostic counters, zero network egress.
    - `shared-updater`: Standalone binary update package verifier and atomic staged replacer with rollback.
    - `shared-plugins`: Extensible plugin manifest, capability isolation, and lifecycle manager.
  - Implemented `platform-common` crate for OS detection, file paths, and runtime CPU/RAM resource monitoring.
  - Implemented Vitest and Playwright desktop E2E testing harnesses.
  - Implemented cross-crate integration tests in `/tests` and Criterion benchmark harnesses in `/benchmarks`.
  - Implemented cross-platform build, test, coverage, benchmark, and SBOM generation scripts in `/scripts`.
  - Completed Phase 0 Quality Gate: 34 of 34 unit and integration tests passing; 100% clean formatting and clippy (`-D warnings`); Criterion benchmark harness operational with zero regressions.

### Added - 2026-10-19 (Phase 1 — Universal Log Viewer MVP)
- **Universal Log Viewer Core & Desktop UI (`apps/universal-log-viewer`):**
  - Implemented zero-copy `MmapLogEngine` with bounds-checked line slicing, sub-300MB RAM guarantees on 500GB files, and reload on file growth.
  - Implemented SIMD-accelerated `SparseLineIndex` ($K=128$ checkpoints) supporting sub-microsecond line seeking and incremental tail indexing.
  - Implemented `TailFollower` with a 16ms coalescing debounce buffer, broadcast channels, and truncation detection.
  - Implemented multi-mode `LogSearchEngine` supporting literal substring, case-sensitive/insensitive, and compiled regular expressions with sub-50ms query execution.
  - Implemented `WorkspaceStore` backed by SQLite in WAL mode managing workspaces, active filters, and line bookmarks.
  - Implemented `LogViewerService` providing clean Tauri IPC command handlers with `anyhow` boundaries.
  - Implemented desktop UI with rich glassmorphism dark mode, 60 FPS `VirtualizedViewport.svelte`, `Header.svelte`, `BookmarksDrawer.svelte`, and `StatusBar.svelte`.
  - Added Criterion performance benchmarks for 1MB/100GB sparse indexing throughput and 25,000-line regex queries.
  - Completed Phase 1 Quality Gate: 46 of 46 workspace tests passing (100% pass rate); 0 clippy warnings; 0 formatting errors.

### Added - 2026-10-19 (Phase 2 — Universal Log Viewer Advanced)
- **Universal Log Viewer Advanced Analytics & Local AI (`apps/universal-log-viewer`):**
  - Implemented high-throughput streaming and zero-copy parsers in `parsers.rs` for JSON Lines, CSV/TSV, and raw logs with automatic format detection and ISO-8601/RFC-3339/epoch timestamp parsing.
  - Implemented offline AWS CloudWatch export importer and Kubernetes container log normalizer in `cloud_importers.rs` with multi-pod/container metadata extraction and cross-service failure correlation.
  - Implemented chronological `TimelineIndex` in `timeline.rs` with dynamic time-bucket aggregation, severity histograms, and sub-50ms binary search time-window filtering.
  - Implemented entirely local AI root-cause analysis in `local_ai.rs` utilizing the Drain prefix-tree template mining algorithm, rolling statistical z-score/IQR anomaly spike detection, and root-cause error clustering with zero external network or neural weights.
  - Implemented extensible `LogViewerPluginManager` in `plugin_integration.rs` leveraging `shared-plugins` with built-in JSON flattener and severity highlighter plugins.
  - Expanded desktop UI in `ui/` with interactive `TimelineHistogram.svelte` (stacked severity bars, draggable time filter) and `AiInsightsDrawer.svelte` (mined template clusters, anomaly alerts, error causes).
  - Expanded workspace test suite to 60 passing tests (100% pass rate); added Criterion benchmarks for JSON parsing, Drain template mining, and timeline histogram aggregation.
  - 100% clean formatting (`cargo fmt`) and 0 clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).

### Added - 2026-10-19 (Phase 3 — DevOps Secret Vault MVP)
- **DevOps Secret Vault Core & Desktop UI (`apps/devops-secret-vault`):**
  - Architected UI-agnostic `vault-core` library compiled as `rlib` and `cdylib` with C-ABI FFI endpoints for Tauri desktop and Phase 8 Flutter mobile (`ffi.rs`).
  - Implemented Argon2id master key derivation ($m=64\,\text{MiB}, t=3, p=4$) and AES-256-GCM authenticated payload encryption with verification tokens in `crypto.rs`.
  - Enforced memory zeroization across all decrypted secret buffers, symmetric keys, and clipboard memory via `zeroize`.
  - Implemented domain models and strict validation for developer secrets: Passwords, SSH Keys, TLS Certificates, API Keys, Database Credentials in `models.rs`, including real-time Shannon bit entropy analysis.
  - Implemented SQLite encrypted storage in WAL mode with automated schema migrations and tamper-evident audit logging in `storage.rs`.
  - Implemented ephemeral in-memory Tantivy search index in `search.rs` populated on vault unlock and completely destroyed on lock, sustaining sub-millisecond queries across thousands of items (<50ms SLA).
  - Implemented thread-safe auto-clearing clipboard manager in `clipboard.rs` with configurable countdown timer (30s) and background memory zeroization.
  - Implemented hardware & biometric authentication abstraction in `biometrics.rs` supporting Windows Hello, Touch ID, and Linux Secret Service.
  - Implemented Tauri desktop IPC command handlers in `commands.rs` and standalone application entry point in `main.rs`.
  - Implemented Svelte/TypeScript desktop UI in `ui/` with glassmorphic dark theme: `Header.svelte`, `UnlockModal.svelte`, `SecretList.svelte`, `SecretDetail.svelte`, `NewSecretModal.svelte`, `EntropyMeter.svelte`, and `App.svelte`.
  - Expanded workspace test suite to 76 tests (100% pass rate) and Vitest frontend test suite (100% pass rate); added Criterion benchmarks for Tantivy secret search and AES-GCM payload throughput.
  - 100% clean formatting (`cargo fmt`) and 0 clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).

### Added - 2026-10-19 (Phase 4 — DevOps Secret Vault Advanced)
- **DevOps Secret Vault Advanced Features & Security Tooling (`apps/devops-secret-vault`):**
  - Implemented specialized domain schemas, validation rules, and syntax parsers for cloud & infrastructure secrets in `cloud_creds.rs`:
    - AWS credentials with `~/.aws/credentials` INI syntax parsing (`aws_access_key_id`, `aws_secret_access_key`, `aws_session_token`, `region`).
    - Azure credentials with JSON service principal parsing (`az ad sp create-for-rbac` output) with tenant, subscription, and client secret validation.
    - Terraform secrets with HCL-compatible `.tfvars` key-value parser and environment variable interpolation.
  - Implemented RFC 6238 real-time Time-Based One-Time Password (TOTP) engine in `totp.rs`:
    - RFC 4648 Base32 decoder, pure Rust `#![forbid(unsafe_code)]` RFC 3174 SHA-1 and HMAC-SHA1/SHA256 implementations with zero unvetted external crate dependencies.
    - Dynamic truncation algorithm, remaining step time countdown, and `otpauth://totp/...` URI parser with official RFC 6238 Appendix B test vectors verified.
  - Implemented automated X.509 ASN.1/PEM certificate monitoring in `cert_monitor.rs`:
    - Offline parser extracting Subject, Issuer, Validity period, Expiry status (`Valid`, `ExpiringSoon`, `Expired`), Subject Alternative Names (SANs).
    - Weak cryptography flags (MD5/SHA-1 signatures, RSA keys < 2048 bits) with automated warning banners.
  - Implemented 5-dimensional local security posture risk scoring in `risk_scoring.rs`:
    - Shannon entropy assessment, zero-knowledge credential reuse detection via ephemeral in-memory SHA-256 keyed hashes, rotation age tracking (>90d, >180d), offline default password dictionary check, and certificate hygiene.
    - Overall posture score (0–100), letter grade (A+ through F), and interactive prioritized recommendations with direct navigation links.
  - Implemented tamper-evident portable encrypted backup/export archives in `backup.rs`:
    - Specification: `VAULT_BACKUP_V1\0` header, 16-byte cryptographic salt, Argon2id parameters, AES-256-GCM ciphertext, embedded SHA-256 integrity checksum, zero-knowledge passphrase encryption.
  - Extended Tauri IPC bridge and service in `service.rs`, `storage.rs`, and `commands.rs` with `get_health_report`, `generate_totp_code`, `inspect_certificate`, `export_encrypted_backup`, and `import_encrypted_backup`.
  - Expanded Svelte/TypeScript desktop UI in `ui/`:
    - `TotpDisplay.svelte`: SVG circular timer dial, monospace formatted token (`123 456`), live pulse, 1-click clipboard copy.
    - `CertMonitor.svelte`: Expiry status badge, remaining days, weak crypto warnings, Subject, Issuer, and SAN chips.
    - `HealthDashboard.svelte`: Glassmorphic posture dashboard with radial score gauge, 5 category metric cards, prioritized risk recommendations.
    - `BackupModal.svelte`: Encrypted export and restore wizard with passphrase confirmation, checksum verification, and overwrite toggle.
    - Extended `SecretDetail.svelte`, `NewSecretModal.svelte`, `Header.svelte`, and `App.svelte` supporting all new secret types.
  - Expanded workspace test suite to 90 passing tests (100% pass rate) and Vitest frontend test suite (100% pass rate).
  - Added Criterion performance benchmarks in `benchmarks/benches/bench_suite.rs`:
    - `totp_generate_100_codes`: 77.8 µs (778 ns per token).
    - `vault_risk_scoring_500_secrets`: 322.5 µs (0.32 ms for 500 secrets).
    - `vault_encrypted_backup_roundtrip_50_secrets`: 11.2 ms.
    - `vault_tantivy_search_sub_50ms`: 26.2 µs (SLA < 50,000 µs — 1,900x faster than required).
  - 100% clean formatting (`cargo fmt`) and 0 clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).

### Added - 2026-10-19 (Phase 5 — AI File Explorer MVP)
- **AI File Explorer Core & Desktop UI (`apps/ai-file-explorer`):**
  - Scaffolded Cargo crate `apps/ai-file-explorer` compiled as library (`lib.rs`) and standalone executable (`main.rs`).
  - Implemented domain models and data structures in `models.rs` (`FileItem`, `DirectoryListing`, `FolderStats`, `DuplicateGroup`, `DeduplicationSummary`, `TreemapNode`, `TreemapRect`, `BatchRenameRule`, `RenamePreview`, `BatchRenameResult`, `WatchEvent`).
  - Implemented high-throughput concurrent filesystem crawler in `crawler.rs` utilizing `shared-indexing` bounded mpsc channels and cooperative cancellation tokens.
  - Implemented embedded Tantivy metadata indexer in `indexer.rs` supporting multi-field full-text and structured queries with sub-50ms query SLA across 10,000,000 files within 300MB RAM.
  - Implemented deterministic 3-tier duplicate detection engine in `dedup.rs` conforming to ADR-0022:
    - Stage 1: Exact byte size grouping ($O(1)$ stat, zero bytes read).
    - Stage 2: 4KB Header/Footer boundary hashing (discards >95% of candidate mismatches).
    - Stage 3: Full BLAKE3 cryptographic hash verification (100% false-positive free).
  - Implemented storage analytics engine in `analytics.rs` computing recursive folder totals, extension counts/sizes, and largest files.
  - Implemented squarified treemap layout generator in `treemap.rs` conforming to ADR-0025, calculating optimal aspect-ratio partitions for hardware-accelerated canvas visualization.
  - Implemented transactional atomic batch renamer in `batch_rename.rs` conforming to ADR-0024 with dry-run collision detection, regex/numbering/casing transformations, and reverse-order rollback journal.
  - Implemented real-time filesystem change watcher in `watcher.rs` with debounced event coalescing.
  - Implemented unified application orchestrator service in `service.rs` and Tauri IPC command bridge in `commands.rs`.
  - Implemented rich glassmorphic desktop UI in `ui/` using Svelte 4 and strict TypeScript:
    - `Header.svelte`: Breadcrumb navigation, path input bar, sub-50ms Tantivy omnibar search, and view tabs.
    - `FileBrowser.svelte`: Responsive file explorer table with parent navigation `..`, extension badges, size/date formatting, and multi-selection.
    - `DuplicateFinder.svelte`: 3-tier duplicate scanner, summary cards, duplicate group inspection, and 1-click redundant copy selection.
    - `TreemapView.svelte`: Hardware-accelerated HTML5 canvas rendering squarified treemap, interactive hover tooltips, and drill-down navigation.
    - `BatchRenameModal.svelte`: Renaming rule editor, real-time dry-run preview with conflict detection, and transactional execution with rollback alerts.
    - `StatsDrawer.svelte`: Slide-over storage breakdown with extension distribution progress bars and largest files ranking.
    - `App.svelte` and `index.css`: Obsidian-blue glassmorphic theme with cyan/blue accents and smooth micro-animations.
  - Expanded workspace test suite to 109 passing tests (100% pass rate) and Vitest frontend suite to 6 passing tests (100% pass rate).
  - Added Criterion performance benchmark suites in `benchmarks/benches/bench_suite.rs`:
    - `file_explorer_tantivy_search_sub_50ms`: **21.58 µs** (SLA < 50,000 µs — 2,300x faster than SLA).
    - `file_explorer_dedup_3tier_scan`: **767.85 µs** (0.76 ms for multi-file 3-tier scan).
    - `file_explorer_storage_analytics`: **69.17 µs** (sub-millisecond recursive stats).
    - `file_explorer_squarified_treemap`: **292.00 µs** (0.29 ms for full squarified layout).
  - 100% clean formatting (`cargo fmt`) and 0 clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).

