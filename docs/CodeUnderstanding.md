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

### 4.2 Log Viewer Large File Inspection Flow
```text
User opens 100GB log file
  --> Universal Log Viewer requests mmap handle via shared-storage/mmap engine
  --> Zero-copy memory map initialized via OS virtual memory
  --> shared-search / SIMD line scanner builds sparse index of newline offsets (100GB < 2s)
  --> Virtualized Svelte UI requests slice: lines 10,000 to 10,050
  --> Rust core resolves line byte offsets from sparse index in O(1) time
  --> Raw byte slice converted to UTF-8 lossy and returned via IPC in < 1ms
```

### 4.3 AI File Explorer Duplicate Detection Flow
```text
Crawl directory
  --> shared-indexing streams file metadata to dedupe engine
  --> Group 1: Group files by exact byte size
  --> Group 2: For candidates with identical size, hash 4KB header and footer
  --> Group 3: For candidates with matching partial hashes, stream full BLAKE3 cryptographic hash
  --> Duplicate groups displayed in UI; zero redundant full-file reads performed
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
