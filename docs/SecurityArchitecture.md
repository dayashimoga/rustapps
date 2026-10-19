# Security Architecture & Threat Model Specification (SAS)

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Defensive Security Blueprint  

---

## 1. Security Philosophy & Principles

The applications in this monorepo handle highly sensitive operational data (passwords, SSH keys, cloud credentials, multi-gigabyte server logs, and personal filesystem indices). Consequently, security is designed directly into the foundation rather than layered on top.

### Guiding Axioms
1. **Zero Egress (True Air-Gap):** Complete absence of network sockets, analytics pings, cloud backends, and remote telemetry.
2. **Defense in Depth:** Multiple independent security barriers protect secrets at rest, in transit (IPC), and in memory.
3. **Cryptographic Provability:** Standardized, peer-reviewed cryptographic primitives exclusively; zero custom cryptography.
4. **Minimal Attack Surface:** No unneeded daemons, no background system services, and `#![forbid(unsafe_code)]` across the codebase.

---

## 2. Threat Modeling (STRIDE / DREAD Analysis)

| Threat Category | Potential Vector | Architectural Mitigation | Status |
| :--- | :--- | :--- | :--- |
| **Spoofing** | Attacker impersonating authorized user to access vault | Native hardware biometrics (Windows Hello, Touch ID) + Argon2id master passphrase verification. | Mitigated |
| **Tampering** | Modification of encrypted database or update package | AES-256-GCM authentication tags on records; Ed25519 cryptographic signatures on update bundles and exports. | Mitigated |
| **Repudiation** | Unauthorized changes to vault records without trace | Cryptographically hashed local audit log recording timestamp and action type. | Mitigated |
| **Information Disclosure** | Memory dump inspection or paging file extraction of secrets | Memory scrubbing with `zeroize`; pinning sensitive allocations; immediate zeroization on drop. | Mitigated |
| **Denial of Service** | Processing malicious 500GB log file to exhaust RAM/CPU | Zero-copy `memmap2` streaming; bounded virtualized viewport; rate-limited parser worker pools. | Mitigated |
| **Elevation of Privilege** | Sandboxed plugin attempting arbitrary OS execution | Capability-restricted plugin sandbox with isolated execution traits and timeout quotas. | Mitigated |

---

## 3. Cryptographic Primitives & Specifications

### 3.1 Symmetric Payload Encryption: AES-256-GCM
- **Algorithm:** AES-256 in Galois/Counter Mode (GCM).
- **Key Length:** 256 bits (32 bytes).
- **Nonce/IV:** 96 bits (12 bytes) cryptographically random, generated via OS CSPRNG (`rand::rngs::OsRng`). Re-use of nonces is cryptographically prohibited.
- **Authentication Tag:** 128 bits (16 bytes) appended to ciphertext; verified prior to decryption.

### 3.2 Key Derivation Function: Argon2id
- **Algorithm:** Argon2id (hybrid data-dependent and data-independent memory-hard function).
- **Default Profile (OWASP 2024+ Compliant):**
  - Memory Cost ($m$): $64\,\text{MiB}$ ($65,536\,\text{KiB}$).
  - Time Cost ($t$): $3$ iterations.
  - Parallelism ($p$): $4$ threads.
  - Salt: 128 bits (16 bytes) CSPRNG-generated random salt per vault.
- **Adaptive Tuning:** Hardware-calibrated parameter selection ensuring key derivation takes $\approx 500\,\text{ms}$ on target hardware.

### 3.3 Asymmetric Signatures: Ed25519
- **Algorithm:** Ed25519 (Edwards-curve Digital Signature Algorithm over Curve25519).
- **Usage:** Verification of standalone application update packages and tamper-evident backup archive manifests.

### 3.4 Secure Memory Management & Anti-Forensics (`zeroize`)
- All cryptographic keys, intermediate hash states, master passphrases, and decrypted secret values are encapsulated within types implementing `zeroize::Zeroize` and `zeroize::ZeroizeOnDrop`.
- Plaintext secrets are dropped immediately upon UI transfer or clipboard handoff.
- The clipboard manager implements an automated securely scrubbed timer (default 30 seconds) that clears secrets from system clipboard memory.

---

## 4. Native OS Biometric Integration

| Platform | Subsystem / API | Execution Mechanism | Fallback Mechanism |
| :--- | :--- | :--- | :--- |
| **Windows** | Windows Hello / Credential Manager | Win32 / WinRT Biometric API | Master Passphrase |
| **macOS** | LocalAuthentication / Touch ID | Secure Enclave FFI bindings | Master Passphrase |
| **Linux** | Secret Service / Polkit | D-Bus Secret Service API | Master Passphrase |
| **Android** | BiometricPrompt / Android Keystore | Hardware-backed TEE / StrongBox | Master Passphrase |
| **iOS** | LocalAuthentication / Secure Enclave | `kSecAccessControlBiometryAny` | Master Passphrase |

---

## 5. Air-Gapped Egress Boundary & Supply Chain

1. **Air-Gap Verification:**
   - The root workspace forbids dependencies that pull in `reqwest`, `hyper`, `curl`, `socket2`, or raw TCP sockets in runtime binaries.
   - Continuous integration runs automated symbol checks ensuring no network socket bindings are present in compiled release artifacts.
2. **Dependency & Supply Chain Controls:**
   - `cargo deny check` actively blocks:
     - Non-OSI compliant licenses (GPL copyleft violations, commercial proprietary licenses).
     - Known vulnerabilities recorded in the RustSec Advisory Database.
     - Multiple conflicting duplicate versions of major dependencies.
   - Automated generation of CycloneDX and SPDX Software Bill of Materials (SBOM) for every release build.
   - Full dependency lock via `Cargo.lock` with deterministic cryptographic checksums.

---

## 6. OWASP Top 10 & ASVS Alignment

- **A01: Broken Access Control:** Addressed via role-less standalone single-tenant model, strict local permissions, and biometric authentication.
- **A02: Cryptographic Failures:** Addressed via Argon2id, AES-256-GCM, zeroize, and OS CSPRNG.
- **A03: Injection:** Addressed via parameterized SQLite queries exclusively; raw SQL string concatenation is forbidden.
- **A04: Insecure Design:** Addressed via threat modeling, offline-first isolation, and bounded resource quotas.
- **A05: Security Misconfiguration:** Hardened SQLite WAL configuration with safe defaults, zero network exposure.
- **A06: Vulnerable Components:** Automated `cargo audit` and `cargo deny` in every pull request and push.
- **A07: Identification and Authentication Failures:** Rate-limited passphrase attempts with exponential backoff on vault unlocks.
- **A08: Software and Data Integrity Failures:** Ed25519 signature verification on all update packages and backup files.
- **A09: Security Logging and Monitoring:** Local encrypted audit logging of security actions; zero remote exfiltration.
- **A10: Server-Side Request Forgery (SSRF):** Not applicable; complete absence of network client stacks.

---

## 7. Security Review Checklist

Before any code is committed or merged:
- [ ] Are all secrets and plaintext decrypted buffers wrapped in `Zeroizing` containers?
- [ ] Is `#![forbid(unsafe_code)]` preserved, or is an approved ADR present for FFI/mmap?
- [ ] Are all database operations strictly parameterized?
- [ ] Are random numbers generated exclusively via `OsRng`?
- [ ] Have `cargo audit` and `cargo deny check` passed with 0 warnings or vulnerabilities?
- [ ] Does the change introduce any network socket or egress dependencies? (Must be 0).
