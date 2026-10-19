# Technical Architecture Specification (TAS)

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Core Architectural Blueprint  

---

## 1. System Overview & Architectural Topology

The monorepo contains three desktop/mobile applications sharing a unified, robust, and reusable Rust foundation. The architectural philosophy prioritizes strict encapsulation, memory safety, zero-copy operations, sub-millisecond local execution, and total offline autonomy.

```
+-----------------------------------------------------------------------------------+
|                                 USER INTERFACES                                   |
|   Desktop: Tauri v2 + Svelte (TypeScript)     |   Mobile: Flutter (Dart)          |
|   - Virtualized Viewports (60 FPS)           |   - Mobile Shell (Phase 8)        |
|   - Hardware Canvas / WebGL                   |   - Native Platform Channels      |
+-----------------------------------------------------------------------------------+
                                         |
                       [ Tauri IPC / C-ABI / Rust FFI ]
                                         v
+-----------------------------------------------------------------------------------+
|                              APPLICATION DOMAIN CORE                              |
|   apps/devops-secret-vault    apps/ai-file-explorer    apps/universal-log-viewer  |
|   - Vault State & Orchestr.   - Crawl & Dedupe Coord.  - Log Mmap & Tail Stream   |
|   - TOTP & Cert Monitors      - Semantic Query Engine  - Filter & Anomaly Engine  |
+-----------------------------------------------------------------------------------+
                                         |
                                         v
+-----------------------------------------------------------------------------------+
|                               SHARED CRATE ECOSYSTEM                              |
|  +--------------------+  +--------------------+  +--------------------+           |
|  |   shared/logging   |  |   shared/storage   |  | shared/encryption  |           |
|  +--------------------+  +--------------------+  +--------------------+           |
|  |   shared/search    |  |  shared/indexing   |  |   shared/settings  |           |
|  +--------------------+  +--------------------+  +--------------------+           |
|  |  shared/telemetry  |  |   shared/updater   |  |   shared/plugins   |           |
|  +--------------------+  +--------------------+  +--------------------+           |
|  |                           platform (OS Abstractions)                       |   |
|  +--------------------------------------------------------------------+-----------+
                                         |
                                         v
+-----------------------------------------------------------------------------------+
|                                INFRASTRUCTURE & OS                                |
|   Native Filesystem  |  SQLite / SQLCipher (WAL)  |  OS Biometrics / Hardware     |
+-----------------------------------------------------------------------------------+
```

---

## 2. Monorepo Structural Anatomy

```text
/
├── platform/                      # Native OS abstractions (paths, biometrics, resource meters)
├── shared/
│   ├── logging/                   # Structured logging, rolling appenders, tracing subscriber
│   ├── storage/                   # SQLite/SQLCipher connection pools, migrations, atomic file I/O
│   ├── indexing/                  # Parallel directory traversal, chunking pipelines, cancellable tokens
│   ├── encryption/                # AES-256-GCM, Argon2id, Zeroize memory buffers, Ed25519 signatures
│   ├── search/                    # Embedded Tantivy search engine, custom query parsers, highlighters
│   ├── updater/                   # Cryptographically signed standalone update verifier & staged replacer
│   ├── plugins/                   # Sandboxed plugin lifecycle, capability manifests, isolated sinks
│   ├── telemetry/                 # Local-only performance counters & metrics (opt-in; default OFF)
│   └── settings/                  # Persistent configuration manager with schema validation
├── apps/
│   ├── devops-secret-vault/       # Secret Vault application (Tauri + Svelte UI & Core)
│   ├── ai-file-explorer/          # AI File Explorer application (Tauri + Svelte UI & Core)
│   └── universal-log-viewer/      # Universal Log Viewer application (Tauri + Svelte UI & Core)
├── docs/                          # Standardized 15 architectural & governance documents
├── assets/                        # Shared vector icons, design tokens, typography, static assets
├── scripts/                       # PowerShell and Bash CI/CD, coverage, benchmark, SBOM scripts
├── tests/                         # End-to-end and cross-crate integration suites
├── benchmarks/                    # Comprehensive Criterion benchmark harnesses
└── .github/workflows/             # 7 strict GitHub Actions CI/CD workflows
```

---

## 3. Shared Subsystem Responsibilities

### 3.1 `shared/logging` (`shared-logging`)
- Configures centralized `tracing` subscriber with non-blocking worker threads.
- Implements rolling file appenders with automatic size-based rotation and compression.
- Provides standard formatting: human-readable pretty logs for development, compact and NDJSON for automated tooling.
- Ensures zero overhead when inactive; zero external network exporters.

### 3.2 `shared/storage` (`shared-storage`)
- Manages embedded SQLite connection pools utilizing Write-Ahead Logging (`PRAGMA journal_mode = WAL;`).
- Applies optimal performance pragmas (`PRAGMA synchronous = NORMAL;`, `PRAGMA temp_store = MEMORY;`, `PRAGMA cache_size = -64000;`).
- Built-in lightweight schema migration engine guaranteeing forward and backward data safety.
- Safe atomic filesystem primitives with flush-to-disk guarantees (`fsync` / `FlushFileBuffers`).

### 3.3 `shared/encryption` (`shared-encryption`)
- Authenticated symmetric encryption using **AES-256-GCM** with 96-bit unique nonces.
- Cryptographic key derivation via **Argon2id** (OWASP recommended parameters: memory 64MB, iterations 3, parallelism 4).
- Safe memory scrubbing: encapsulates all keys, passphrases, and sensitive buffers in `Zeroizing<T>` to guarantee zeroization on drop.
- **Ed25519** asymmetric digital signatures for verifying update payloads and encrypted backup authenticity.
- Unified OS biometrics abstraction interface (Windows Hello, macOS TouchID, Linux Secret Service).

### 3.4 `shared/search` (`shared-search`)
- Embeds **Tantivy** search engine directly in-process; zero external search processes.
- Schema definitions tailored for full-text code/log/file metadata search.
- Sub-50ms query parser supporting terms, phrases, wildcards, fuzzy matching, and boolean logic.
- In-memory fast index support for ephemeral datasets and on-disk persistent indices.

### 3.5 `shared/indexing` (`shared-indexing`)
- High-throughput parallel directory walker utilizing multi-core work stealing.
- Streaming backpressure channels with `tokio::sync::mpsc`.
- Fine-grained cancellation via `tokio::sync::watch` or cancellation tokens.
- Automatic respect for `.gitignore` files, system hidden files, and symbolic link loop prevention.

### 3.6 `shared/settings` (`shared-settings`)
- Safe local settings management stored in local atomic JSON/SQLite.
- Strongly-typed Serde serialization and validation with fallback defaults.
- Reactive event broadcasting for live configuration hot-reloading.

### 3.7 `shared/telemetry` (`shared-telemetry`)
- Strictly opt-in local performance counter and diagnostic monitor.
- **Default state:** 100% disabled; zero collection unless explicitly enabled by user.
- **Zero Network Egress:** Diagnostics are aggregated in-memory and persisted strictly to local disk upon user request.

### 3.8 `shared/updater` (`shared-updater`)
- Standalone offline binary update verification.
- Validates SHA-256 binary checksums and Ed25519 cryptographic signatures against embedded public keys.
- Implements staged atomic replacement with rollback backup safeguards.

### 3.9 `shared/plugins` (`shared-plugins`)
- Dynamic plugin interface for parsers, syntax highlighters, and data sinks.
- Strictly bounded execution with timeouts and memory quotas to prevent application lockup.

### 3.10 `platform` (`platform-common`)
- Cross-platform OS capabilities abstraction.
- Native standard paths (`%LOCALAPPDATA%`, `~/.local/share`, `~/Library/Application Support`).
- In-process resource monitor measuring instantaneous process RSS memory and CPU percentage.

---

## 4. Concurrency & Memory Management Model

1. **Async Runtime:** Tokio multi-threaded runtime handles I/O-bound tasks (file streaming, database queries, IPC).
2. **CPU-Bound Computation:** Dedicated rayon thread pools handle parallel SIMD log scanning, file hash calculations, and cryptographic key derivations.
3. **Zero-Copy Memory Mapping:** Universal Log Viewer leverages `memmap2` to project multi-gigabyte log files into virtual memory address space without reading files into heap memory.
4. **Memory Zeroization:** Plaintext secrets never escape bounded scopes and are protected by `zeroize` memory clearing.
