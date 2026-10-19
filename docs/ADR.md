# Architecture Decision Records (ADR)

This document records the foundational architectural decisions made for the monorepo and its three applications. Every entry adheres strictly to the mandated schema: **Problem**, **Alternatives**, **Decision**, **Rationale**, and **Consequences**.

---

## ADR-0001: Monorepo Structure and Shared Reusable Rust Core

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Building three high-performance desktop/mobile applications with independent codebases would lead to duplication of logging, storage, cryptography, search, indexing, and platform abstractions, increasing maintenance costs and security vulnerabilities.
- **Alternatives Considered:**
  1. *Multi-repo architecture:* Separate repositories for each app and shared libraries published to a private/public crates.io registry.
  2. *Single monolithic binary:* One executable containing all three apps switched by subcommands or tabs.
  3. *Cargo monorepo with shared internal crates:* Single repository containing `apps/`, `shared/`, `platform/`, `docs/`, `scripts/`, `tests/`, and `benchmarks/`.
- **Decision:** Adopt a Cargo monorepo with fine-grained shared internal crates in `/shared/` and a common platform layer in `/platform/`.
- **Rationale:** Monorepos allow atomic commits across shared libraries and consumer apps, unified CI/CD pipelines, consistent versioning, and zero duplicate code.
- **Consequences:** Workspace builds must be carefully managed to prevent unnecessary recompilation; crate boundaries must be strictly enforced via `deny.toml` and clean dependency paths.

---

## ADR-0002: Tantivy Embedded Search Engine for Offline Sub-50ms Indexing

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** All three applications require ultra-fast, structured and full-text search across large datasets (10M files, 100k secrets, 500GB logs) without relying on external search daemons like Elasticsearch, Solr, or Meilisearch.
- **Alternatives Considered:**
  1. *SQLite FTS5:* Built-in full-text search extension inside SQLite.
  2. *Custom inverted index:* Hand-crafted in-memory indexing library.
  3. *Tantivy:* Pure Rust, Lucene-inspired embedded full-text search engine library.
- **Decision:** Embed **Tantivy** directly in-process via `shared-search`.
- **Rationale:** Tantivy is pure Rust, highly optimized, supports schema-based indexing, fast BM25 scoring, boolean and fuzzy queries, and runs completely in-process with sub-50ms query latency on multi-million record datasets.
- **Consequences:** Tantivy introduces a modest binary size addition (~3MB) and requires careful index segment merge scheduling to avoid CPU contention.

---

## ADR-0003: SQLite WAL Mode and SQLCipher Cryptographic Storage with Zeroize

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** DevOps Secret Vault and application settings require structured relational persistence with transactional guarantees, instant queries, transparent encryption at rest, and zero memory remnants after use.
- **Alternatives Considered:**
  1. *Flat JSON files with AES encryption:* Prone to data corruption during crashes; poor query performance.
  2. *RocksDB / Sled:* Key-value storage lacks relational queries and battle-tested SQLCipher encryption.
  3. *SQLite with Write-Ahead Logging (WAL) and SQLCipher / AES-256-GCM:* Industrial standard for embedded relational data with zero-knowledge encryption.
- **Decision:** Standardize on SQLite in WAL mode with SQLCipher and AES-256-GCM payload encryption, wrapping all plaintext and keys in `zeroize::Zeroizing<T>`.
- **Rationale:** SQLite WAL enables concurrent readers without blocking writers. SQLCipher and AES-256-GCM provide authenticated encryption. `zeroize` guarantees memory is securely scrubbed when dropped.
- **Consequences:** Requires compiling the bundled SQLite/SQLCipher C source or linking platform crypto libraries.

---

## ADR-0004: Memory-Mapped I/O (`memmap2`) and Sparse Line-Offset Indexing for Universal Log Viewer

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Opening 100GB to 500GB log files in traditional text editors or viewers exhausts system RAM and takes minutes to load.
- **Alternatives Considered:**
  1. *Reading file into memory buffer (`Vec<u8>`):* Fails immediately for files larger than available RAM ($> 16\,\text{GB}$).
  2. *Chunk-by-chunk file streaming:* High I/O overhead when seeking backwards or scrolling arbitrarily.
  3. *Memory-mapped I/O (`memmap2`) with a sparse SIMD line-offset index:* Projects file into virtual memory; reads only pages currently visible on screen.
- **Decision:** Utilize `memmap2` paired with an asynchronous SIMD newline scanner building a sparse 64-bit line-offset index.
- **Rationale:** Memory mapping allows instantaneous opening of 500GB files with resident RAM remaining $< 250\,\text{MB}$. The OS kernel handles page caching and eviction automatically.
- **Consequences:** Memory mapping requires safe abstraction wrappers and handling of file truncation/invalidation edge cases.

---

## ADR-0005: Desktop UI Layer via Tauri v2, Svelte, and Strict TypeScript

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Desktop applications need responsive, modern, visually stunning user interfaces while maintaining cold startup $< 2,000\,\text{ms}$, idle CPU $< 2\%$, and total RAM $< 300\,\text{MB}$.
- **Alternatives Considered:**
  1. *Electron:* High memory overhead ($> 250\,\text{MB}$ idle), large installer size ($> 100\,\text{MB}$), slow startup.
  2. *Native Rust GUI (Iced / Slint / egui):* Less mature styling ecosystem, complex custom component styling, and limited rich typography/web animation support.
  3. *Tauri v2 + Svelte & TypeScript:* Uses OS native webview, minimal memory footprint (~30MB base), instant startup, excellent ecosystem, and strict TypeScript safety.
- **Decision:** Standardize desktop UI presentation on **Tauri v2** with **Svelte** and strict **TypeScript**.
- **Rationale:** Tauri v2 provides native performance and tiny installer sizes ($< 15\,\text{MB}$), while Svelte compiles away runtime framework overhead, delivering 60 FPS virtualized rendering.
- **Consequences:** Dependent on the host OS webview engine (WebView2 on Windows, WebKitGTK on Linux, WKWebView on macOS).

---

## ADR-0006: Local-Only AI and Quantized Embedding Inference

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** AI File Explorer requires semantic search and Universal Log Viewer requires anomaly clustering without transmitting any data to cloud AI APIs (OpenAI, Anthropic) or consuming excessive RAM.
- **Alternatives Considered:**
  1. *Local Ollama / LLaMA.cpp server:* Requires heavy background daemon and $> 4\,\text{GB}$ VRAM/RAM, violating the $< 300\,\text{MB}$ total RAM mandate.
  2. *External SaaS API calls:* Violates the strict 100% offline-first and zero-telemetry mandate.
  3. *Embedded quantized ONNX / tract models:* Small footprint (MiniLM or MobileNet embeddings, $< 40\,\text{MB}$ weights), running CPU/NPU inference in-process within memory limits.
- **Decision:** Embed lightweight quantized ONNX models using pure Rust runtime (`tract` / ONNX runtime bindings) executing strictly locally.
- **Rationale:** Ensures zero data leaves the user's workstation, maintains sub-50ms query latency, and remains well within the 300MB RAM ceiling.
- **Consequences:** Model selection is constrained to optimized, small-footprint embedding and clustering models ($< 50\,\text{MB}$).

---

## ADR-0007: Cryptographic Authenticity for Updates via Ed25519 Signatures

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Standalone binary updates must be verified completely offline to prevent binary tampering, supply chain injection, or corrupted installations.
- **Alternatives Considered:**
  1. *HTTPS certificate check:* Fails in air-gapped environments and does not guarantee binary integrity against server compromise.
  2. *Plain SHA-256 checksums:* Vulnerable to tampering if the checksum file is modified alongside the executable.
  3. *Asymmetric Ed25519 digital signatures:* The application embeds the developer's public key; updates are verified cryptographically before installation.
- **Decision:** Implement offline update verification in `shared-updater` using **Ed25519** digital signatures and SHA-256 checksums.
- **Rationale:** Ed25519 offers high performance, small 64-byte signatures, 32-byte public keys, and resistance to side-channel attacks.
- **Consequences:** Release manager must securely custody the offline Ed25519 private signing key.

---

## ADR-0008: Strict `#![forbid(unsafe_code)]` with ADR-Exempt Safe FFI/Mmap Wrappers

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Memory corruption, buffer overflows, and concurrency races are leading causes of security vulnerabilities in systems software.
- **Alternatives Considered:**
  1. *Permissive unsafe usage:* Allows ad-hoc `unsafe` blocks throughout the codebase, increasing audit surface.
  2. *Pure safe Rust without memory mapping:* Eliminates ability to perform zero-copy memory mapping for 500GB log files.
  3. *Global `#![forbid(unsafe_code)]` with isolated, audited exceptions:* Forbid unsafe code globally; encapsulate low-level OS mmap and FFI bindings inside strictly audited, minimal modules.
- **Decision:** Enforce `#![forbid(unsafe_code)]` across all application and shared crates. Isolate low-level memory mapping (`memmap2`) and OS biometrics FFI inside designated platform adapters with formal safety documentation.
- **Rationale:** Maximizes compiler-enforced memory safety guarantees while permitting essential systems-level capabilities.
- **Consequences:** All low-level interactions must provide safe, panic-free Rust abstractions to client crates.
