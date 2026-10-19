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

---

## ADR-0009: Lock-Free Sparse Line-Offset Indexing Structure for Fast Line Addressing

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Storing byte offsets for every newline in a 500GB log file containing 5 billion lines would require $5 \times 10^9 \times 8\,\text{bytes} \approx 40\,\text{GB}$ of RAM, severely violating the 300MB maximum RAM constraint.
- **Alternatives Considered:**
  1. *Full array of 64-bit line offsets:* Consumes tens of gigabytes of heap memory on large files.
  2. *Scanning from beginning on every scroll seek:* Unacceptable latency ($> 10\,\text{seconds}$) when seeking towards the end of a 500GB file.
  3. *Sparse checkpoint indexing (every $K=128$ lines):* Records a 64-bit byte offset only every 128th newline. Looking up line $N$ computes checkpoint index $N / 128$ in $O(1)$ time, seeks to that byte offset, and scans at most 127 newlines using SIMD `memchr`.
- **Decision:** Adopt the sparse checkpoint indexing structure with checkpoint frequency $K=128$ backed by contiguous 64-bit vectors.
- **Rationale:** For a 500GB log with 5 billion lines, storing 1 out of 128 offsets requires only $312\,\text{MB}$ in the worst theoretical case, and for typical 100GB logs ($10^9$ lines), consumes under $60\,\text{MB}$ of RAM. Resolving any line takes $< 1\,\text{microsecond}$.
- **Consequences:** Line seeking incurs a deterministic microsecond-level linear scan of at most 127 newlines, easily masked within 60 FPS frame budgets.

---

## ADR-0010: Tauri v2 IPC Command and Event Streaming Architecture for Live Tail Mode

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Streaming real-time appended lines from high-throughput log writers (e.g., 50,000 lines/sec) directly into a webview UI can flood IPC channels and cause dropped frames or UI freezing.
- **Alternatives Considered:**
  1. *Polling via Tauri IPC commands (`get_new_lines` every 100ms):* Inefficient when logs are idle, yet loses lines during high-frequency write bursts.
  2. *WebSocket server running on localhost:* Introduces network stack overhead and local port conflicts, violating the zero-network mandate.
  3. *Tauri v2 event streaming with backpressure coalescing:* File watcher streams new byte offsets into an in-process batcher that debounces event emissions to 60 FPS (16ms windows), transferring line chunks via binary or structured IPC payloads.
- **Decision:** Implement event-driven streaming with a 16ms coalescing debounce buffer via Tauri v2 event channels.
- **Rationale:** Guarantees that the UI never receives more than 60 updates per second, completely preventing IPC congestion while delivering smooth 60 FPS live updates.
- **Consequences:** At peak write rates, line updates arrive in micro-batches rather than individual line-by-line events.

---

## ADR-0011: Virtualized DOM Rendering Strategy with Reusable Row Pools in Svelte

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Rendering millions of DOM elements in a web browser crashes the browser tab and exceeds the 300MB RAM limit.
- **Alternatives Considered:**
  1. *Pagination (1000 lines per page):* Discontinuous user experience; breaks smooth vertical scrolling and visual pattern recognition.
  2. *Infinite scroll with DOM accumulation:* Continues adding nodes until browser memory is exhausted.
  3. *Strict DOM virtualization with recycled row pools:* Renders only the visible rows ($(\text{viewport height} / \text{row height}) + 10$ buffer rows, typically ~50 rows total). The scrollbar height represents the virtual total document height.
- **Decision:** Implement a custom virtualized viewport in Svelte that maintains a fixed pool of ~50 DOM rows, dynamically updating row content and offsets based on scroll position.
- **Rationale:** Keeps frontend DOM memory constant ($\approx 5\,\text{MB}$) regardless of whether the log file contains 1,000 lines or 100,000,000 lines. Guarantees 60 FPS scrolling.
- **Consequences:** Requires fixed or estimated line heights and dynamic slice querying from the Rust core.
