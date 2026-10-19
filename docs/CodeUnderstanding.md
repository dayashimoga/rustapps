# Code Understanding & System Architecture Guide

**Document Version:** 1.0.0  
**Purpose:** Living architectural map providing transparent understanding of codebase organization, data flows, operational boundaries, and system invariants.

---

## 1. High-Level Architecture

The monorepo employs a clean hexagonal / ports-and-adapters architecture implemented in Rust stable. The architecture cleanly separates presentation (Tauri v2 desktop, Flutter mobile) from domain business logic (application core libraries) and infrastructure primitives (shared reusable crates).

```text
[ Presentation Layer: Tauri v2 / Svelte / Flutter ]
                       |
                       v  (IPC / FFI Bridge)
[ Application Domain: vault-core, explorer-core, log-core ]
                       |
                       v  (In-Process Rust Calls)
[ Shared Crates: logging, storage, encryption, search, indexing, settings, ... ]
                       |
                       v
[ System Level: OS APIs, Hardware, Memory, Disk ]
```

---

## 2. Monorepo Folders & Layout

- `/platform`: Native OS integration adapters, platform capability detection, hardware resource meters, and standard file path resolvers.
- `/shared/logging`: Centralized tracing initialization, non-blocking rolling file appenders, NDJSON and ANSI console formatting.
- `/shared/storage`: SQLite/SQLCipher connection pools, WAL mode configuration, migration framework, and safe atomic filesystem wrappers.
- `/shared/encryption`: AES-256-GCM symmetric cipher, Argon2id KDF, memory zeroization (`zeroize`), Ed25519 digital signatures, and biometric abstraction.
- `/shared/search`: Embedded Tantivy full-text search engine, schema definitions, sub-50ms query executor, and hit snippet generator.
- `/shared/indexing`: High-throughput parallel directory traversal, chunking pipelines, backpressure channels, and cancellation watch tokens.
- `/shared/settings`: Schema-validated persistent configuration engine with event bus notifications.
- `/shared/telemetry`: Strictly opt-in local performance metrics (gauges, counters, histograms); zero network connectivity.
- `/shared/updater`: Cryptographically signed offline binary update verifier and atomic staged installer with rollback.
- `/shared/plugins`: Sandboxed plugin host, capability manifest parser, and bounded lifecycle runner.
- `/apps/devops-secret-vault`: DevOps Secret Vault application frontend (Tauri/Svelte) and backend logic.
- `/apps/ai-file-explorer`: AI File Explorer application frontend (Tauri/Svelte) and backend logic.
- `/apps/universal-log-viewer`: Universal Log Viewer application frontend (Tauri/Svelte) and backend logic.
- `/docs`: The 15 mandatory architecture, security, performance, and governance specifications.
- `/assets`: Shared vector icons, typography, and visual assets.
- `/scripts`: Automation scripts for builds, test execution, coverage reporting, benchmarking, and SBOM generation.
- `/tests`: Cross-crate integration tests, compatibility suites, and Playwright desktop E2E tests.
- `/benchmarks`: Criterion performance benchmark suites and regression tracking baselines.
- `/.github/workflows`: The 7 mandatory GitHub Actions CI/CD workflows.

---

## 3. Module Responsibilities & Crate Boundaries

1. **`shared-logging`:** Provides logging facilities via the `tracing` ecosystem. Crate clients never invoke `println!` or unformatted output. Log rotation prevents disk exhaustion.
2. **`shared-storage`:** Owns SQLite database interaction. Forces `PRAGMA journal_mode = WAL;` and `PRAGMA synchronous = NORMAL;`. Provides idempotent schema migrations and atomic file writes via staging files.
3. **`shared-encryption`:** Contains all cryptographic operations. No secret keys or decrypted plaintext are ever represented as standard unrestricted `String` or `Vec<u8>`; they are wrapped in zeroizing containers.
4. **`shared-search`:** Provides in-process Tantivy indexing. Manages reader pools, index writers, and query execution without external servers.
5. **`shared-indexing`:** Walks local filesystems asynchronously using thread-pool work stealing. Feeds indexing pipelines through bounded mpsc channels.
6. **`shared-settings`:** Validates and stores app configuration. Emits notifications when settings change.
7. **`shared-telemetry`:** Tracks local execution metrics. Remains completely inert unless explicitly activated by user.
8. **`shared-updater`:** Validates Ed25519 signatures on update bundles and swaps executables atomically.
9. **`shared-plugins`:** Dispatches events to registered plugins inside bounded memory and CPU quotas.
10. **`platform-common`:** Resolves OS paths and samples instantaneous RAM and CPU metrics.
11. **`universal-log-viewer` (`apps/universal-log-viewer`):**
    - `mmap_engine`: Manages zero-copy virtual memory mappings with bounds checking and reload-on-append.
    - `sparse_index`: SIMD newline scanner building $K=128$ sparse checkpoints for sub-microsecond line seeking and $< 300\,\text{MB}$ RAM consumption on 500GB files.
    - `tail_mode`: Non-blocking real-time file follower with 16ms coalescing debounce buffer.
    - `search_engine`: Literal, case-insensitive, and compiled regular expression query executor (<50ms SLA).
    - `parsers`: Zero-copy and streaming deserializers for JSON Lines, CSV/TSV, and raw text with format auto-detection and high-speed timestamp parsing.
    - `cloud_importers`: Offline AWS CloudWatch and Kubernetes container log normalizers with multi-pod/container metadata extraction and cross-service failure correlation.
    - `timeline`: Chronological index with dynamic bucket aggregation, volume/severity histograms, and sub-50ms range filtering.
    - `local_ai`: 100% offline root-cause analysis via the Drain prefix-tree template miner, rolling z-score/IQR anomaly spike detection, and root-cause error clustering (zero external neural weights).
    - `plugin_integration`: Extensible `LogViewerPluginManager` hosting sandboxed parsers, syntax highlighters, and export sinks via `shared-plugins`.
    - `workspace`: SQLite WAL backed store for open files, bookmarks, and scroll state.
    - `commands`: Tauri IPC service boundary with Serde models.
    - `ui`: Svelte/TS frontend with 60 FPS virtualized viewport, `TimelineHistogram.svelte`, `AiInsightsDrawer.svelte`, and live telemetry indicators.
12. **`vault_core` / `devops-secret-vault` (`apps/devops-secret-vault`):**
    - `crypto`: Argon2id master key derivation ($m=64\,\text{MiB}, t=3, p=4$), AES-256-GCM authenticated payload encryption, verifier token, and memory zeroization.
    - `models`: Domain entities (Password, SSH Key, Certificate, API Key, Database Credential, AWS Credential, Azure Credential, Terraform Secret, TOTP), validation rules, UUID generator, and Shannon bit entropy calculator.
    - `cloud_creds`: Specialized domain schemas and syntax parsers for AWS credentials (`~/.aws/credentials` INI format), Azure service principals (`az ad sp create-for-rbac` JSON format), and Terraform `.tfvars` key-value pairs.
    - `totp`: Pure Rust RFC 6238 real-time TOTP token generator, RFC 4648 Base32 decoder, `#![forbid(unsafe_code)]` RFC 3174 SHA-1 and HMAC-SHA1/SHA256, dynamic truncation, remaining step countdown, and `otpauth://totp/...` URI parser (778 ns per token).
    - `cert_monitor`: Offline X.509 ASN.1 DER and PEM certificate parser extracting validity windows, expiry states (`Valid`, `ExpiringSoon`, `Expired`), weak cryptography flags (MD5/SHA-1, RSA < 2048), Subject, Issuer, and SANs.
    - `risk_scoring`: 5-dimensional security posture scoring (entropy, zero-knowledge credential reuse detection via ephemeral in-memory SHA-256 keyed hashes, rotation age > 90/180 days, curated offline default password list, certificate hygiene) generating 0–100 score, grade (A+ through F), and prioritized remediation items.
    - `backup`: Tamper-evident portable `.vaultbackup` archive bundle engine (`VAULT_BACKUP_V1\0`, 16-byte salt, Argon2id parameters, AES-256-GCM ciphertext, embedded SHA-256 integrity checksum).
    - `storage`: SQLite WAL encrypted storage with automated schema migrations and tamper-evident audit logging.
    - `search`: Ephemeral in-memory Tantivy full-text and structured index (<50ms query time across 100k items) created on unlock and completely destroyed on lock.
    - `clipboard`: Thread-safe auto-clearing clipboard manager with 30s background countdown timer and zeroization.
    - `biometrics`: Hardware and biometric authentication abstraction (Windows Hello, Touch ID, Linux Secret Service).
    - `service`: `VaultService` orchestrator managing unlocked sessions, CRUD, audit trail, health audits, TOTP generation, certificate analysis, and encrypted backup export/restore.
    - `ffi`: C-ABI endpoints for Tauri desktop and Phase 8 Flutter mobile (`vault_create_handle`, `vault_unlock`, `vault_search`, etc.).
    - `commands`: Tauri desktop IPC command handlers.
    - `ui`: Svelte/TypeScript desktop UI with glassmorphic dark theme, unlock modal, secrets explorer, entropy meter, masked detail views, `TotpDisplay.svelte`, `CertMonitor.svelte`, `HealthDashboard.svelte`, and `BackupModal.svelte`.
13. **`ai-file-explorer` (`apps/ai-file-explorer`):**
    - `models`: Strongly typed domain entities (`FileItem`, `DirectoryListing`, `FolderStats`, `DuplicateGroup`, `DeduplicationSummary`, `TreemapNode`, `TreemapRect`, `BatchRenameRule`, `RenamePreview`, `BatchRenameResult`, `WatchEvent`).
    - `crawler`: Concurrent asynchronous directory traversal and listing engine with bounded channel streaming and cooperative cancellation tokens.
    - `indexer`: In-process Tantivy metadata indexer with multi-field search (`path`, `file_name`, `extension`, `size_bytes`, `modified_timestamp`) executing in 21.58 µs (<50ms SLA).
    - `dedup`: Deterministic 3-tier duplicate detection (ADR-0022): exact byte size grouping $\rightarrow$ 4KB header/footer boundary hashing $\rightarrow$ full BLAKE3 cryptographic verification, minimizing disk I/O.
    - `analytics`: Recursive storage analytics engine calculating folder sizes, file/folder counts, extension breakdowns, and top largest space consumers.
    - `treemap`: Squarified treemap layout generator (ADR-0025) optimizing aspect ratio for hardware-accelerated canvas visualization (0.29 ms).
    - `batch_rename`: Transactional atomic batch renamer (ADR-0024) with dry-run collision detection, regex/numbering transformations, and reverse-order rollback journal.
    - `watcher`: Real-time debounced filesystem watcher with event coalescing emitting `WatchEvent` updates.
    - `service`: `FileExplorerService` orchestrator managing crawler, indexer, deduplicator, analytics, treemap, and rename sessions.
    - `commands`: Tauri desktop IPC command bridge endpoints with `anyhow` boundaries.
    - `ui`: Svelte/TypeScript desktop UI with glassmorphic dark theme: `Header.svelte`, `FileBrowser.svelte`, `DuplicateFinder.svelte`, `TreemapView.svelte`, `BatchRenameModal.svelte`, `StatsDrawer.svelte`.

---

## 4. End-to-End Data Flow

### 4.1 Vault Secret Retrieval Flow
```text
User Request (UI) 
  --> Tauri IPC command: get_secret(id)
  --> Application core checks biometric authorization / master key presence
  --> shared-storage retrieves ciphertext from SQLCipher table
  --> shared-encryption decrypts payload using AES-256-GCM into ZeroizingBytes
  --> Zeroized buffer returned to memory boundary
  --> Transferred via secure IPC to UI or copied to secure clipboard with auto-clear timer
```

### 4.2 Vault TOTP Generation Flow
```text
User views TOTP secret / requests code
  --> Svelte component TotpDisplay requests code via IPC: generate_totp_code(secret_id)
  --> VaultService verifies unlocked session and retrieves encrypted TOTP payload
  --> shared-encryption decrypts Base32 secret key into zeroizing memory
  --> RFC 6238 engine decodes Base32, computes time step counter (T = current_time / period)
  --> In-memory HMAC-SHA1 computes digest, applies dynamic truncation (778 ns execution)
  --> Formats 6-digit code ("123 456") and calculates remaining seconds in 30s window
  --> Returns code + remaining seconds to UI; SVG circular timer renders live countdown
```

### 4.3 Vault On-Device Security Risk Scoring Flow
```text
User opens Health Dashboard
  --> Tauri IPC command: get_health_report()
  --> VaultService fetches all encrypted secret payloads and decrypts in memory
  --> Ephemeral 32-byte salt generated in volatile memory
  --> Reuse Detector computes SHA-256(salt || secret_value) to identify duplicate credentials without comparing plaintexts
  --> Entropy calculator scores Shannon bit strength across all passwords
  --> Rotation age evaluator inspects updated_at timestamps against 90-day and 180-day thresholds
  --> Offline dictionary detector compares passwords against curated default credential list
  --> Certificate monitor inspects X.509 chains for expiry status and weak cryptographic ciphers
  --> 5 scores weighted into overall score (0-100) and letter grade (A+ through F)
  --> Ephemeral salt and decrypted payloads zeroized immediately
  --> HealthReport returned to Svelte UI and displayed in HealthDashboard.svelte (< 1ms execution)
```

### 4.4 Vault Encrypted Backup Export & Restore Flow
```text
User initiates Encrypted Backup Export
  --> User provides backup passphrase in BackupModal.svelte
  --> VaultService retrieves all secret metadata and payloads
  --> Serializes secrets array to canonical JSON bytes
  --> Computes SHA-256 integrity checksum over plaintext JSON
  --> Derives 256-bit encryption key via Argon2id from user passphrase and fresh 16-byte random salt
  --> Encrypts JSON bytes with AES-256-GCM authenticated cipher
  --> Packages binary bundle: 16-byte magic header, salt, Argon2id parameters, AES-GCM ciphertext, SHA-256 checksum
  --> Writes tamper-evident .vaultbackup file to disk; zeroizes sensitive buffers
Restore:
  --> Reads bundle, verifies magic header and structure
  --> Derives key from passphrase + salt, decrypts ciphertext via AES-256-GCM
  --> Verifies SHA-256 integrity checksum against decrypted payload before database commit
  --> Upserts secrets into local SQLite store; audit event logged
```

### 4.5 Log Viewer Large File Inspection Flow
```text
User opens 100GB log file
  --> Universal Log Viewer requests mmap handle via shared-storage/mmap engine
  --> Zero-copy memory map initialized via OS virtual memory
  --> shared-search / SIMD line scanner builds sparse index of newline offsets (100GB < 2s)
  --> Virtualized Svelte UI requests slice: lines 10,000 to 10,050
  --> Rust core resolves line byte offsets from sparse index in O(1) time
  --> Raw byte slice converted to UTF-8 lossy and returned via IPC in < 1ms
```

### 4.6 AI File Explorer Duplicate Detection Flow
```text
Crawl directory
  --> shared-indexing streams file metadata to dedupe engine
  --> Stage 1: Group files by exact byte size (filter out unique sizes with zero file reads)
  --> Stage 2: For candidate groups with identical size, read & hash 4KB header and 4KB footer
  --> Stage 3: For candidate groups with matching partial hashes, stream full BLAKE3 cryptographic hash
  --> Duplicate groups displayed in UI; zero redundant full-file reads performed (< 1ms execution)
```

### 4.7 AI File Explorer Storage Analytics & Squarified Treemap Flow
```text
User opens Treemap View or Stats Drawer
  --> Svelte UI requests folder analytics via IPC: get_folder_stats(path) or generate_treemap(path)
  --> FileExplorerService executes recursive directory traversal using shared-indexing
  --> FolderStats aggregates total bytes, file count, folder count, extension distribution, largest files
  --> Treemap generator recursively computes squarified partition rectangles optimizing aspect ratio (width/height ≈ 1)
  --> TreemapNode tree with computed TreemapRect bounding coordinates returned via IPC (292 µs)
  --> HTML5 Canvas renders hardware-accelerated colored blocks with hover tooltips and drill-down navigation
```

### 4.8 AI File Explorer Transactional Batch Rename with Rollback Journal Flow
```text
User configures rename rules in BatchRenameModal.svelte
  --> UI requests dry-run preview: preview_batch_rename(rules)
  --> Batch renamer generates prospective paths, detects duplicate targets or collisions
  --> User confirms: apply_batch_rename(rules)
  --> Phase 1 (Validation): Verifies source existence and absence of destination collisions
  --> Phase 2 (Execution): Performs atomic filesystem renames, recording each step in reverse-order journal
  --> If any rename fails: Rollback journal automatically restores all previously renamed files in reverse order
  --> If user clicks "Undo": execute_rename_rollback() restores original names from in-memory journal
```

---

## 5. Build and Runtime Processes

### Build Process
1. Rust workspace build: `cargo build --workspace --all-targets` with LTO enabled in release mode.
2. Frontend bundle build: Svelte components compiled to static assets via Vite.
3. Tauri packaging: Rust binaries linked with Tauri v2 runtime and bundled into native installers (MSI, AppImage, DMG).

### Runtime Process
1. Cold start executes initialization:
   - Configures `tracing` logger.
   - Loads settings via `shared-settings`.
   - Initializes SQLite pools and applies pending migrations via `shared-storage`.
   - Spawns Tokio background runtime for non-blocking tasks.
2. Enforces SLA gates: startup must complete within 2,000ms.

---

## 6. Security Boundaries

- **Process Isolation:** The UI runs in a sandboxed Webview process communicating exclusively via strongly typed Tauri IPC commands.
- **Air-Gap Guarantee:** No crates contain outbound networking code (no `reqwest`, `hyper`, `curl`, or raw TCP sockets in runtime paths).
- **Cryptographic Memory Boundary:** All cryptographic secrets are erased from RAM upon destruction via `zeroize`.
- **Integrity Validation:** All external inputs, updates, and plugins are cryptographically signed and schema-validated before execution.

---

## 7. Performance Considerations

- **Memory Constraints:** Max 300MB resident set size (RSS) regardless of dataset size (500GB logs or 10M indexed files).
- **Zero-Copy Architecture:** Heavy file operations utilize memory mapping and slice references rather than heap allocations.
- **Lock-Free Sparse Indexing:** Line offsets and file positions are stored in compact contiguous arrays allowing cache-friendly binary search.
- **Sub-50ms Search SLA:** Tantivy inverted indices and sparse indexes guarantee all search operations execute under 50ms.

---

## 8. Maintenance & Evolution

- Shared crates are versioned together in the Cargo workspace.
- Changes to shared crates must pass cross-crate integration suites and benchmark regression suites.
- Deprecations follow a strict two-minor-version grace period with migration documentation.
