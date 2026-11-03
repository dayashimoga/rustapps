# Code Understanding & System Architecture Guide

**Document Version:** 2.0.0  
**Purpose:** Living architectural map providing transparent understanding of codebase organization, data flows, operational boundaries, and system invariants across all 5 flagship applications and 14 shared crates.

---

## 1. High-Level Architecture

The monorepo employs a clean hexagonal / ports-and-adapters architecture implemented in Rust stable. The architecture cleanly separates presentation (Tauri v2 desktop with Svelte 5, Flutter 3.x mobile) from domain business logic (application core libraries) and infrastructure primitives (shared reusable crates).

```text
[ Presentation Layer: Tauri v2 / Svelte 5 (Desktop) & Flutter 3.x (Mobile) ]
                                |
                                v  (Tauri IPC / C-ABI FFI Bridge)
[ Application Domain: ulv_core, vault_core, afe_core, ult_core, pave_core ]
                                |
                                v  (In-Process Modular Rust Calls)
[ Shared Crates: storage, encryption, search, indexing, licensing, model-hub, ai-core, media-core... ]
                                |
                                v
[ System Level: Win32/WASAPI/DirectML, Linux PipeWire/SecretService, macOS CoreAudio/Metal, Android, iOS ]
```

---

## 2. Monorepo Folders & Layout

- `/platform`: Native OS integration adapters, platform capability detection, hardware resource meters, and standard file path resolvers (`AppPaths`).
- `/shared/logging`: Centralized tracing initialization, non-blocking rolling file appenders, NDJSON and ANSI console formatting.
- `/shared/storage`: SQLite connection pools in WAL mode, migration framework, and safe atomic filesystem wrappers.
- `/shared/encryption`: AES-256-GCM symmetric cipher, Argon2id KDF, memory zeroization (`zeroize::Zeroizing`), Ed25519 digital signatures, and biometric abstraction.
- `/shared/search`: Embedded Tantivy full-text search engine, schema definitions, sub-50ms query executor, and hit snippet generator.
- `/shared/indexing`: High-throughput parallel directory traversal, chunking pipelines, backpressure channels, and cancellation watch tokens.
- `/shared/settings`: Schema-validated persistent configuration engine with event bus notifications.
- `/shared/telemetry`: Strictly opt-in local performance metrics (gauges, counters, histograms); zero network connectivity.
- `/shared/updater`: Cryptographically signed offline binary update verifier and atomic staged installer with rollback.
- `/shared/plugins`: Sandboxed plugin host, capability manifest parser, and bounded lifecycle runner.
- `/shared/licensing`: Ed25519 offline cryptographic licensing engine with hardware fingerprinting and feature gating.
- `/shared/model-hub`: Cryptographically verified offline AI model package manager verifying SHA-256 and Ed25519 signatures.
- `/shared/ai-core`: Pure-Rust offline AI foundation primitives: audio DSP (Mel-spectrogram, VAD), text tokenizer, dense quantized embeddings, image decoders, and template OCR.
- `/shared/media-core`: Multi-track linear PCM audio buffers, color space conversions (sRGB, Rec.709, Rec.2020), 3D `.cube` LUT parsers, and broadcast video scopes.
- `/apps/universal-log-viewer`: Universal Log Viewer application (desktop Tauri v2 / Svelte 5 + Flutter mobile).
- `/apps/devops-secret-vault`: DevOps Secret Vault application (desktop Tauri v2 / Svelte 5 + Flutter mobile).
- `/apps/ai-file-explorer`: AI File Explorer application (desktop Tauri v2 / Svelte 5 + Flutter mobile).
- `/apps/universal-live-translator`: Universal Live Translator application (desktop Tauri v2 / Svelte 5 + Flutter mobile).
- `/apps/pro-av-editor`: Professional Audio & Video Editor application (desktop Tauri v2 / Svelte 5 + Flutter mobile).
- `/docs`: Architecture, security, performance, and governance specifications.
- `/scripts`: Automation scripts for builds, packaging, testing, and SBOM generation across all 5 platforms.
- `/tests`: Cross-crate integration tests verifying end-to-end multi-app workflows.
- `/benchmarks`: Criterion performance benchmark suites tracking sub-50ms and 60 FPS SLAs.
- `/.github/workflows`: Independent per-application release workflows and continuous integration matrices.

---

## 3. Module Responsibilities & Crate Boundaries

1. **`shared-logging`:** Structured diagnostics via `tracing` with non-blocking rolling file appenders.
2. **`shared-storage`:** Owns SQLite database interaction. Forces `PRAGMA journal_mode = WAL;` and `PRAGMA synchronous = NORMAL;`. Idempotent schema migrations and atomic staging files.
3. **`shared-encryption`:** Authenticated AES-256-GCM encryption, Argon2id KDF, Shamir secret sharing, and memory zeroization.
4. **`shared-search`:** In-process Tantivy full-text indexing, schema builders, and sub-50ms query execution.
5. **`shared-indexing`:** Asynchronous filesystem crawling using thread-pool work stealing and bounded mpsc streaming.
6. **`shared-settings`:** Validates and stores app configuration with JSON/TOML serialization and change notifications.
7. **`shared-telemetry`:** Tracks local execution metrics. Inert unless explicitly enabled by user.
8. **`shared-updater`:** Validates Ed25519 signatures on update bundles and swaps executables atomically.
9. **`shared-plugins`:** Dispatches events to registered plugins inside bounded memory and CPU quotas.
10. **`shared-licensing`:** Offline cryptographic tier verification (Free, Pro, Business) using Ed25519 signatures and hardware fingerprinting.
11. **`shared-model-hub`:** Validates, mounts, and tracks offline neural weights and model bundles without network access.
12. **`shared-ai-core`:** Foundational offline algorithms: Mel filterbank extraction, VAD energy detection, token hashing, quantized embeddings, Otsu binarization.
13. **`shared-media-core`:** Audio buffer mixing, constant-power panning, color grading math, 3D LUT trilinear interpolation, and scope matrices.
14. **`platform-common`:** Native OS subsystem abstraction layer: `AppPaths` directory resolver, `ThemeDetector`, `PlatformKeyring` (Windows Hello, Touch ID, Linux Secret Service), `PowerTelemetry`, and resource usage monitors.

### Application Crates

15. **`universal-log-viewer` (`ulv_core` in `apps/universal-log-viewer`):**
    - `mmap_engine`: Zero-copy memory mapping (`memmap2`) with reload-on-growth.
    - `sparse_index`: SIMD newline scanner building $K=128$ sparse checkpoints for sub-microsecond line seeking.
    - `tail_mode`: Non-blocking real-time file follower streaming newly appended lines.
    - `search_engine`: Literal, case-insensitive, and compiled regex search (<50ms SLA).
    - `redaction`: Real-time PII masker for emails, credit cards, IPv4/IPv6, SSNs, and JWT tokens.
    - `local_ai`: 100% offline root-cause analysis via the Drain prefix-tree template miner and anomaly spike detection.
    - `ffi`: C-ABI endpoints (`ulv_*`) powering the Flutter mobile client.
    - `ui`: Svelte 5 frontend with 60 FPS virtualized viewport, timeline histogram, and AI drawer.

16. **`devops-secret-vault` (`vault_core` in `apps/devops-secret-vault`):**
    - `crypto`: Argon2id KDF, AES-256-GCM metadata envelope encryption, and memory zeroization.
    - `shamir`: Pure-Rust $GF(2^8)$ Shamir Secret Sharing ($(k, n)$ threshold scheme).
    - `storage`: SQLite WAL storage with chained SHA-256 audit logging.
    - `totp`: RFC 6238 real-time TOTP generator with Base32 decoding and sub-microsecond token computation.
    - `cert_monitor`: X.509 ASN.1 certificate parser extracting validity windows and weak ciphers.
    - `risk_scoring`: 5-dimensional security posture scoring (entropy, reuse, rotation age, default dictionary, cert hygiene).
    - `ffi`: C-ABI endpoints (`vault_*`) powering the Flutter mobile client.
    - `ui`: Svelte 5 desktop manager with glassmorphic styling, Shamir modal, and health dashboard.

17. **`ai-file-explorer` (`apps/ai-file-explorer`):**
    - `crawler` & `watcher`: Asynchronous crawler and OS kernel recursive filesystem event watcher.
    - `dedup`: 3-stage duplicate detector: size grouping $\rightarrow$ 4KB header/footer hashing $\rightarrow$ full BLAKE3 verification.
    - `ocr`: Pure-Rust image decoding (BMP, PNG, JPEG) and template OCR engine.
    - `screenshot`: 64-bit gradient perceptual difference hashing (`dHash`) and Hamming distance search.
    - `embeddings`: 8-bit quantized dense semantic vector inference (`[i8; 64]`) and cosine vector index.
    - `compliance`: PII and PCI scanner detecting sensitive numbers, credit cards (Luhn algorithm), and private keys.
    - `ffi`: C-ABI endpoints (`afe_*`) powering the Flutter mobile browser.
    - `ui`: Svelte 5 desktop explorer with squarified treemap, compliance drawer, and search modals.

18. **`universal-live-translator` (`ult_core` in `apps/universal-live-translator`):**
    - `lid`: Sub-500µs n-gram Language Identification supporting 24 languages across Latin, Cyrillic, Greek, Arabic, Hebrew, CJK, Indic, and Thai scripts.
    - `nmt`: Sub-150ms offline Neural Machine Translation with glossary overrides and auto-pivot routing.
    - `speech`: Sub-400ms streaming ASR with 16kHz energy-based VAD and 80-channel Mel-spectrogram processing.
    - `tts`: Offline formant synthesizer producing 22.05kHz linear PCM audio in standard WAV format.
    - `camera_translate`: Bounding-box OCR document translation with text overlay.
    - `subtitles`: SubRip (`.srt`) and WebVTT (`.vtt`) timecoded subtitle translators.
    - `phrasebook`: SQLite WAL persistent phrasebook and translation audit log.
    - `ffi`: C-ABI endpoints (`ult_*`) powering the Flutter mobile translation studio.
    - `ui`: Svelte 5 desktop studio with speech conversation view, camera view, and phrasebook drawer.

19. **`pro-av-editor` (`pave_core` in `apps/pro-av-editor`):**
    - `models` & `timeline`: Non-destructive multi-track timeline DAG (Video, Audio, Subtitle, Adjustment), SMPTE timecode (`HH:MM:SS:FF`), keyframe curves, transactional `CommandHistory`, and `.avedproj` project persistence.
    - `compositor`: $< 16.6\text{ ms}$ (60 FPS) multi-track layer compositor with 8 blend modes and green/blue screen chroma keying with despill.
    - `color`: 3-Way color grading wheels (Lift, Gamma, Gain), Adobe 3D `.cube` LUT parser, and real-time broadcast video scopes (RGB histogram, luma waveform, vectorscope).
    - `audio_engine`: Multi-track mixer, constant-power pan, 8-band parametric EQ, compressor, EBU R128 loudness meter (LUFS and True Peak), and 16-bit linear PCM WAV exporter.
    - `ai_editor` & `templates`: AI silence ripple-cutter, visual scene-cut detector, beat detector, auto-captions, and CMX 3600 EDL / FCPXML exporters.
    - `ffi`: C-ABI endpoints (`ave_*`) powering the Flutter mobile NLE studio.
    - `ui`: Svelte 5 NLE studio UI with program monitor, multi-track timeline, color wheels, audio mixer, and AI tools modal.

---

## 4. End-to-End Data Flows

### 4.1 Vault Secret Retrieval Flow
```text
User Request (UI) 
  --> Tauri IPC command: get_secret(id)
  --> Application core checks biometric authorization / master key presence
  --> shared-storage retrieves ciphertext from SQLite table
  --> shared-encryption decrypts envelope using AES-256-GCM into ZeroizingBytes
  --> Zeroized buffer returned to memory boundary
  --> Transferred via secure IPC to UI or copied to secure clipboard with auto-clear timer
```

### 4.2 Vault Shamir Secret Sharing Flow
```text
User creates distributed split (k=3, n=5)
  --> User provides secret passphrase
  --> vault_core generates random polynomial coefficients in GF(2^8)
  --> Evaluates shares (x, P(x)) for x in 1..n
  --> Emits shares with Base32 encoding and BLAKE3 integrity tag
Recovery:
  --> User enters k distinct shares
  --> Lagrange polynomial interpolation computes secret P(0) in GF(2^8)
  --> Reconstitutes master key in zeroized memory
```

### 4.3 Log Viewer Large File Inspection & PII Redaction Flow
```text
User opens 100GB log file
  --> Universal Log Viewer requests mmap handle via shared-storage/mmap engine
  --> Zero-copy memory map initialized via OS virtual memory
  --> SIMD line scanner builds sparse index of newline offsets (100GB < 2s)
  --> Virtualized Svelte UI requests slice: lines 10,000 to 10,050
  --> Rust core resolves line byte offsets from sparse index in O(1) time
  --> PII redaction engine masks sensitive patterns (credit cards, emails, IP addresses)
  --> Cleaned byte slice converted to UTF-8 lossy and returned via IPC in < 1ms
```

### 4.4 AI File Explorer Compliance Scanning Flow
```text
User triggers compliance scan on folder
  --> FileExplorerService queues document extractor across files
  --> Document text extracted from PDF, DOCX, Markdown, or Code
  --> Compliance scanner executes regex detectors:
        - SSN (xxx-xx-xxxx)
        - Credit cards (Luhn mod-10 algorithmic checksum verification)
        - Email addresses & private key headers
  --> Findings aggregated into ComplianceReport with tier gating (Pro vs Business)
  --> ComplianceScanner.svelte renders interactive risk breakdown
```

### 4.5 Universal Live Translator Real-Time Speech Translation Flow
```text
Microphone audio stream (16kHz PCM)
  --> VAD engine evaluates energy thresholds (30dB above noise floor)
  --> Mel-spectrogram feature extractor builds 80-channel spectral frames
  --> Streaming ASR synthesizes text candidate (<400ms SLA)
  --> LID engine detects spoken language (<500µs SLA)
  --> Offline NMT engine applies glossary rules and routes translation (<150ms SLA)
  --> Formant TTS engine generates synthesized speech WAV at 22.05kHz
  --> Audio output emitted to speaker while UI displays dual-language conversation bubbles
```

### 4.6 Pro AV Editor 60 FPS Compositor & Video Scopes Flow
```text
Timeline Playhead moves to timestamp T
  --> EditorService evaluates active clips across all tracks at timestamp T
  --> Compositor resolves keyframed transforms (position, scale, rotation, opacity)
  --> Decodes source frames and applies Chroma Key removal / despill suppression
  --> Composites layers from bottom to top using selected blend modes (<16.6ms)
  --> 3-Way color grading applies Lift, Gamma, Gain math and 3D LUT interpolation
  --> Video scope engine computes 256-bin RGB histograms, 128x64 waveform, 64x64 vectorscope
  --> Rendered frame and scope matrices streamed to ProgramMonitor.svelte at 60 FPS
```

---

## 5. Security & Air-Gap Guarantees

1. **Absolute Air-Gap Isolation:** Zero networking client libraries (`reqwest`, `hyper`, `curl`, `ureq`) exist in application runtime dependencies. All translation models, OCR dictionaries, and AI engines run 100% locally.
2. **Cryptographic Memory Hygiene:** All decrypted secrets, biometric session tokens, and intermediate keys are stored in `zeroize::Zeroizing<T>` wrappers that overwrite memory with zeros on drop.
3. **Chained Integrity Logs:** Audit logs use cryptographic hash chaining (`prev_hash` + entry data $\rightarrow$ `current_hash`) to ensure tamper detection.
4. **Zero-Mock Enforcement:** Production binaries contain 0 mock adapters or simulated fallbacks. All mock keys and preview stubs are compiled exclusively under `#[cfg(test)]`.

---

## 6. Mobile Architecture Across All 5 Flagship Apps

Each application compiles a standalone C-ABI dynamic library (`cdylib`) and exports strongly typed endpoints consumed by Flutter 3.x via `dart:ffi`:

| Application | C-ABI Prefix | Mobile Service Class | Key Mobile Capabilities |
| :--- | :--- | :--- | :--- |
| **Universal Log Viewer** | `ulv_*` | `UlvService` | File mmap opening, sparse line seeking, tail streaming, PII redaction |
| **DevOps Secret Vault** | `vault_*` | `VaultService` | Biometric unlock, AES-GCM envelope, TOTP generation, Shamir sharing |
| **AI File Explorer** | `afe_*` | `AfeService` | Directory crawling, 3-stage dedup, OCR scanning, compliance report |
| **Universal Live Translator** | `ult_*` | `UltService` | Real-time speech translation, camera OCR overlay, offline phrasebook |
| **Pro Audio & Video Editor** | `ave_*` | `AveService` | Multi-track timeline preview, audio mixing, AI silence cutter, EDL export |
