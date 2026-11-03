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

### 3.5 Pure Rust RFC 6238 TOTP Engine Security
- **Algorithm:** RFC 6238 Time-Based One-Time Password with RFC 4648 Base32 decoding and RFC 2104 HMAC.
- **In-House Cryptographic Implementation:** Built natively in `#![forbid(unsafe_code)]` Rust using only workspace cryptographic dependencies (`sha2`). Implements RFC 3174 SHA-1 with exact Big-Endian word processing and RFC 6238 dynamic truncation.
- **Supply Chain Elimination:** Avoids pulling in third-party unvetted TOTP or Base32 crates, maintaining zero supply chain vulnerability risk.
- **Anti-Replay & Time Windows:** Computes counter $T = \lfloor \frac{\text{unix\_time}}{\text{period}} \rfloor$; supports configurable window drift tolerance for verification while remaining strictly offline.

### 3.6 Zero-Knowledge In-Memory Credential Reuse Detection
- **Threat Vector:** Auditing all vault credentials for reuse could expose plaintexts to heap inspection or side-channel leakage if plaintexts are compared directly in memory.
- **Mitigation:** Ephemeral Keyed Hashing. During security risk assessment, an ephemeral 256-bit cryptographically random salt is generated in volatile memory (`salt = OsRng`).
- **Hash Identifier:** For each credential, a zero-knowledge duplicate identifier is calculated: $H = \text{HMAC-SHA256}(\text{salt}, \text{secret\_value})$ or $\text{SHA-256}(\text{salt} \mathbin{\Vert} \text{secret\_value})$.
- Identical secrets generate identical buckets with mathematical certainty; distinct secrets are cryptographically unlinkable.
- The ephemeral salt is immediately zeroized upon health assessment completion, making cross-audit correlation or forensic reconstruction impossible.

### 3.7 Tamper-Evident Portable `.vaultbackup` Specification
- **Magic Header:** 16-byte fixed binary signature (`VAULT_BACKUP_V1\0\0`).
- **Key Derivation:** Argon2id ($m=64\,\text{MiB}, t=3, p=4$) with dedicated 16-byte CSPRNG random salt per backup file.
- **Authenticated Payload:** AES-256-GCM authenticated ciphertext wrapping canonical JSON representation of vault metadata and encrypted secret payloads.
- **Embedded Integrity Checksum:** Plaintext JSON SHA-256 digest verified post-decryption before any records are committed to the local database, guaranteeing protection against corrupted or manipulated archives.

### 3.8 AI File Explorer Local Filesystem Sandbox & Path Traversal Mitigations
- **Path Canonicalization & Containment:** All filesystem paths submitted via Tauri IPC are canonicalized and validated. Traversal sequences (`../`, null bytes, control characters) and unauthorized symlink dereferences targeting restricted system partitions are rejected before I/O.
- **Transactional Two-Phase Batch Rename Safety (ADR-0024):** Dry-run preflight checks detect namespace collisions, identical source/destination paths, or permission barriers before executing any disk modifications. An in-memory reverse-order journal records every atomic rename step, enabling automatic rollback on midway failure and user-initiated single-click undo.
- **Safe 3-Tier Deduplication (ADR-0022):** The multi-stage pipeline strictly avoids unnecessary full file reads. Stage 1 groups by file size, Stage 2 samples 4KB header and footer hashes, and Stage 3 uses BLAKE3 only on surviving candidates. Duplicate detection is strictly non-destructive; deletion or hardlinking requires explicit user confirmation.
- **Strict Offline Air-Gap Guarantee:** File paths, folder hierarchies, sizes, hashes, and filenames never leave the local machine. No external cloud indexing, network sockets, or remote telemetry are permitted.

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

---

## 8. Offline Intelligence & Content Extraction Security Controls

The advanced intelligence capabilities of AI File Explorer (Phase 6) adhere to strict air-gap and memory safety boundaries:

1. **Pure-Rust Offline OCR Engine (ADR-0026):**
   - Implements 100% pure-Rust image decoding, binarization, and glyph classification without linking external C/C++ native libraries (e.g., zero Tesseract or Leptonica C dependencies). This eliminates common native memory corruption vulnerabilities (such as buffer overflows or use-after-free bugs).
   - Enforces a hard dimension limit (`MAX_IMAGE_DIMENSION = 4096` pixels per axis) to prevent allocation exhaustion and pixel decompression bombs.

2. **Perceptual Visual Hashing Isolation (ADR-0027):**
   - 64-bit difference perceptual hashing (`dHash`) executes entirely in volatile memory with box sampling and bitwise gradient shifts (`u64`).
   - Image buffers are freed immediately after computing the 64-bit hash; no uncompressed bitmaps are retained in long-lived memory structures.

3. **Multi-Format Document Extraction Hardening (ADR-0028):**
   - Hard upper bound on document size (`MAX_DOC_SIZE_BYTES = 50 MB`).
   - Pure-Rust `flate2` streaming decompression operates on bounded buffers to prevent zip bombs, recursive compression loops, or infinite FlateDecode streams.
   - Text parsing ignores active scripts, macros, or external references in PDF and DOCX files, preventing script injection or XML external entity (XXE) attacks.

4. **Quantized Vector Embeddings & In-Memory Index (ADR-0029):**
   - 8-bit integer quantization (`[i8; 64]`) consumes exactly 64 bytes per document, allowing 100,000 indexed documents to fit in $< 6.4\,\text{MB}$ of vector memory, well within the 300MB RAM budget.
   - Zero-call external API isolation: All tokenization, character n-gram hashing, and vector projections are calculated on-device with zero cloud telemetry or network egress.

---

## 9. Universal Live Translator & Pro AV Editor Offline Security Controls

1. **Universal Live Translator Security Controls:**
   - **Local Acoustic & NMT Isolation:** Language identification (LID), Neural Machine Translation (NMT), Speech Recognition (ASR), and Formant Speech Synthesis (TTS) execute 100% locally on-device. Audio streams from WASAPI, CoreAudio, ALSA, or PipeWire are processed in volatile memory buffers and immediately zeroized on drop.
   - **Encrypted Local History & Phrasebook:** User phrasebook records and translation logs are stored in a dedicated local SQLite database operating in WAL mode, with sensitive notes encrypted via AES-256-GCM.
   - **Offline Camera OCR & Subtitle Parsing:** Camera image frames and SRT/VTT subtitle files are parsed using pure-Rust decoders with strict memory bounds, preventing memory corruption or script injection vulnerabilities.

2. **Professional Audio & Video Editor Security Controls:**
   - **Atomic Project Persistence:** Project files (`.avedproj`) use atomic staging write patterns and are verified on load with BLAKE3 cryptographic hashes, preventing corrupted or tampered timelines from causing denial of service.
   - **Bounded Compositor & Frame Safety:** Frame decoding enforces dimension boundaries (`render_width <= 7680`, `render_height <= 4320`) and executes layer compositing in memory-safe Rust buffers with strict numeric overflow checks.
   - **Air-Gapped Media Processing:** Multi-track audio mixing, 8-band Biquad EQ filtering, EBU R128 loudness metering, and video color grading operate strictly offline with zero external network dependencies.


