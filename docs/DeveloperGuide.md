# Monorepo Developer & Contributor Guide

**Document Version:** 2.0.0  
**Status:** Approved  
**Classification:** Internal Engineering Standard  

---

## 1. Development Prerequisites & Environment Setup

To build, test, and contribute to the monorepo across all 5 flagship applications, configure the following tools on your workstation:

### Required Toolchains
- **Rust Toolchain:** Rust Stable ($\ge 1.80.0$). Install via [rustup](https://rustup.rs/):
  ```bash
  rustup default stable
  rustup component add rustfmt clippy llvm-tools-preview
  ```
- **Node.js & Package Manager:** Node.js LTS ($\ge 20.x$) and npm ($\ge 10.x$):
  ```bash
  node --version
  npm --version
  ```
- **Flutter SDK (for Mobile Clients):** Flutter ($\ge 3.24.x$) with Dart SDK.
- **Platform Native Build Tools:**
  - **Windows:** Visual Studio 2022 C++ Build Tools (MSVC), Windows 10/11 SDK, WiX Toolset v3/v4.
  - **Linux:** `build-essential`, `libssl-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `libasound2-dev`.
  - **macOS:** Xcode Command Line Tools.

---

## 2. Monorepo Quickstart

```bash
# Clone the repository
git clone https://github.com/organization/offline-apps-monorepo.git
cd offline-apps-monorepo

# Install root JavaScript/TypeScript dependencies
npm install

# Build all 19 shared crates and application libraries
cargo build --workspace

# Run all unit and integration tests across the workspace
cargo test --workspace --all-targets

# Run all frontend Vitest unit tests
npm run test:unit

# Typecheck all TypeScript codebases
npm run typecheck

# Check code formatting and linter diagnostics
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## 3. Standard Development Workflows

### 3.1 Cargo Commands
- **Check Compilation:** `cargo check --workspace --all-targets`
- **Run Fast Workspace Tests:** `cargo test --workspace`
- **Run Targeted Benchmark:** `cargo bench -p benchmarks`
- **Security & License Audit:**
  ```bash
  cargo audit
  cargo deny check
  ```

### 3.2 Frontend & Mobile Commands
- **Run Vitest Unit Tests:** `npm run test:unit`
- **Typecheck Frontend (Strict TypeScript):** `npm run typecheck`
- **Build All Desktop UI Bundles:** `npm run build`

### 3.3 Running Desktop Applications Locally
```bash
# Universal Log Viewer
npm run dev --workspace=apps/universal-log-viewer

# DevOps Secret Vault
npm run dev --workspace=apps/devops-secret-vault

# AI File Explorer
npm run dev --workspace=apps/ai-file-explorer

# Universal Live Translator
npm run dev --workspace=apps/universal-live-translator

# Professional Audio & Video Editor
npm run dev --workspace=apps/pro-av-editor
```

### 3.4 Packaging Applications Independently
Each application builds, packages, and releases independently via dedicated scripts:
```bash
# Windows portable ZIP and WiX MSI installers
.\scripts\package-windows.ps1 -AppName "universal-log-viewer"
.\scripts\package-windows.ps1 -AppName "devops-secret-vault"
.\scripts\package-windows.ps1 -AppName "ai-file-explorer"
.\scripts\package-windows.ps1 -AppName "universal-live-translator"
.\scripts\package-windows.ps1 -AppName "pro-av-editor"

# Linux tarball and AppImage packages
./scripts/package-linux.sh universal-log-viewer
./scripts/package-linux.sh devops-secret-vault
./scripts/package-linux.sh ai-file-explorer
./scripts/package-linux.sh universal-live-translator
./scripts/package-linux.sh pro-av-editor

# macOS universal binary and DMG bundles
./scripts/package-macos.sh universal-log-viewer
./scripts/package-macos.sh devops-secret-vault
./scripts/package-macos.sh ai-file-explorer
./scripts/package-macos.sh universal-live-translator
./scripts/package-macos.sh pro-av-editor

# Mobile Android APK/AAB and iOS XCFrameworks
./scripts/package-mobile.sh universal-log-viewer
./scripts/package-mobile.sh devops-secret-vault
./scripts/package-mobile.sh ai-file-explorer
./scripts/package-mobile.sh universal-live-translator
./scripts/package-mobile.sh pro-av-editor
```

---

## 4. Coding Conventions & Invariants

1. **Safety First:**
   - Every crate must specify `#![forbid(unsafe_code)]` at its root.
   - Any low-level memory mapping or FFI integration that requires `unsafe` must be isolated in a dedicated module accompanied by an approved Architecture Decision Record (ADR) and explicit `// SAFETY:` justifications.
2. **Error Handling Architecture:**
   - **Shared Libraries & Domain Cores:** Exclusively utilize `thiserror` to define strongly typed, exhaustive error enums.
   - **Application & IPC Boundaries:** Use typed `ServiceError` or `anyhow` for top-level command boundaries, error translation, and logging.
3. **Zero Leaks & Zero Egress:**
   - Never import networking client libraries (`reqwest`, `hyper`, `curl`, `ureq`) into application runtime builds.
   - All secret material must be held in `Zeroizing` byte buffers or strings.
4. **Zero Mocks in Production:**
   - Never return fake or simulated metrics (e.g., hardcoded RSS/CPU, mock keyrings in production).
   - Test adapters must be strictly isolated behind `#[cfg(test)]`.

---

## 5. Mobile Client Development Workflow

All 5 applications feature production Flutter mobile clients residing in `apps/<app>/mobile/`:

| Application | Mobile Directory | FFI Bridge File | Test Suite |
| :--- | :--- | :--- | :--- |
| **Universal Log Viewer** | `apps/universal-log-viewer/mobile/` | `lib/bindings/ulv_ffi.dart` | `test/ulv_mobile_test.dart` |
| **DevOps Secret Vault** | `apps/devops-secret-vault/mobile/` | `lib/bindings/vault_ffi.dart` | `test/vault_mobile_test.dart` |
| **AI File Explorer** | `apps/ai-file-explorer/mobile/` | `lib/bindings/afe_ffi.dart` | `test/afe_mobile_test.dart` |
| **Universal Live Translator** | `apps/universal-live-translator/mobile/` | `lib/bindings/ult_ffi.dart` | `test/ult_mobile_test.dart` |
| **Pro Audio & Video Editor** | `apps/pro-av-editor/mobile/` | `lib/bindings/ave_ffi.dart` | `test/ave_mobile_test.dart` |

### Running Mobile Unit & Widget Tests
```bash
# Run tests for a specific mobile client
cd apps/universal-log-viewer/mobile && flutter test
cd apps/devops-secret-vault/mobile && flutter test
cd apps/ai-file-explorer/mobile && flutter test
cd apps/universal-live-translator/mobile && flutter test
cd apps/pro-av-editor/mobile && flutter test
```
