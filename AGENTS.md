# Global Engineering Contract & Workspace Rules (AGENTS.md)

**Scope:** Repository-wide (`H:/rustapps`)  
**Enforcement Level:** Mandatory, Non-Negotiable (`ALWAYS_ON`)

---

## 1. Core Engineering Philosophy: Zero-Mock, Evidence-Based Production Readiness

1. **Absolute Prohibition of Mocks/Simulations in Production Code:**
   - Never implement or retain fake/simulated logic presented as real functionality (e.g., returning hardcoded `64 MB` RSS or `0.5%` CPU, wrapping `InMemoryKeyring` inside OS keyring structs, copying compressed PNG/JPEG bytes into grayscale buffers without decoding, or generating synthetic character sequences in OCR).
   - In-memory or mock adapters (`InMemoryKeyring`, `MockBiometricProvider`, UI preview data) MUST be gated behind `#[cfg(test)]` or explicit `feature = "mock-preview"` flags, never active as silent fallbacks in production builds.
2. **Standalone Application Independence:**
   - All five flagship applications (`ai-file-explorer`, `devops-secret-vault`, `universal-log-viewer`, `universal-live-translator`, `pro-av-editor`) MUST compile, run, test, package, and release independently.
   - No application crate may depend on another application crate. Shared capabilities MUST reside in opt-in, modular `shared/*` or `platform` crates.
3. **100% Offline Autonomy & Zero Unsolicited Egress:**
   - All core runtime features (search, cryptography, OCR, ASR, NMT translation, TTS, audio/video DSP, AI auto-editing) MUST operate 100% offline.
   - Model downloading (for `universal-live-translator`, `ai-file-explorer`, `pro-av-editor`) MUST be isolated in a dedicated, user-initiated, cryptographically verified download manager (`shared/model-hub`) verifying SHA-256 + Ed25519 signatures before mounting models.
   - Zero telemetry, analytics, or crash reports may ever be transmitted over the network.
4. **Mandatory Embedded Desktop GUI & Self-Test Verification:**
   - Every desktop application binary (`apps/*/src/main.rs`) MUST embed its compiled Svelte 5 frontend bundle (`../ui/dist/index.html`, `../ui/dist/assets/app.js`, `../ui/dist/assets/app.css`), start the `platform_common::run_desktop_app` loopback GUI host, and route all UI commands through `commands::dispatch_ipc_command`.
   - Console CLI stubs that print a banner and exit without launching a GUI window are strictly prohibited as default entrypoints (CLI diagnostic modes must be gated behind `--cli`).
   - Every built desktop binary MUST support and pass `<binary> --self-test`, verifying embedded asset sizes and executing live IPC commands against the Rust backend before any release artifact is packaged.

---

## 2. Target Platform Parity Matrix

Every application and shared crate must maintain maximum feature parity across all 5 target operating systems:

| Platform | Architectures | Desktop / Mobile Shell | Native Subsystem Integrations | Distribution Formats |
| :--- | :--- | :--- | :--- | :--- |
| **Windows** | `x86_64`, `aarch64` | Tauri v2 + Svelte 5 (TS) | Win32 Credential Manager, Windows Hello (`windows-sys`), WASAPI, DirectML / AVX2 | `.msi` (WiX), `.exe` (NSIS), Portable `.zip` |
| **Linux** | `x86_64`, `aarch64` | Tauri v2 + Svelte 5 (TS) | Secret Service D-Bus (`zbus`), Polkit, PipeWire/ALSA, `/proc` metrics, Vulkan/VAAPI | `.AppImage`, `.deb`, `.rpm`, `.tar.gz` |
| **macOS** | `x86_64`, `aarch64` (Universal) | Tauri v2 + Svelte 5 (TS) | Keychain Services, LocalAuthentication (Touch ID), CoreAudio, Metal / VideoToolbox | Universal `.dmg`, Notarized `.app.tar.gz` |
| **Android** | `arm64-v8a`, `x86_64` | Flutter 3.x + Rust C-ABI / JNI (`flutter_rust_bridge`) | Android Keystore StrongBox, `BiometricPrompt`, Oboe/AAudio, NNAPI / Vulkan | Signed `.apk`, Play Store `.aab` |
| **iOS** | `arm64`, `sim-arm64` | Flutter 3.x + Rust C-ABI (`XCFramework`) | Secure Enclave Keychain, Face ID / Touch ID, AVAudioEngine, CoreML / Metal | Signed `.ipa`, `.xcframework` |

---

## 3. Coding Standards & Safety Invariants

1. **Rust Safety & Linting:**
   - `#![forbid(unsafe_code)]` is required in all application domain crates and pure-Rust shared crates.
   - Where `unsafe` is strictly required for OS FFI, `memmap2`, hardware audio/video buffers, or SIMD intrinsics (`platform`, `shared/storage`, `shared/media-core`, `ffi.rs`), it MUST be isolated in dedicated submodules with `// SAFETY:` proofs and `cargo miri` / sanitizer validation where supported.
   - Every workspace crate must pass `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` with zero warnings.
2. **Error Handling:**
   - Library crates (`shared/*`, `platform`, app core libs) MUST use typed errors via `thiserror::Error`.
   - `unwrap()` and `expect()` are forbidden in production code paths (enforced via `clippy::unwrap_used` and `clippy::expect_used` lints); use `?`, `.ok_or_else()`, or graceful fallback handling.
3. **Cryptographic & Memory Hygiene:**
   - All keys, passphrases, plaintext secrets, and biometric tokens MUST be wrapped in `zeroize::Zeroizing<T>` or implement `ZeroizeOnDrop`.
   - Database encryption at rest MUST encrypt both secret payloads and sensitive metadata fields (using SQLCipher or authenticated envelope encryption per row/table).
   - Never use non-cryptographic PRNGs for salts, nonces, or IDs; use `rand::rngs::OsRng` exclusively.

---

## 4. Performance SLAs & Resource Budgets

| Metric | Universal Log Viewer | DevOps Secret Vault | AI File Explorer | Universal Live Translator | Pro Audio & Video Editor |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Cold Startup** | $< 1,200\text{ ms}$ | $< 1,200\text{ ms}$ | $< 1,500\text{ ms}$ | $< 1,500\text{ ms}$ | $< 2,000\text{ ms}$ |
| **Primary Query / Op** | $< 50\text{ ms}$ (100GB search) | $< 20\text{ ms}$ (100k items) | $< 35\text{ ms}$ (10M files) | $< 150\text{ ms}$ text / $< 400\text{ ms}$ streaming ASR | $< 16.6\text{ ms}$ frame render (60 FPS) |
| **Base Idle RSS RAM** | $< 180\text{ MB}$ | $< 100\text{ MB}$ | $< 220\text{ MB}$ | $< 250\text{ MB}$ (+ active model) | $< 350\text{ MB}$ (+ frame cache) |
| **Idle CPU Usage** | $< 0.5\%$ | $< 0.2\%$ | $< 0.5\%$ | $< 0.5\%$ | $< 0.8\%$ |

---

## 5. Verification, Testing & Release Gates

Every stage, phase, and subphase MUST provide verifiable evidence before completion:
1. **Unit & Integration Tests:** `cargo test --workspace --all-targets` (100% pass rate, $>85\%$ line coverage on core logic).
2. **Frontend & Mobile Tests:** `npm run typecheck`, `npm run test:unit` (Vitest), `npm run test:e2e` (Playwright), and `flutter test` for mobile targets.
3. **Security & Supply Chain:** `cargo deny check` (licenses, bans, advisories, sources) and `cargo audit` with 0 vulnerabilities.
4. **Performance Benchmarks:** `cargo bench` via Criterion; any $>5\%$ regression ($p < 0.05$) blocks completion.
5. **Artifact & Self-Test Verification:** Every application must build its Svelte 5 UI (`npm run build`) before `cargo build --release`, pass `<binary> --self-test` with 100% IPC check pass rate, and package into a complete GUI executable + portable `.zip` / installer / mobile bundle with SHA-256 checksum + CycloneDX SBOM.

