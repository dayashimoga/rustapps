# Monorepo Developer & Contributor Guide

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Internal Engineering Standard  

---

## 1. Development Prerequisites & Environment Setup

To build, test, and contribute to the monorepo, configure the following tools on your workstation:

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
- **Platform Native Build Tools:**
  - **Windows:** Visual Studio 2022 C++ Build Tools (MSVC), Windows 10/11 SDK.
  - **Linux:** `build-essential`, `libssl-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`.
  - **macOS:** Xcode Command Line Tools.

---

## 2. Monorepo Quickstart

```bash
# Clone the repository
git clone https://github.com/organization/offline-apps-monorepo.git
cd offline-apps-monorepo

# Install root JavaScript/TypeScript dependencies
npm install

# Build all shared crates and platform libraries
cargo build --workspace

# Run all unit and integration tests
cargo test --workspace --all-targets

# Check code formatting and linter diagnostics
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 3. Standard Development Workflows

### 3.1 Cargo Commands
- **Check Compilation:** `cargo check --workspace`
- **Run Fast Tests:** `cargo test --workspace`
- **Run Targeted Benchmark:** `cargo bench -p shared-search`
- **Security & License Audit:**
  ```bash
  cargo audit
  cargo deny check
  ```

### 3.2 Frontend & E2E Commands
- **Run Unit Tests (Vitest):** `npm run test:unit`
- **Run Desktop E2E Tests (Playwright):** `npm run test:e2e`
- **Typecheck Frontend:** `npm run typecheck`

### 3.3 Running Applications Locally
```bash
# Run Universal Log Viewer in desktop dev mode
npm run dev --workspace=apps/universal-log-viewer

# Run DevOps Secret Vault in desktop dev mode
npm run dev --workspace=apps/devops-secret-vault

# Run AI File Explorer in desktop dev mode
npm run dev --workspace=apps/ai-file-explorer
```

---

## 4. Coding Conventions & Invariants

1. **Safety First:**
   - Every crate must specify `#![forbid(unsafe_code)]` at its root.
   - Any low-level memory mapping or FFI integration that requires `unsafe` must be isolated in a dedicated module accompanied by an approved Architecture Decision Record (ADR) justifying its safety invariants.
2. **Error Handling Architecture:**
   - **Shared Libraries & Domain Cores:** Exclusively utilize `thiserror` to define strongly typed, exhaustive error enums.
   - **Application & IPC Boundaries:** Use `anyhow` for top-level command boundaries, error translation, and logging.
3. **Zero Leaks & Zero Egress:**
   - Never import networking client libraries (`reqwest`, `hyper`, `curl`) into desktop application builds.
   - All secret material must be held in `Zeroizing` byte buffers or strings.

---

## 5. Mandatory Contribution & PR Lifecycle

Every task must execute through the mandated three-step cycle:
1. **Before Coding:**
   - Review and update `TechnicalRequirements.md` and `TechnicalArchitecture.md`.
   - Append to `ImplementationPlan.md` and `Todo.md`.
   - Record architectural choices in `ADR.md`.
2. **During Coding:**
   - Follow Test-Driven Development across all 8 tiers and 6 test case types.
   - Maintain rustdoc comments for all public APIs.
   - Maintain Criterion benchmark harnesses.
3. **After Coding:**
   - Append changes to `Changelog.md` (Keep a Changelog format).
   - Update all 8 sections of `CodeUnderstanding.md`.
   - Update `SecurityArchitecture.md`, `TestingStrategy.md`, `DeploymentGuide.md`, `DeveloperGuide.md`, `UserGuide.md`, `RiskRegister.md`, `PerformanceStrategy.md`.
   - Move completed tasks intact to the Completed section in `Todo.md`.
   - Verify 100% passing tests, zero benchmark regressions, and $\ge 90\%$ test coverage.
