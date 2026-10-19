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



