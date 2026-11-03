# Release Process & Distribution Governance Specification

**Document Version:** 2.0.0  
**Status:** Approved  
**Classification:** DevOps & Release Management Standard  

---

## 1. Release Philosophy & Independent Application Delivery

The monorepo enforces **independent application release lifecycles**. Each of the five flagship applications (`universal-log-viewer`, `devops-secret-vault`, `ai-file-explorer`, `universal-live-translator`, `pro-av-editor`) is versioned, packaged, tested, and published as a standalone product.

### Versioning Scheme
Releases follow **Semantic Versioning 2.0.0** (`vMAJOR.MINOR.PATCH`) prefixed by application name:
- `universal-log-viewer-v<version>`
- `devops-secret-vault-v<version>`
- `ai-file-explorer-v<version>`
- `universal-live-translator-v<version>`
- `pro-av-editor-v<version>`

Global monorepo milestones use `v<version>-<milestone>` tags (e.g. `v2.0.0-zero-mock`).

---

## 2. Pre-Release Quality Gates

Prior to creating any release tag, all mandatory Quality Gates must pass:
- [ ] 100% passing automated test suite across all 19 workspace crates (`cargo test --workspace --all-targets`).
- [ ] 100% passing frontend Vitest test suite (`npm run test:unit`).
- [ ] TypeScript typecheck passing with zero diagnostics (`npm run typecheck`).
- [ ] Code formatting verified with zero differences (`cargo fmt --all -- --check`).
- [ ] Clippy linter passing with zero warnings (`cargo clippy --workspace --all-targets --all-features -- -D warnings`).
- [ ] Zero-mock audit passing: all test fixtures and mock keyrings gated behind `#[cfg(test)]`.
- [ ] Air-gap verification: zero networking client libraries (`reqwest`, `hyper`, `curl`, `ureq`) in production dependencies.
- [ ] Criterion benchmarks confirm SLAs: startup $< 2,000\,\text{ms}$, primary op $< 50\,\text{ms}$ or 60 FPS, RAM $< 350\,\text{MB}$.
- [ ] `Changelog.md`, `ImplementationPlan.md`, and `Todo.md` synchronized.

---

## 3. Git Tagging & Release Automation

A release is initiated by pushing an annotated, cryptographically signed Git tag for the target application:

```bash
# Example: Tagging Pro AV Editor v0.1.0
git tag -a pro-av-editor-v0.1.0 -m "Release Pro AV Editor v0.1.0"
git push origin pro-av-editor-v0.1.0
```

### Dedicated Application Release Workflows
Pushing an application tag triggers its dedicated GitHub Actions workflow:
- `.github/workflows/release-universal-log-viewer.yml`
- `.github/workflows/release-devops-secret-vault.yml`
- `.github/workflows/release-ai-file-explorer.yml`
- `.github/workflows/release-universal-live-translator.yml`
- `.github/workflows/release-pro-av-editor.yml`

All workflows invoke the central `.github/workflows/reusable-app-release.yml` with the appropriate application matrix:

```text
[ Application Release Tag Pushed ]
          |
          +---> Desktop Matrix (Windows x64/ARM64, Linux x64/ARM64, macOS Universal)
          |       ├── Compile Rust core + Svelte 5 frontend
          |       ├── Package WiX MSI, AppImage, DEB, DMG, and Portable ZIP
          |       ├── Generate CycloneDX SBOM
          |       └── Compute SHA-256 Checksum & Ed25519 Signature
          |
          +---> Mobile Packaging (Android APK/AAB & iOS XCFramework)
          |       ├── Build Rust C-ABI FFI shared library
          |       └── Assemble Flutter mobile client bundle
          |
          +---> GitHub Release Publication
                  ├── Attach desktop installers & portable archives
                  ├── Attach mobile APK / XCFramework bundles
                  ├── Attach SHA-256 checksums and Ed25519 signatures
                  └── Attach CycloneDX SBOM manifests
```

---

## 4. Cryptographic Artifact Integrity

Every release package is distributed with:
1. `<filename>.sha256`: SHA-256 hash for verification (`sha256sum -c`).
2. `<filename>.sig`: Detached Ed25519 signature generated via release signing key.
3. `cyclonedx-sbom.json`: CycloneDX Software Bill of Materials.

---

## 5. Hotfix Procedure

1. Branch from the released tag: `git checkout -b hotfix/pro-av-editor-v0.1.1 pro-av-editor-v0.1.0`.
2. Apply minimal targeted fix and add regression test reproducing the issue in `tests/`.
3. Verify test pass rate (100%) and 0 clippy warnings.
4. Append to `Changelog.md` under a new patch header.
5. Merge into `master`, tag `pro-av-editor-v0.1.1`, and trigger the automated release pipeline.
