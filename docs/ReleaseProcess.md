# Release Process & Distribution Governance Specification

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** DevOps & Release Management Standard  

---

## 1. Release Philosophy & Cadence

Releases represent cryptographically signed, production-ready, standalone packages. The project adheres to strict **Semantic Versioning 2.0.0** (`vMAJOR.MINOR.PATCH`):
- **MAJOR:** Incompatible architectural or breaking schema changes.
- **MINOR:** Backward-compatible new features, parsers, or platform expansions.
- **PATCH:** Backward-compatible security fixes, bug repairs, or performance optimizations.

### Release Cadence
- **Standard Releases:** Bi-weekly or at the conclusion of gated development phases.
- **Security Hotfixes:** Out-of-band within 24 hours of patch verification.

---

## 2. Pre-Release Quality Gates

Prior to creating any release tag, the release manager must verify:
- [ ] 100% passing automated test suite across all 8 tiers and 6 test case types.
- [ ] Code coverage $\ge 90\%$ verified via `cargo llvm-cov`.
- [ ] `cargo clippy -- -D warnings` and `cargo fmt -- --check` pass with zero diagnostics.
- [ ] `cargo audit` and `cargo deny check` report zero vulnerabilities and zero license violations.
- [ ] Criterion benchmarks show zero regressions against historical baselines; startup $< 2,000\,\text{ms}$, search $< 50\,\text{ms}$, RAM $< 300\,\text{MB}$.
- [ ] `Changelog.md`, `ImplementationPlan.md`, and `Todo.md` are updated and committed.

---

## 3. Git Tagging & Release Automation

A release is initiated by pushing an annotated, cryptographically signed Git tag matching `v*`:

```bash
git tag -s v1.0.0 -m "Release v1.0.0: Universal Log Viewer MVP"
git push origin v1.0.0
```

### Automated Release Pipeline (`.github/workflows/release.yml`)
Pushing a `v*` tag triggers the automated multi-platform release workflow:

```text
[ Git Tag v* Pushed ]
         |
         +---> Matrix Build Job
         |       ├── Windows x64 & ARM64 (MSI, Portable EXE)
         |       ├── Linux x64 & ARM64 (AppImage, DEB, RPM)
         |       └── macOS Intel & Apple Silicon (DMG, tarball)
         |
         +---> Quality & Security Validation
         |       ├── Run 8-tier test suites
         |       ├── Generate JUnit XML & HTML test reports
         |       └── Verify Coverage >= 90%
         |
         +---> Supply Chain & Integrity
         |       ├── Generate CycloneDX & SPDX SBOM
         |       ├── Compute SHA-256 Checksums
         |       └── Sign artifacts with Ed25519 Release Key
         |
         +---> GitHub Release Publication
                 ├── Attach native installers, portable bundles, and signatures
                 ├── Attach SHA-256 checksums
                 ├── Attach SBOM files
                 ├── Attach test coverage & benchmark HTML reports
                 └── Set 90-day artifact retention policy
```

---

## 4. Cryptographic Artifact Integrity

Every release package is distributed alongside:
1. `<filename>.sha256`: SHA-256 hash formatted for `sha256sum -c`.
2. `<filename>.sig`: Detached Ed25519 signature generated via developer release key.
3. `sbom-cyclonedx.json`: CycloneDX Software Bill of Materials.
4. `sbom-spdx.json`: SPDX Software Bill of Materials.

---

## 5. Artifact Retention & Storage Policy

- GitHub Release assets (installers, checksums, signatures, SBOMs) are retained **permanently**.
- Intermediate build logs, raw diagnostic traces, and full profiling data are configured with a **90-day retention policy**.

---

## 6. Hotfix Procedure

1. Branch from the released tag: `git checkout -b hotfix/v1.0.1 v1.0.0`.
2. Apply minimal targeted fix and add regression test reproducing the issue in `tests/regression/`.
3. Verify test pass and coverage $\ge 90\%$.
4. Append to `Changelog.md` under a new patch header.
5. Merge into `main`, tag `v1.0.1`, and trigger the automated release pipeline.
