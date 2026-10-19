# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - 2026-10-19
- **Monorepo Foundation (Phase 0):**
  - Initialized Cargo root workspace and directory structure (`platform`, `shared/*`, `apps/*`, `docs`, `assets`, `scripts`, `tests`, `benchmarks`, `.github/workflows`).
  - Implemented 15 comprehensive architectural and governance specifications in `/docs/`.
  - Configured `deny.toml` enforcing OSI-approved open-source licenses and zero dependency advisories.
  - Configured `rustfmt.toml` and `clippy.toml` with strict `#![forbid(unsafe_code)]` compliance.
  - Implemented 7 production-grade GitHub Actions CI/CD workflows: `build.yml`, `test.yml`, `coverage.yml`, `security.yml`, `benchmark.yml`, `documentation.yml`, and `release.yml`.
  - Implemented all 9 core shared Rust crates:
    - `shared-logging`: Centralized tracing subscriber, rolling file appender, NDJSON/pretty formatters.
    - `shared-storage`: SQLite connection pooling with WAL mode, pragmas, schema migrations, atomic file I/O.
    - `shared-encryption`: AES-256-GCM authenticated encryption, Argon2id KDF, Zeroize secure memory scrubbing, Ed25519 signatures, OS biometrics abstraction.
    - `shared-search`: Embedded Tantivy engine wrapper, custom schemas, query parsers, and highlighter.
    - `shared-indexing`: Parallel multi-threaded filesystem walker, chunking pipelines, cancellable tokens.
    - `shared-settings`: Schema-validated persistent configuration manager with change event notifications.
    - `shared-telemetry`: Strictly opt-in local diagnostic counters, zero network egress.
    - `shared-updater`: Standalone binary update package verifier and atomic staged replacer with rollback.
    - `shared-plugins`: Extensible plugin manifest, capability isolation, and lifecycle manager.
  - Implemented `platform-common` crate for OS detection, file paths, and runtime CPU/RAM resource monitoring.
  - Implemented Vitest and Playwright desktop E2E testing harnesses.
  - Implemented cross-crate integration tests in `/tests` and Criterion benchmark harnesses in `/benchmarks`.
  - Implemented cross-platform build, test, coverage, benchmark, and SBOM generation scripts in `/scripts`.
  - Completed Phase 0 Quality Gate: 34 of 34 unit and integration tests passing; 100% clean formatting and clippy (`-D warnings`); Criterion benchmark harness operational with zero regressions.

