# Enterprise Risk Register & Mitigation Strategy

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Operational Risk Management  
**Review Cadence:** Bi-weekly / At each Phase Gate  

---

## 1. Risk Assessment Methodology

Risks are assessed based on **Likelihood (L: 1–5)** and **Impact (I: 1–5)**, resulting in an aggregate **Risk Score (1–25)**:
- **Low (1–6):** Managed via standard development controls.
- **Medium (7–14):** Active architectural oversight and monitoring required.
- **High (15–25):** Critical architectural blocker; requires immediate defensive engineering.

---

## 2. Active Risk Register Matrix

| Risk ID | Category | Description | L | I | Score | Mitigation Strategy | Contingency Plan | Status |
| :--- | :--- | :--- | :---: | :---: | :---: | :--- | :--- | :--- |
| **RSK-001** | Performance | Memory exhaustion when loading 500GB+ logs in Universal Log Viewer. | 2 | 5 | **10** | Use zero-copy `memmap2` and sparse index holding only 64-bit line offsets. Heap RAM stays $< 250\,\text{MB}$. | Fallback to segmented chunk windowing if mmap virtual address space fails. | Controlled |
| **RSK-002** | Security | Memory dump / cold-boot attack extracting plaintext passwords or SSH keys from RAM. | 3 | 5 | **15** | Encapsulate all secret material in `zeroize::Zeroizing<T>` types; scrub upon drop; minimize plaintext duration in memory. | Introduce OS page locking (`mlock` / `VirtualLock`) via platform abstraction. | Controlled |
| **RSK-003** | Architectural | Tauri v2 or Webview memory accumulation during long-running sessions. | 3 | 4 | **12** | Implement strict virtualized DOM rendering in Svelte; recycle DOM elements; stream slices via IPC on demand. | Enforce periodic garbage collection trigger and background memory recycling. | Controlled |
| **RSK-004** | Supply Chain | Malicious dependency update or proprietary license contamination in transitive tree. | 2 | 5 | **10** | Continuous enforcement of `cargo deny check` and `cargo audit` in CI; lockfile validation; ban copyleft/proprietary licenses. | Pin explicit crate versions and maintain vendor mirror for offline reproducibility. | Controlled |
| **RSK-005** | Performance | Filesystem watcher drops events during burst disk operations (e.g., git checkout of 50k files). | 4 | 3 | **12** | Use asynchronous buffer queue with coalesce debounce logic; trigger incremental crawler rescan on queue overflow. | Periodic lightweight metadata integrity sync every 5 minutes. | Controlled |
| **RSK-006** | Platform | Windows Hello or macOS Touch ID FFI interface changes breaking biometric unlock. | 2 | 4 | **8** | Isolate biometrics behind clean platform traits in `shared-encryption`; thoroughly mock in unit/integration tests. | Seamless fallback to Argon2id master passphrase unlock. | Controlled |
| **RSK-007** | Performance | Tantivy indexing causing CPU spikes exceeding 2% idle budget. | 3 | 3 | **9** | Throttle background indexing worker threads; pin to lower process priority (idle/below normal). | Suspend indexing when battery powered or user activity detected. | Controlled |
| **RSK-008** | Security | Cryptographic side-channel attacks during Argon2id key derivation or AES-GCM verification. | 1 | 5 | **5** | Rely strictly on standardized, constant-time implementations (`aes-gcm` and `argon2` RustCrypto crates). | Continuous validation against NIST test vectors in Tier 7 tests. | Controlled |
| **RSK-009** | Performance | JSON streaming parser memory accumulation on deeply nested or massive JSON Lines exports. | 2 | 4 | **8** | Stream line-by-line directly from mmap byte slices; extract standard keys and discard raw parse tree immediately. | Fall back to targeted substring regex extraction if JSON syntax is malformed. | Controlled |
| **RSK-010** | Algorithmic | Drain prefix-tree combinatorial depth blowup on noisy unstructured log messages. | 2 | 4 | **8** | Enforce static depth bounding ($d=4$) and regex variable pre-masking (IPs, UUIDs, hex, numbers); clamp maximum children per node. | Truncate token lists to first 50 tokens if message exceeds standard length. | Controlled |
| **RSK-011** | Security | Plaintext password comparison during vault-wide credential reuse auditing leaking secrets in memory dumps. | 3 | 5 | **15** | Use ephemeral keyed hashing (`SHA-256(salt || secret)`) with volatile 32-byte salt; zero plaintexts compared; salt zeroized immediately. | Partition audits into bounded batches; enforce zeroize on drop. | Controlled |
| **RSK-012** | Functional | TOTP token generation drift on air-gapped devices with skewed system clocks. | 2 | 4 | **8** | Implement configurable time window drift allowance (±1 period) during verification and display live countdown dial. | Warn user if OS monotonic time drifts from system wall clock. | Controlled |
| **RSK-013** | Integrity | Malicious or corrupted backup archive injection overwriting valid vault database state. | 2 | 5 | **10** | Enforce `.vaultbackup` binary envelope with Argon2id KDF, AES-256-GCM authentication, and embedded SHA-256 checksum verified prior to commit. | Reject archive on any tag or checksum mismatch; abort transaction. | Controlled |

---

## 3. Governance & Incident Escalation

1. Any event or discovery that elevates a risk score to $\ge 15$ mandates an immediate halt to feature advancement until mitigated.
2. The Risk Register is formally evaluated at every gated Phase review.
