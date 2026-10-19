# Technical Requirements Specification (TRS)

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Open Source (OSI-Compliant)  
**Target Systems:** Windows (x64/ARM64), Linux (x64/ARM64), macOS (Apple Silicon/Intel), Mobile (Android/iOS for Vault)

---

## 1. Executive Summary & Core Mandates

This document establishes the comprehensive technical, functional, security, performance, and operational requirements for three standalone, ultra-fast, lightweight, offline-first applications:
1. **DevOps Secret Vault**: A secure, cryptographic vault for sensitive developer and operational secrets.
2. **AI File Explorer**: An intelligent, ultra-responsive local file browser, deduplicator, and semantic organizer.
3. **Universal Log Viewer**: A high-throughput, instantaneous log analysis engine capable of processing 500GB+ logs with sub-300MB RAM consumption.

### Nonnegotiable Axioms
- **Zero Proprietary Dependencies:** Exclusively OSI-approved open-source libraries and tooling.
- **100% Offline-First:** Zero network egress, zero SaaS/cloud dependencies, zero telemetry by default, zero external runtime services.
- **Strict Architecture Topology:** User $\rightarrow$ Desktop (Tauri v2 + Svelte/TypeScript) or Mobile (Flutter) $\rightarrow$ Reusable Rust Core $\rightarrow$ Local SQLite/SQLCipher $\rightarrow$ Native Filesystem.
- **No Heavy Frameworks:** Prohibited: Electron, Docker runtimes, Kubernetes clusters, PostgreSQL, Redis, Elasticsearch, Node.js background servers.

---

## 2. Product Functional Requirements

### 2.1 DevOps Secret Vault (`apps/devops-secret-vault`)
- **FR-DSV-01: Cryptographic Storage Engine**
  - Storage backed by SQLCipher with AES-256-GCM payload encryption.
  - Master key derivation using Argon2id ($m=64\,\text{MiB}, t=3, p=4$ by default, adaptive scaling).
  - Volatile memory protection utilizing `zeroize` on all decrypted plaintext, keys, and cryptographic buffers upon drop.
- **FR-DSV-02: Native Biometric Integration**
  - Windows Hello (Credential Manager / Biometric Framework FFI).
  - macOS Touch ID / Keychain integration via native platform abstraction.
  - Linux Secret Service / Polkit integration.
  - Mobile BiometricPrompt (Android) and LocalAuthentication/Secure Enclave (iOS).
- **FR-DSV-03: Secret Schemas & Lifecycle Management**
  - First-class support for passwords, SSH key pairs, TLS/SSL certificates, API keys, database connection strings, AWS credentials, Azure credentials, and Terraform state secrets.
  - RFC 6238-compliant TOTP engine generating live time-based tokens with zero external time synchronization.
  - X.509 certificate inspector parsing ASN.1/PEM structures, alerting on upcoming expiry, weak signature algorithms, and key lengths.
- **FR-DSV-04: Security Health & Risk Auditing**
  - Local entropy calculation, password reuse detection, rotation age tracking, and exposed default credential checks.
  - Interactive Health Dashboard calculating aggregate security posture scores completely offline.
- **FR-DSV-05: Backup & Export Bundles**
  - Portable, self-verifying encrypted backup archives using Argon2id + AES-256-GCM with Ed25519 tamper-evident signatures.

### 2.2 AI File Explorer (`apps/ai-file-explorer`)
- **FR-AFE-01: Concurrent Filesystem Indexer**
  - Non-blocking directory crawling capable of cataloging 10,000,000+ files.
  - Live filesystem watching (notify/ReadDirectoryChangesW) updating the index within 50ms of filesystem modifications.
- **FR-AFE-02: Fast Multi-Stage Deduplication**
  - Stage 1: Fast grouping by file size.
  - Stage 2: 4KB header/footer hashing using ultra-fast non-cryptographic or BLAKE3 hashing.
  - Stage 3: Full cryptographic BLAKE3 content hashing for matching candidate pairs.
- **FR-AFE-03: Storage Analytics & Interactive Visualization**
  - Real-time disk usage treemap rendered at 60 FPS via hardware-accelerated canvas/WebGL.
  - Extension, age, and size distribution histograms.
- **FR-AFE-04: Atomic Batch Renamer**
  - Transactional regex/rule-based batch renaming with preview and one-click atomic rollback journal.
- **FR-AFE-05: Local Semantic Search & Offline OCR**
  - Local quantized ONNX/tract embedding inference running on CPU/NPU with sub-50ms query response.
  - Offline Tesseract/OCR engine for images and scanned PDFs.
  - Extraction and indexing for text, PDF, Markdown, DOCX, and source code.

### 2.3 Universal Log Viewer (`apps/universal-log-viewer`)
- **FR-ULV-01: Zero-Copy Memory-Mapped Engine**
  - Instantaneous opening of single or multiple log files up to 500GB using memory-mapped I/O (`memmap2`).
  - Total resident RAM consumption constrained to $< 300\,\text{MB}$.
- **FR-ULV-02: Sparse Line-Offset Indexing**
  - SIMD-accelerated newline scanner building a compact index of line byte offsets.
  - Capable of indexing a 100GB log file in $< 2\,\text{seconds}$.
- **FR-ULV-03: Non-Blocking Real-Time Tail Mode**
  - Asynchronous file follower streaming newly appended lines into the viewport at native write speeds without UI freezing.
- **FR-ULV-04: Sub-50ms Multi-File Search & Filtering**
  - Compiled regex, literal string, and structured log search (JSON, CSV, Syslog, Apache/Nginx, Logfmt).
  - Offline import and normalizer for AWS CloudWatch and Kubernetes log bundles with multi-pod/container correlation.
- **FR-ULV-05: Visual Timeline & Local Anomaly Detection**
  - Interactive log volume and severity histogram.
  - Log template mining (Drain/Spell algorithm variant) grouping repetitive log formats.
  - Anomaly spike detection and failure clustering running 100% locally.
- **FR-ULV-06: Sandboxed Plugin Architecture**
  - Pluggable custom parsers, log highlighters, and export sinks executed in safe, resource-bounded sandboxes.

---

## 3. Strict Performance SLAs

| Metric | DevOps Secret Vault | AI File Explorer | Universal Log Viewer | SLA Gate |
| :--- | :--- | :--- | :--- | :--- |
| **Cold Startup Time** | $< 1,500\,\text{ms}$ | $< 2,000\,\text{ms}$ | $< 1,500\,\text{ms}$ | **$< 2,000\,\text{ms}$ Max** |
| **Search Query Latency** | $< 30\,\text{ms}$ (100k items) | $< 50\,\text{ms}$ (10M items) | $< 50\,\text{ms}$ (100GB file) | **$< 50\,\text{ms}$ Max** |
| **Peak Resident RAM** | $< 120\,\text{MB}$ | $< 280\,\text{MB}$ | $< 250\,\text{MB}$ (500GB log) | **$< 300\,\text{MB}$ Max** |
| **Idle CPU Utilization**| $< 0.5\%$ | $< 1.0\%$ | $< 0.8\%$ | **$< 2.0\%$ Max** |
| **Scan / Index Throughput** | N/A | $> 100,000\,\text{files/s}$ | $> 50\,\text{GB/s}$ | **100GB $< 2\,\text{s}$** |
| **UI Frame Rate** | Constant 60 FPS | Constant 60 FPS | Constant 60 FPS | **Zero dropped frames** |

Any failure to meet these SLAs during automated CI Criterion benchmarks will cause an immediate build abort.

---

## 4. Platform & Target Matrix

1. **Tier 1 (Launch Priority):**
   - Windows x64: `x86_64-pc-windows-msvc`
2. **Tier 2 (Phase 7 Expansion):**
   - Windows ARM64: `aarch64-pc-windows-msvc`
   - Linux x64: `x86_64-unknown-linux-gnu`
3. **Tier 3 (Phase 7 Expansion):**
   - Linux ARM64: `aarch64-unknown-linux-gnu`
   - macOS Intel: `x86_64-apple-darwin`
   - macOS Apple Silicon: `aarch64-apple-darwin`
4. **Tier 4 (Phase 8 Mobile Vault Expansion):**
   - Android: `aarch64-linux-android`
   - iOS: `aarch64-apple-ios`

---

## 5. Non-Functional & Governance Requirements

- **Code Quality:** `#![forbid(unsafe_code)]` enforced globally in shared and application crates (ADR approval required for specific low-level mmap/OS FFI wrappers).
- **Test Coverage:** $\ge 90\%$ line and branch coverage across all shared libraries and application cores.
- **Linting & Formatting:** 100% clean `cargo fmt` and `cargo clippy -- -D warnings`.
- **Supply Chain Security:** Zero vulnerabilities allowed via `cargo audit` and `cargo deny check`.
- **Packaging:** Automated packaging into Windows MSI/EXE, Linux AppImage/DEB/RPM, and macOS DMG/tarball with SHA-256 checksums and CycloneDX/SPDX SBOMs.
