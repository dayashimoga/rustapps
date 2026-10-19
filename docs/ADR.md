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

---

## ADR-0012: Zero-Copy and Streaming Parsers for Structured JSON and CSV Logs

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Structured log formats (JSON Lines, CSV/TSV, CloudWatch exports) can reach tens of gigabytes. Reading entire files into DOM or deserializing massive JSON arrays into in-memory structures violates the $< 300\,\text{MB}$ RAM ceiling and causes out-of-memory crashes.
- **Alternatives Considered:**
  1. *Full Document Deserialization into Vectors:* High memory allocation proportional to file size ($> 2\,\text{GB}$ for moderate logs).
  2. *Regex-only extraction:* Fast but fragile across nested JSON objects, quoted CSV strings, and escaped characters.
  3. *Streaming Line-by-Line Parsers with Zero-Copy Borrows:* Stream records using `serde_json::Deserializer::from_slice` and `csv::ReaderBuilder` on mmap-backed line slices, borrowing string slices where possible and maintaining only windowed caches in RAM.
- **Decision:** Implement zero-copy and iterator-based streaming parsers in `parsers.rs` that deserialize structured records on-demand from mmap byte slices.
- **Rationale:** Sustains $> 100\,\text{MB/s}$ parse throughput while keeping resident memory $< 50\,\text{MB}$ regardless of log file size.
- **Consequences:** Nested deeply structured payloads are selectively flattened into standard normalized keys (`timestamp`, `severity`, `message`, `fields`).

---

## ADR-0013: On-Device Root-Cause Analysis via Drain Template Mining and Statistical Spike Detection

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Debugging enterprise log dumps containing millions of events requires automated template identification, anomaly detection, and error clustering without relying on external cloud LLMs, SaaS endpoints, or heavy local neural weights that exceed 300MB RAM.
- **Alternatives Considered:**
  1. *Embedded LLM (e.g. LLaMA / ONNX 7B model):* Requires $> 4\,\text{GB}$ VRAM/RAM and multiple seconds per inference, violating offline lightweight desktop constraints.
  2. *Simple string grouping:* Fails to generalize variable parameters (IP addresses, UUIDs, IDs, durations) causing thousands of near-duplicate groups.
  3. *Drain Log Parsing Algorithm with Prefix Tree and Regex Tokenization:* Proven state-of-the-art algorithmic log miner (He et al.) utilizing bounded depth prefix trees and regex masking to mine log templates in linear $O(N)$ time with deterministic sub-millisecond per-log classification.
- **Decision:** Implement Drain template mining alongside statistical rolling z-score and IQR spike detection in `local_ai.rs`.
- **Rationale:** 100% offline, zero external model files, operates in $< 20\,\text{MB}$ RAM, and processes 100,000 log lines in under $500\,\text{ms}$.
- **Consequences:** Templates are structural patterns rather than natural-language summaries, perfectly suited for high-fidelity log root-cause analysis.

---

## ADR-0014: Offline Kubernetes and CloudWatch Correlation via Local Metadata Extraction and Time-Window Clustering

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Kubernetes and CloudWatch logs exported from distributed systems contain interleaved records across multiple pods, containers, and streams. Correlating cascading failures across services requires multi-dimensional tracking without connecting to live cluster APIs.
- **Alternatives Considered:**
  1. *Online cluster querying via Kubernetes client API / AWS SDK:* Violates strict offline/standalone nonnegotiable constraint.
  2. *Flat chronological sorting only:* Obscures per-pod causal relationships and interleaves unrelated microservices.
  3. *Offline multi-dimensional indexing with sliding correlation windows:* Parse Kubernetes path conventions (`/var/log/pods/<ns>_<pod>_<id>/<container>/`) and CRI/CloudWatch envelope formats, index by `(namespace, pod, container, stream)`, and run windowed failure correlation.
- **Decision:** Implement offline CloudWatch and Kubernetes normalizers in `cloud_importers.rs` with bidirectional cross-container error propagation tracking.
- **Rationale:** Delivers enterprise-grade correlation and timeline filtering entirely from exported disk archives with zero network egress.
- **Consequences:** Requires users to supply raw log export directories or dump files.

---

## ADR-0015: UI-Agnostic Reusable `vault-core` Library with Unified Tauri & Flutter C-ABI FFI

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** DevOps Secret Vault must support desktop (Tauri v2 + Svelte on Windows, Linux, macOS) and mobile (Phase 8 Flutter on Android and iOS) without duplicating cryptographic, storage, or business logic.
- **Alternatives Considered:**
  1. *Dual implementations in Rust (desktop) and Dart/C++ (mobile):* Violates the single-source-of-truth invariant, doubles testing and security audit overhead, and risks cryptographic divergence.
  2. *REST / Local IPC daemon between Rust and Mobile UI:* Adds network attack surfaces, background battery drain, and OS sandboxing failures on iOS.
  3. *Pure UI-Agnostic Rust core (`vault-core`) with Dual Surface Bindings:* Structure the vault core as a standalone Rust crate compiled as `rlib` for Tauri desktop commands and `cdylib` / `staticlib` exporting clean C-ABI FFI endpoints for Flutter `dart:ffi`.
- **Decision:** Architect `apps/devops-secret-vault` as a pure, UI-independent Rust library with high-level safe services and standardized C-ABI foreign function interface.
- **Rationale:** Guarantees 100% cryptographic and data integrity parity between desktop and mobile while maintaining zero runtime overhead and zero network exposure.
- **Consequences:** Internal structs and pointers passed across FFI boundaries require opaque handle encapsulation and deterministic lifecycle destruction methods.

---

## ADR-0016: Master Key Derivation via Argon2id and Strict In-Memory Zeroization Lifecycle

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Secrets stored in local databases and cached in memory are vulnerable to offline brute-force attacks and memory dump / cold-boot extraction.
- **Alternatives Considered:**
  1. *PBKDF2 or standard SHA-256:* Inadequate resistance against modern GPU/ASIC hardware acceleration.
  2. *Plain string heap allocations for decrypted credentials:* Plaintext strings linger indeterminately in allocator memory pages, exposed to swap-file leaks and heap inspection.
  3. *Argon2id KDF with Ephemeral Zeroizing Secret Wrappers:* Derive encryption keys using memory-hard Argon2id ($m=64\,\text{MiB}, t=3, p=4$) and wrap all plaintext credentials, intermediate keys, and decrypted buffers in `zeroize::Zeroizing<T>` types that overwrite memory with zeros immediately on drop.
- **Decision:** Mandate Argon2id for all key derivation and enforce `zeroize` on all decrypted plaintext, symmetric keys, and clipboard buffers.
- **Rationale:** Provides state-of-the-art resistance against GPU password cracking and guarantees that unencrypted sensitive materials never persist in RAM beyond their immediate functional scope.
- **Consequences:** Key derivation takes approximately $100$–$200\,\text{ms}$ on startup, well within the $< 2,000\,\text{ms}$ cold start SLA.

---

## ADR-0017: Ephemeral Encrypted In-Memory Tantivy Indexing for Fast Secret Discovery

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Users with large secret databases (100,000+ items) require instant ($< 50\,\text{ms}$) search across metadata, titles, usernames, tags, and notes, but persisting plaintext search index tokens to disk would violate zero-knowledge local encryption.
- **Alternatives Considered:**
  1. *Sequential linear scan in SQLite via SQL LIKE queries:* Degradation to $> 500\,\text{ms}$ for large inventories, failing the 50ms SLA.
  2. *Disk-persisted Tantivy index:* Exposes indexed secret metadata in plaintext files on disk or requires double-encryption layer complexity.
  3. *Encrypted In-Memory Tantivy Index:* Build an ephemeral Tantivy RAM index populated during vault unlock and cleared on lock. Queries run against in-memory inverted indices in sub-millisecond time.
- **Decision:** Implement ephemeral in-memory Tantivy indexing for secret search metadata populated on vault unlock and completely destroyed on lock.
- **Rationale:** Achieves $< 2\,\text{ms}$ query latency across 100,000 items while ensuring zero unencrypted index files ever touch non-volatile disk.
- **Consequences:** Vault unlock indexes metadata into RAM; bounded by $< 30\,\text{MB}$ memory footprint, easily satisfying the 300MB RAM limit.

---

## ADR-0018: RFC 6238 TOTP Engine with Zero-External Clock Sync

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Two-factor authentication (TOTP) codes must be generated and verified accurately without external NTP network queries in strictly air-gapped offline environments.
- **Alternatives Considered:**
  1. *Network NTP time synchronization:* Violates strict offline/standalone nonnegotiable constraint.
  2. *OS monotonic / system clock with sliding tolerance window:* Read OS system timestamp and evaluate standard 30-second epoch counter with configurable drift tolerance ($\pm 1$ step).
- **Decision:** Implement native RFC 6238 TOTP generator with Base32 decoding, HMAC-SHA1/SHA256 support, and zero external network queries.
- **Rationale:** Guarantees 100% offline compliance while maintaining compatibility with standard authenticator apps and services.
- **Consequences:** Relies on local OS system time synchronization.

---

## ADR-0019: Automated X.509 Certificate Chain Inspection & Cryptographic Weakness Detection

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Developers manage TLS/SSL certificates and SSH keys that expire unnoticed or use deprecated algorithms (e.g. SHA-1, MD5, RSA-1024), causing production outages.
- **Alternatives Considered:**
  1. *Shelling out to external OpenSSL binary:* Incompatible across platforms; violates zero external dependency rule.
  2. *Native Rust X.509 ASN.1/DER/PEM parser embedded in vault-core:* Directly parse certificate structures, extract validity dates, Subject Alternative Names (SANs), key lengths, and flag weak algorithms offline.
- **Decision:** Implement embedded X.509 certificate parsing and weakness detection directly in `vault-core`.
- **Rationale:** Provides instant, offline expiration alerts and security posture evaluation within sub-millisecond execution times and zero external binaries.
- **Consequences:** Parses standard X.509 v3 certificates and PEM formatting; unsupported proprietary certificate formats are flagged gracefully.

---

## ADR-0020: On-Device Vault Risk Scoring & Differential Credential Reuse Detection

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Users need comprehensive visibility into credential hygiene (entropy, reuse, age, default passwords) without exposing plaintext secrets to inspection.
- **Alternatives Considered:**
  1. *Cloud-based credential audit APIs (e.g. HaveIBeenPwned API):* Violates offline-first constraint and leaks hashes over the network.
  2. *Plaintext cross-matching in memory:* Risks lingering sensitive string allocations in RAM.
  3. *Local Differential Risk Scoring with Zero-Knowledge Hash Bucketing:* Calculate Shannon entropy, rotation age, evaluate against a curated offline list of default developer credentials, and hash secret payloads with an ephemeral in-memory salt to identify duplicates without storing plaintext matches.
- **Decision:** Implement local zero-knowledge risk scoring and Health Dashboard evaluation in `vault-core`.
- **Rationale:** Delivers enterprise-grade security posture analysis entirely on-device with zero data exfiltration.
- **Consequences:** Risk scoring runs on-demand during vault unlock or audit requests in $< 20\,\text{ms}$.

---

## ADR-0021: Tamper-Evident Encrypted Backup Archive Format with Argon2id + AES-256-GCM

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Users require portable encrypted backup archives to migrate or preserve secret inventories across machines without risk of tampering or corruption.
- **Alternatives Considered:**
  1. *Plain JSON/CSV exports:* Extremely high security risk if saved unencrypted on shared or untrusted storage.
  2. *Standard ZIP password encryption (ZipCrypto/PKZIP):* Trivially breakable.
  3. *Authenticated `.vaultbackup` Archive Envelope:* Standardized binary envelope with magic bytes (`VAULT_BACKUP_V1`), salt, Argon2id parameters, AES-256-GCM ciphertext, and cryptographic integrity verification.
- **Decision:** Implement authenticated `.vaultbackup` archive format using Argon2id key derivation and AES-256-GCM encryption.
- **Rationale:** Guarantees that backups cannot be decrypted without the backup passphrase and cannot be tampered with or corrupted without immediate cryptographic detection.
- **Consequences:** Backups are self-contained, offline-first portable binary files.

---

## ADR-0022: Deterministic 3-Tier Duplicate Detection Pipeline

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Detecting duplicate files across hundreds of gigabytes or millions of files by computing cryptographic hashes for every file creates devastating disk I/O bottlenecks and saturates system storage busses.
- **Alternatives Considered:**
  1. *Full cryptographic hashing of every file:* Disastrous performance on multi-gigabyte or millions of files; reads 100% of data from disk.
  2. *Filename and timestamp matching:* Extremely high false-negative and false-positive rate; unreliable.
  3. *Deterministic 3-Tier Multi-Stage Pipeline:*
     - **Stage 1 (Metadata Grouping):** Group all candidate files by exact file size in bytes ($O(1)$ stat lookup). Discard unique file sizes immediately without reading a single byte.
     - **Stage 2 (4KB Boundary Hashing):** For candidate groups with identical size, read only the first 4KB and last 4KB into memory and compute an ultra-fast boundary hash. Discard mismatches.
     - **Stage 3 (Full Cryptographic BLAKE3 Verification):** For matching partial-hash candidate pairs, stream full-file contents through BLAKE3 SIMD hashing ($> 3\,\text{GB/s}$) to guarantee zero false positives.
- **Decision:** Implement the 3-Tier Multi-Stage Duplicate Detection Pipeline (Size $\rightarrow$ 4KB Header/Footer $\rightarrow$ Full BLAKE3).
- **Rationale:** Eliminates $> 95\%$ of unnecessary disk I/O on typical filesystems while delivering 100% cryptographically guaranteed duplicate accuracy.
- **Consequences:** Files with identical sizes and identical headers/footers require streaming the entire file; BLAKE3's tree-hashing minimizes CPU overhead.

---

## ADR-0023: High-Throughput Concurrent Filesystem Indexing within 300MB RAM SLA

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Indexing 10,000,000 files in traditional desktop search engines consumes gigabytes of heap memory and freezes the UI thread, breaching the $< 300\,\text{MB}$ RAM and $< 50\,\text{ms}$ query SLAs.
- **Alternatives Considered:**
  1. *In-memory vectors and structs for all 10M files:* Consumes $> 2\,\text{GB}$ of heap RAM, breaching the 300MB SLA.
  2. *SQLite database table for metadata:* Slower full-text search across filenames and paths; higher disk write overhead.
  3. *Embedded Tantivy Index with Streaming Worker Batches:* Traversal streams metadata through bounded Tokio mpsc channels ($1,024$ items) into a dedicated Tantivy indexing pipeline. Writer commits are batched every 50,000 records or 500ms. Queries execute against memory-mapped Tantivy segments.
- **Decision:** Use `shared-indexing` bounded crawler streaming into `shared-search` Tantivy index with dedicated schema for file metadata (path, filename, extension, size, modified timestamp, is_directory).
- **Rationale:** Keeps memory usage strictly bounded ($< 150\,\text{MB}$ working RAM) while indexing $> 100,000\,\text{files/s}$ and executing complex multi-field searches in $< 50\,\text{ms}$.
- **Consequences:** Initial full-drive crawl runs asynchronously in the background with cooperative yielding.

---

## ADR-0024: Transactional Atomic Batch Renaming with Rollback Journal

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Batch renaming hundreds or thousands of files using regex or numbering rules can fail mid-operation (e.g. permission error, naming collision, disk full), leaving the filesystem in a corrupted, half-renamed state.
- **Alternatives Considered:**
  1. *Immediate direct renaming:* Any mid-run failure corrupts state with no automatic recovery.
  2. *Copy-and-delete:* Massive disk I/O and doubles storage space consumption.
  3. *Transactional Two-Phase Rename with Rollback Journal:*
     - Phase 1 (Dry-Run Validation): Validate all source paths exist, target names have no collisions with each other or existing disk files, and permissions are valid.
     - Phase 2 (Execution with Step-by-Step Journal): Record each completed rename step in an in-memory journal. If an I/O error occurs at step $N$, automatically roll back steps $N-1 \dots 1$ in reverse order.
- **Decision:** Implement a transactional atomic batch rename engine with dry-run collision detection and automatic reverse-journal rollback.
- **Rationale:** Protects user data from filesystem corruption and guarantees all-or-nothing atomicity.
- **Consequences:** Circular renames (e.g., $A \rightarrow B$ and $B \rightarrow A$) require an intermediate temporary rename step.

---

## ADR-0025: Local Hierarchical Storage Treemap Layout

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Visualizing multi-gigabyte disk consumption requires hierarchical drill-down that renders smoothly at 60 FPS without UI jank.
- **Alternatives Considered:**
  1. *DOM-heavy nested `<div>` tree:* Stalls browser layout engine when rendering thousands of folder boxes.
  2. *Squarified Treemap Algorithm on HTML5 Canvas:* Compute rectangular partitions using the squarified treemap layout algorithm (minimizing aspect ratio distortion) and render directly onto an HTML5 Canvas with sub-millisecond hover hit-testing.
- **Decision:** Compute folder size aggregation in the Rust core and render squarified treemaps using hardware-accelerated Canvas in Svelte.
- **Rationale:** Guarantees 60 FPS rendering, instant zooming/drilling, and minimal browser memory consumption.
- **Consequences:** Drill-down re-aggregates children dynamically from the folder tree index.

---

## ADR-0026: Offline Image & Scanned PDF OCR Extraction Pipeline

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Extracting search terms from image files (PNG, JPEG, BMP) and scanned PDFs typically relies on cloud vision APIs (AWS Textract, Google Cloud Vision) or heavy native dynamic library bindings (Tesseract C++ / Leptonica), violating offline-first and zero-dependency constraints.
- **Alternatives Considered:**
  1. *Cloud Vision APIs:* Strict violation of air-gap, privacy, and zero-cost constraints.
  2. *Tesseract C++ FFI bindings:* Requires system dynamic libraries (`libtesseract.so`, `leptonica.dll`), complicated cross-platform compilation, and large distribution size (>50MB).
  3. *In-Process Pure Rust Offline OCR Pipeline:*
     - Image loading and normalization (grayscale conversion, Otsu's adaptive thresholding binarization).
     - Connected-component segmentation into lines and words.
     - Normalized feature extraction ($8 \times 8$ density, topological loops, crossing counts) matched against an embedded offline glyph dictionary / neural weight matrix with fallback to embedded PDF text layers.
- **Decision:** Implement an in-process, pure-Rust offline OCR pipeline in `ocr.rs` for raster images and scanned PDF documents.
- **Rationale:** 100% offline, zero external dynamic libraries, memory-safe, $< 300\,\text{MB}$ RAM, and seamlessly streams extracted words into Tantivy schema fields.
- **Consequences:** Optimized for machine-printed text, screenshots, code snippets, and structured receipts; cursive handwritten recognition is out of scope.

---

## ADR-0027: Visual Perceptual Hashing (dHash) for Sub-Millisecond Screenshot Similarity

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Users capture frequent screenshots that differ only by minor resizing, compression artifacts, or slight crop differences. Cryptographic hashes (SHA-256, BLAKE3) fail because even a single altered bit invalidates the hash.
- **Alternatives Considered:**
  1. *Pixel-by-pixel Mean Squared Error (MSE):* High CPU overhead ($O(W \times H)$) and intolerant to resolution scaling.
  2. *Deep convolutional neural feature extractors (ResNet/ViT):* Heavy weight files (>100MB), slow CPU inference (>100ms per image), high RAM usage.
  3. *Perceptual Difference Hashing (dHash):*
     - Downsample image to a $9 \times 8$ grayscale matrix.
     - Compute horizontal gradient bits: $\text{bit}(x, y) = 1$ if $P(x+1, y) > P(x, y)$ else $0$.
     - Yields a compact 64-bit integer (`u64`) fingerprint.
     - Distance measured via bitwise XOR Hamming distance: `(h1 ^ h2).count_ones()`.
- **Decision:** Implement 64-bit Perceptual Difference Hashing (dHash) combined with Tantivy OCR text search in `screenshot.rs`.
- **Rationale:** Fingerprint computation takes $< 100\,\text{µs}$; Hamming distance comparison completes in $< 1\,\text{µs}$ (single CPU cycle `popcnt`), allowing real-time similarity clustering across tens of thousands of screenshots.
- **Consequences:** Images must be downscaled to $9 \times 8$ during indexing.

---

## ADR-0028: Multi-Format Document Text and Metadata Extraction Engine

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Comprehensive full-text search requires indexing textual content and metadata across diverse document formats: PDF, DOCX, Markdown, source code, and archives, without external runtime binaries (`pdftotext`, `pandoc`, `unzip`).
- **Alternatives Considered:**
  1. *External CLI tools:* High security risk, OS portability hurdles, and external process spawning latency.
  2. *Heavy Java/Python server services:* Violates strict offline-first and lightweight client rules.
  3. *Pure-Rust Streaming Extractors in `doc_extract.rs`:*
     - *PDF:* Parser extracting body objects, FlateDecode decompression, font encoding maps, text operators (`BT`, `ET`, `Tj`, `TJ`), and `/Info` dictionary metadata.
     - *DOCX:* In-memory ZIP archive reader streaming `word/document.xml` for `<w:t>` text nodes and `docProps/core.xml` for Dublin Core metadata.
     - *Markdown:* Fast scanner extracting title headers, YAML/TOML frontmatter, and stripping syntax tokens.
     - *Source Code:* Language detector extracting module docstrings, function signatures, and code tokens across 20+ programming languages.
     - *Archives:* Container scanner indexing archive manifests and inner text files under 1MB.
- **Decision:** Implement native in-process streaming document extractors directly within `apps/ai-file-explorer` feeding into the Tantivy indexing pipeline.
- **Rationale:** 100% offline, zero external dependencies, bounded heap memory ($< 50\,\text{MB}$ working RAM), and sub-millisecond per-file parsing throughput.
- **Consequences:** Password-protected or DRM-locked files require explicit user authentication prior to extraction.

---

## ADR-0029: 8-Bit Quantized Local Embedding Inference & Vector Similarity Index for Semantic Search

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Keyword-only queries fail when users search using conceptual or natural-language phrasing (e.g. searching "database credential leak" should match "unencrypted postgres password"). External vector databases and large LLMs exceed our strict $< 300\,\text{MB}$ RAM and $< 50\,\text{ms}$ query SLAs.
- **Alternatives Considered:**
  1. *Remote vector embedding APIs (OpenAI, Cohere):* Strict violation of air-gap constraint and zero-cost policy.
  2. *Heavy local LLMs (Ollama, Llama.cpp 7B):* Requires 4GB–16GB RAM and dedicated GPU acceleration.
  3. *In-Process 8-Bit Quantized Embedding Model & Cosine Vector Index (`embeddings.rs`):*
     - 64-dimensional dense semantic vector space.
     - 8-bit integer quantization (`[i8; 64]`) consuming only 64 bytes per document.
     - Character n-gram and subword token hasher with positional encoding and trained projection weights.
     - In-memory contiguous vector bank with SIMD-accelerated integer dot products ($A \cdot B = \sum A_i B_i$) for instant cosine similarity.
- **Decision:** Implement an in-memory 8-bit quantized embedding inference engine and vector similarity index in `embeddings.rs`.
- **Rationale:** 100,000 document vectors occupy only $6.4\,\text{MB}$ of resident RAM; vector query across 100,000 items executes in $< 2\,\text{ms}$ on standard CPU cores; 100% offline and zero cloud dependencies.
- **Consequences:** Calibrated for technical documentation, code, file paths, and general English corpus search.

---

## ADR-0030: Cross-Platform Multi-Target Matrix & Deterministic Packaging Pipelines

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** All three applications (DevOps Secret Vault, AI File Explorer, Universal Log Viewer) must compile and distribute as first-class native standalone binaries across the complete tier-1 desktop platform matrix: Windows x64 and ARM64, Linux x64 and ARM64, and macOS Intel and Apple Silicon, packaged into native distribution bundles (MSI/EXE, AppImage/DEB/RPM, DMG/Universal binary).
- **Alternatives Considered:**
  1. *Electron or Web Container:* Massive memory footprint (>200MB baseline per app), high startup latency (>2s), violates strict memory SLAs.
  2. *Single-target x86_64 host compilation only:* Leaves Apple Silicon, Windows ARM64 Copilot+ PCs, and Linux ARM64 workstations dependent on translation layers (Rosetta, Prism), sacrificing performance.
  3. *6-Target GitHub Actions Matrix + Cross-Compilation Pipelines:*
     - `x86_64-pc-windows-msvc` (Windows x64): Native MSVC compilation, WiX Toolset MSI installer, portable ZIP.
     - `aarch64-pc-windows-msvc` (Windows ARM64): Native MSVC cross-compilation, MSI installer.
     - `x86_64-unknown-linux-gnu` (Linux x64): Native GNU compilation, AppImage bundle, DEB package, RPM package.
     - `aarch64-unknown-linux-gnu` (Linux ARM64): `cross` Docker cross-compilation, DEB, RPM, tarball.
     - `x86_64-apple-darwin` (macOS Intel): Clang cross-target compilation.
     - `aarch64-apple-darwin` (macOS Apple Silicon): Native Xcode/Clang compilation, `lipo` Universal Binary generation, `.app` bundle, and signed DMG creation.
- **Decision:** Standardize CI/CD on a 6-target build and release matrix with scripted reproducible packaging harnesses in `scripts/`.
- **Rationale:** Guarantees zero runtime performance penalties from emulation across ARM64 hardware (MacBook M-series, Snapdragon X Elite, Raspberry Pi / AWS Graviton desktops) while keeping 100% open-source toolchain integrity.
- **Consequences:** CI pipelines must manage multi-architecture cross-compilation dependencies (LLVM/Clang sysroots for ARM64).

---

## ADR-0031: Native Platform Integrations & Security Keyring Abstraction

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Desktop applications require native OS integration for standard directory resolution, dark/light theme synchronization, power/sleep lifecycle telemetry, and hardware-backed biometric security (Touch ID, Windows Hello, Secret Service), while strictly maintaining `#![forbid(unsafe_code)]` in portable crates and zero cloud dependencies.
- **Alternatives Considered:**
  1. *Hardcoded fallback paths (`~/.appname`):* Violates OS platform design guidelines (Apple App Store rules, XDG specification, Windows Known Folders).
  2. *Proprietary cloud keystores (AWS KMS, Azure Key Vault):* Violates nonnegotiable air-gap and zero-cost constraints.
  3. *Tiered Platform Abstraction in `platform-common` and `shared-encryption`:*
     - *Directory Resolution:* Canonical mapping: XDG Base Directory Specification on Linux, Apple Standard Paths (`~/Library/Application Support`, `~/Library/Caches`) on macOS, and Windows Known Folders (`%APPDATA%`, `%LOCALAPPDATA%`).
     - *Theme Detection:* Real-time queries for OS dark/light mode across Windows Registry (`AppsUseLightTheme`), macOS `defaults read -g AppleInterfaceStyle`, and Linux D-Bus `org.freedesktop.portal.Settings`.
     - *Keyring & Biometrics:* Uniform trait abstraction delegating to Windows Credential Manager / Windows Hello on Windows, macOS Keychain / LocalAuthentication (Touch ID) on macOS, and Freedesktop Secret Service API / Polkit on Linux.
- **Decision:** Implement a clean, decoupled platform integration and keyring abstraction layer in `platform-common` and `shared-encryption`.
- **Rationale:** Preserves native OS look, feel, conventions, and hardware security posture across all supported operating systems without compromising security boundaries or portability.
- **Consequences:** Platform-specific FFI boundaries must be rigorously tested with mock adapters in headless CI environments.

---

## ADR-0032: C-ABI Dynamic FFI & Flutter Architecture for Mobile DevOps Secret Vault

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** DevOps Secret Vault must be accessible on Android and iOS mobile devices while maintaining zero code duplication of core cryptography (Argon2id, AES-256-GCM, Ed25519), SQLite storage, TOTP generation (RFC 6238), and 5-dimensional security risk scoring.
- **Alternatives Considered:**
  1. *Rewrite vault logic in Kotlin and Swift:* High risk of cryptographic divergence, audit overhead, and duplicated maintenance burdens.
  2. *React Native with JavaScript bridge:* Slower startup, larger binary footprint, potential bridge overhead and memory leakage.
  3. *Flutter UI with C-ABI FFI bindings (`dart:ffi`) to Rust `vault_core`:*
     - Rust exposes a stable C-ABI (`pub unsafe extern "C"`) in `apps/devops-secret-vault/src/ffi.rs`.
     - Raw pointers and JSON strings are passed across language boundaries with explicit allocation and zero-leak deallocation via `vault_free_string`.
     - High-level Flutter/Dart service layer provides asynchronous API, reactive state management, and mock mode for tests and development.
- **Decision:** Implement a C-ABI FFI dynamic binding architecture connecting a Flutter mobile application directly to compiled `vault_core` binaries (`libvault_core.so` on Android, `vault_core.framework` on iOS).
- **Rationale:** Delivers 100% cryptographic parity, air-gapped security assurance, native 60/120fps UI rendering, and single-source-of-truth domain models.
- **Consequences:** Mobile builds require compiled Rust target binaries (`aarch64-linux-android`, `armv7-linux-androideabi`, `aarch64-apple-ios`) bundled into application archives.

---

## ADR-0033: Mobile Hardware Biometric Security & Encrypted Container Persistence

- **Status:** Approved
- **Date:** 2026-10-19
- **Problem:** Mobile users require friction-free biometric authentication (fingerprint, Face ID) while ensuring master encryption keys are never stored in plaintext on disk or in shared memory.
- **Alternatives Considered:**
  1. *Plaintext key storage in SharedPreferences / UserDefaults:* Highly vulnerable to root/jailbreak inspection and physical extraction.
  2. *Cloud-based token authentication:* Violates zero-cloud egress and offline-first architectural principles.
  3. *Hardware-Backed Keystore with Biometric Authorization:*
     - On Android: AndroidKeyStore with `KeyGenParameterSpec` backed by StrongBox / TEE requiring `BiometricPrompt` user authentication.
     - On iOS: LocalAuthentication / Security framework with `kSecAccessControlBiometryAny` backed by Secure Enclave.
     - Ephemeral Master Key is decrypted into memory upon biometric validation and zeroized immediately upon app backgrounding or explicit lock.
- **Decision:** Implement hardware-backed biometric authentication in `biometric_service.dart` paired with automated clipboard clearing (45-second timer) and zeroized memory lifecycles.
- **Rationale:** Balances enterprise-grade security with modern mobile ergonomics without sending data across network interfaces.
- **Consequences:** Devices lacking biometric hardware or user enrollment gracefully fall back to master password entry.


