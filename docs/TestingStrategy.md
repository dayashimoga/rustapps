# Comprehensive Testing Strategy & Quality Assurance Architecture (TSQA)

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Quality Assurance Standard  
**Mandate:** Line & branch coverage $\ge 90\%$; 100% test, clippy, and formatting pass rate.

---

## 1. Testing Philosophy & Quality Standard

The monorepo enforces an uncompromising Quality Assurance regime. All code must pass automated validation across eight test tiers and six case types per feature before merge or release. CI runs the full suite on every push and fails if code coverage drops below 90% or if any performance benchmark regresses.

---

## 2. Eight Test Tiers

```text
+-------------------------------------------------------------------------------+
| Tier 1: Unit Tests (Rust cargo test, Vitest frontend components)             |
+-------------------------------------------------------------------------------+
| Tier 2: Integration Tests (Cross-crate workflows, storage + crypto + search) |
+-------------------------------------------------------------------------------+
| Tier 3: E2E Tests (Playwright desktop automation of Tauri webviews)           |
+-------------------------------------------------------------------------------+
| Tier 4: Regression Tests (Historical defect reproduction harnesses)          |
+-------------------------------------------------------------------------------+
| Tier 5: Performance Tests (SLA verification: startup, memory, latency)       |
+-------------------------------------------------------------------------------+
| Tier 6: Benchmark Tests (Criterion statistical micro & macro benchmarks)     |
+-------------------------------------------------------------------------------+
| Tier 7: Security Tests (Crypto vectors, zeroize verification, tamper checks) |
+-------------------------------------------------------------------------------+
| Tier 8: Compatibility Tests (OS matrix, filesystem edge cases, encodings)    |
+-------------------------------------------------------------------------------+
```

### Detailed Tier Breakdown
1. **Tier 1 — Unit Tests:** Verify fine-grained function contracts, structs, state transitions, and error handling in isolation.
2. **Tier 2 — Integration Tests:** Verify cross-crate composition (e.g., storing an encrypted secret in SQLite, indexing it in Tantivy, retrieving it via settings).
3. **Tier 3 — End-to-End (E2E) Tests:** Drive desktop applications via Playwright to simulate genuine user flows (opening 100GB logs, creating secrets, filtering files).
4. **Tier 4 — Regression Tests:** Every identified bug or security issue is accompanied by a permanent regression test added to `tests/regression/`.
5. **Tier 5 — Performance Tests:** Automated verification of operational SLAs ($< 2,000\,\text{ms}$ startup, $< 50\,\text{ms}$ search, $< 300\,\text{MB}$ RAM, $< 2\%$ CPU).
6. **Tier 6 — Benchmark Tests:** Criterion harnesses measuring throughput (GB/s, docs/s) and latency distribution ($p50, p95, p99$) with historical baselines.
7. **Tier 7 — Security Tests:** Known cryptographic test vectors (NIST CAVP for AES-GCM, RFC 7693 for BLAKE3, RFC 8032 for Ed25519), memory zeroization checks, and tamper resistance.
8. **Tier 8 — Compatibility Tests:** Cross-platform behavior on Windows (MSVC), Linux (GNU), and macOS (Darwin); UTF-8/UTF-16/CP1252 log encodings; symlink handling; path length limits.

---

## 3. Six Test Case Types per Feature

For every feature or shared capability, test suites must explicitly implement all six case types:

| Case Type | Purpose | Example (DevOps Secret Vault) | Example (Universal Log Viewer) | Example (AI File Explorer) |
| :--- | :--- | :--- | :--- | :--- |
| **1. Positive** | Validate correct behavior under standard expected inputs. | Create, save, and retrieve an API key successfully. | Open a 1GB structured JSON log and view line 100. | Crawl folder, index files, query Tantivy in 21 µs. |
| **2. Negative** | Validate rejection of malformed or unauthorized inputs. | Attempt decryption with an incorrect passphrase; verify error. | Feed invalid regex syntax; verify safe error prompt without crash. | Invalid regex or nonexistent directory path returns structured error. |
| **3. Boundary** | Validate extremes (0 bytes, max limits, off-by-one, empty strings). | Secret with empty description, 1MB payload, 0-character notes. | 0-byte log file, 1-byte file, line exceeding 10MB without newline. | 0-byte files deduped, deeply nested directories, empty folder analytics. |
| **4. Performance**| Validate execution within strict SLA limits. | Search 100,000 secrets in $< 30\,\text{ms}$. | Index 100GB log in $< 2\,\text{s}$; query in $< 50\,\text{ms}$. | Tantivy query $< 50\,\text{ms}$ (measured 21.58 µs), squarified treemap 292 µs. |
| **5. Regression** | Validate historical defects remain fixed permanently. | Verify TOTP timezone drift fix remains active. | Verify ANSI escape code stripping doesn't drop trailing characters. | Windows NTFS file handle buffer flush before metadata read. |
| **6. Failure** | Validate recovery from catastrophic environment failures. | Disk write fails midway through database save; verify WAL rollback. | Underlying file is truncated or deleted while mmap handle is open. | Mid-rename permission failure triggers reverse rollback restoring all original paths. |


---

## 4. Test Reporting & CI Artifacts

Continuous integration automatically compiles and publishes four standardized report artifacts:
1. **JUnit XML Reports:** Standard machine-readable test execution summaries (`target/junit.xml`).
2. **HTML Test Reports:** Interactive browsable test execution results.
3. **Coverage Reports (LCOV & HTML):** Line and branch coverage reports generated via `cargo-llvm-cov` or `cargo-tarpaulin` (`target/coverage/html/index.html`).
4. **Criterion Benchmark Reports:** Detailed interactive charts showing throughput, latency distributions, and regression comparisons against baseline (`target/criterion/report/index.html`).

---

## 5. Coverage Enforcement & CI Gates

```bash
# Line and branch coverage validation
cargo llvm-cov --workspace --all-features --lcov --output-path target/coverage/lcov.info
cargo llvm-cov --workspace --all-features --fail-under-lines 90
```

Any build with $< 90\%$ test coverage fails CI immediately.
