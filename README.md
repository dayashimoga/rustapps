# Offline Commercial Application Monorepo (`rustapps`)

[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org)
[![Svelte 5](https://img.shields.io/badge/frontend-Svelte%205%20%2B%20TypeScript-red.svg)](https://svelte.dev)
[![Architecture](https://img.shields.io/badge/architecture-100%25%20Offline%20Air--Gapped-blue.svg)](file:///h:/rustapps/AGENTS.md)
[![Verification](https://img.shields.io/badge/verification-Zero--Mock%20Certified-green.svg)](file:///h:/rustapps/AGENTS.md)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-lightgrey.svg)](file:///h:/rustapps/LICENSE)

An enterprise-grade monorepo containing **12 standalone desktop applications** and **13 shared modular libraries** engineered in Rust and Svelte 5. Every application operates with **100% offline autonomy, zero unsolicited network egress, and zero mock/simulation fallbacks in production**.

---

## 🏛️ Core Architectural Invariants (`AGENTS.md`)

All crates and applications strictly adhere to the mandatory engineering laws defined in [`AGENTS.md`](file:///h:/rustapps/AGENTS.md):

1. **Zero-Mock, Evidence-Based Production Readiness:**
   No fake metrics, hardcoded buffers, synthetic character generators, or mock storage adapters are permitted in release code paths. All test doubles are strictly isolated behind `#[cfg(test)]`.
2. **Standalone Application Independence:**
   All 12 applications compile, run, test, package, and release independently. No application crate may depend on another application crate.
3. **100% Offline Autonomy & Zero Unsolicited Egress:**
   Every capability—cryptography, search, OCR, neural machine translation, ASR speech recognition, audio/video compositing, and protocol decoding—operates completely offline. Network telemetry and cloud beacons are strictly forbidden.
4. **Mandatory Embedded Desktop GUI & `--self-test`:**
   Every desktop binary embeds its compiled Svelte 5 frontend bundle (`ui/dist/index.html`, `ui/dist/assets/app.js`, `ui/dist/assets/app.css`) directly into the Rust executable, launches an embedded loopback GUI host, and must pass `<binary> --self-test` with a 100% IPC verification pass rate before release.
5. **Memory & Cryptographic Hygiene:**
   `#![forbid(unsafe_code)]` enforced across domain crates. Plaintext secrets, master keys, and tokens implement `ZeroizeOnDrop` or are wrapped in `zeroize::Zeroizing<T>`. All PRNG nonces use `rand::rngs::OsRng` exclusively.

---

## 📦 Monorepo Applications Portfolio (12 Standalone Apps)

### Flagship Tier (Phases 1 – 3)

| Application | Domain | Key Architecture & Primitives | Self-Test IPC | Port |
| :--- | :--- | :--- | :--- | :--- |
| **[AI File Explorer](file:///h:/rustapps/apps/ai-file-explorer)** | Systems / File Mgmt | Tantivy indexing, BLAKE3 deduplication, squarified treemaps, dense vector embeddings, offline OCR | 9 / 9 PASSED | `1420` |
| **[DevOps Secret Vault](file:///h:/rustapps/apps/devops-secret-vault)** | Security / DevOps | AES-256-GCM envelope encryption, Argon2id KDF, TOTP, Shamir $k$-of-$n$ key sharing, offline leak scanner | 11 / 11 PASSED | `1421` |
| **[Universal Log Viewer](file:///h:/rustapps/apps/universal-log-viewer)** | Observability / Big Data | `memmap2` zero-copy stream, Drain AI template clustering, PII redaction, OpenTelemetry trace waterfall | 11 / 11 PASSED | `1422` |
| **[Universal Live Translator](file:///h:/rustapps/apps/universal-live-translator)** | AI / NMT / Speech | Offline Marian NMT, n-gram LID (<500μs), formants TTS synthesis, ASR mel-spectrogram extraction | 9 / 9 PASSED | `1423` |
| **[Pro Audio & Video Editor](file:///h:/rustapps/apps/pro-av-editor)** | Creative / Media | Multi-track timeline compositor, SMPTE timecode, 3D LUTs, EBU R128 loudness, cross-correlation audio sync | 12 / 12 PASSED | `1424` |

### Next-Generation Innovation Expansion Suite (Phases 4.1 – 4.7)

| Application | Domain | Key Architecture & Primitives | Self-Test IPC | Port |
| :--- | :--- | :--- | :--- | :--- |
| **[Personal Knowledge Graph](file:///h:/rustapps/apps/personal-knowledge-graph)** | Productivity / AI | Bi-directional wikilinks, BFS graph traversal, BM25 + dense semantic vector search, SQLite WAL | 5 / 5 PASSED | `1425` |
| **[Air-Gapped API Protocol Studio](file:///h:/rustapps/apps/api-protocol-studio)** | DevTools / SecOps | Offline HTTP/gRPC mocking & security fuzzing (SQLi/XSS/Command Injection), variable interpolation | 5 / 5 PASSED | `1426` |
| **[Parquet Analytics Workbench](file:///h:/rustapps/apps/parquet-analytics-workbench)** | Data Science | Columnar scanning, SQL querying via DataFusion, group-by aggregations, column statistics | 5 / 5 PASSED | `1427` |
| **[Offline DFIR Triage Kit](file:///h:/rustapps/apps/dfir-triage-kit)** | Cybersecurity | Pure mathematical Shannon entropy, process tree analysis, socket enumeration, file integrity verification | 5 / 5 PASSED | `1428` |
| **[Offline AI Screen Recorder Studio](file:///h:/rustapps/apps/screen-recorder-studio)** | Media / EdTech | Auto-zoom keyframe generation, dead-air silence removal, tutorial step extraction, interactive HTML export | 5 / 5 PASSED | `1429` |
| **[Scientific Signal Oscilloscope](file:///h:/rustapps/apps/signal-telemetry-oscilloscope)** | Scientific / Hardware | Radix-2 Cooley-Tukey FFT spectrum analyzer, Hann windowing, zero-crossing frequency, UART 8N1 decoding | 5 / 5 PASSED | `1430` |
| **[Audio Mastering Studio](file:///h:/rustapps/apps/audio-mastering-studio)** | Media / Audio | 4-stem demixing (Vocals, Drums, Bass, Other), 5-band biquad parametric EQ, dynamics, EBU R128 | 5 / 5 PASSED | `1431` |

---

## 🛠️ Shared Modular Crates (`shared/*` & `platform`)

- **`platform` (`platform-common`):** Cross-platform desktop loopback host, embedded UI bundle loader, and `--self-test` verification runner.
- **`shared/licensing`:** Offline Ed25519 signature verification, commercial tier feature gating (Free, Pro, Enterprise), and offline `.lic` certificate management.
- **`shared/encryption`:** AES-256-GCM AEAD, Argon2id key derivation, CSPRNG nonces, and zeroize-on-drop memory wrappers.
- **`shared/storage`:** SQLite WAL connection pooling, transaction helpers, and migration management.
- **`shared/search`:** Tantivy full-text search indexing, schema definitions, and tokenization.
- **`shared/indexing`:** Sparse line indexers and binary search primitives for fast memory-mapped file seeking.
- **`shared/media-core`:** Raw audio/video buffers, pixel format converters, and color math.
- **`shared/ai-core`:** Offline vector cosine similarity, embeddings index, and machine learning primitives.
- **`shared/telemetry`:** In-memory performance metrics, operation counters, and latency histogram timers.
- **`shared/logging`:** Structured local file logger (`tracing` with rolling daily log files); zero remote egress.
- **`shared/settings`:** Offline JSON configuration persistence.
- **`shared/updater`:** Offline cryptographic update verification.
- **`shared/plugins`:** Sandboxed IPC plugin host.
- **`shared/model-hub`:** Cryptographically verified offline AI model package loader (SHA-256 + Ed25519).

---

## 🚀 Quick Start & Developer Guide

### 1. Prerequisites
- **Rust:** `stable` (1.80+ recommended)
- **Node.js:** `20.x` or later (for compiling Svelte 5 frontend bundles)
- **Cargo Tools:** `rustfmt`, `clippy`

### 2. Running Any Application in Desktop GUI Mode
Each application embeds its Svelte 5 frontend bundle. To run any app:
```powershell
# 1. Build the app's Svelte 5 UI bundle
npm --prefix apps/devops-secret-vault/ui install
npm --prefix apps/devops-secret-vault/ui run build

# 2. Launch the desktop GUI
cargo run -p devops-secret-vault
```

### 3. Running Live IPC `--self-test` Verification
Every application binary supports a standalone headless `--self-test` mode that verifies embedded asset sizes and executes live IPC roundtrips:
```powershell
cargo run -p devops-secret-vault -- --self-test
cargo run -p ai-file-explorer -- --self-test
cargo run -p signal-telemetry-oscilloscope -- --self-test
cargo run -p audio-mastering-studio -- --self-test
```

### 4. Running the Verification Test Suite
```powershell
# Run all unit tests across the monorepo
cargo test --workspace --all-targets

# Run air-gap network egress and integration tests
cargo test -p integration-tests

# Run Criterion performance benchmarks (56 benchmark cases)
cargo test -p benchmarks --benches -- --test
```

### 5. Formatting & Linting
```powershell
# Check code formatting (must be 100% compliant)
cargo fmt --all -- --check

# Check clippy warnings (zero warnings enforced)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 6. Packaging Release Artifacts (Windows)
```powershell
# Package a single application
powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1 -AppName audio-mastering-studio

# Package all 12 applications
powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1
```
Output artifacts are saved to `dist/windows`:
- Standalone `.exe`
- Portable `.zip`
- WiX MSI installer template `.wxs`
- CycloneDX 1.5 JSON Software Bill of Materials (SBOM)
- Cryptographic `.sha256` checksums for every artifact

---

## 📊 Verification & SLA Standards

| Metric | Universal Log Viewer | DevOps Secret Vault | AI File Explorer | Universal Live Translator | Pro Audio & Video Editor |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Cold Startup** | $< 1,200\text{ ms}$ | $< 1,200\text{ ms}$ | $< 1,500\text{ ms}$ | $< 1,500\text{ ms}$ | $< 2,000\text{ ms}$ |
| **Primary Op Latency** | $< 50\text{ ms}$ (100GB search) | $< 20\text{ ms}$ (100k items) | $< 35\text{ ms}$ (10M files) | $< 150\text{ ms}$ text / $< 500\mu\text{s}$ LID | $< 16.6\text{ ms}$ frame render (60 FPS) |
| **Base Idle RSS RAM** | $< 180\text{ MB}$ | $< 100\text{ MB}$ | $< 220\text{ MB}$ | $< 250\text{ MB}$ | $< 350\text{ MB}$ |
| **Idle CPU Usage** | $< 0.5\%$ | $< 0.2\%$ | $< 0.5\%$ | $< 0.5\%$ | $< 0.8\%$ |
| **Network Egress** | **0 bytes (100% offline)** | **0 bytes (100% offline)** | **0 bytes (100% offline)** | **0 bytes (100% offline)** | **0 bytes (100% offline)** |

---

## 📜 Documentation Index

- [`AGENTS.md`](file:///h:/rustapps/AGENTS.md): Global Engineering Contract & Behavioral Execution Protocol.
- [`MASTER_ENGINEERING_BLUEPRINT.md`](file:///h:/rustapps/MASTER_ENGINEERING_BLUEPRINT.md): Complete architectural blueprint, audit review, monetization tiers, and innovation portfolio roadmap.
- [`PROJECT_WALKTHROUGH.md`](file:///h:/rustapps/PROJECT_WALKTHROUGH.md): Technical walkthrough of the 5 flagship applications, platform host, and expansion suite.
- [`FINAL_SYSTEM_STATUS_REPORT.md`](file:///h:/rustapps/FINAL_SYSTEM_STATUS_REPORT.md): Monorepo-wide test counts, benchmark metrics, and packaging verification manifest.
