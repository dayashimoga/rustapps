---
name: security-benchmark-gate
description: >-
  Use this skill when running security audits, cryptographic verification, air-gap
  network egress checks, or Criterion performance benchmarks across the monorepo.
---

# Security & Performance Benchmark Gate Skill

Run this skill at the conclusion of every implementation phase to produce verifiable audit evidence.

## 1. Cryptographic & Air-Gap Security Verification

```powershell
# 1. Supply chain license, ban, and vulnerability check
cargo deny check

# 2. Verify zero prohibited network client crates in runtime dependency trees
$forbidden = @("reqwest", "hyper", "curl", "ureq", "attohttpc")
foreach ($crate in $forbidden) {
    $tree = cargo tree --workspace --edges normal | Select-String -Pattern "\b$crate\b"
    if ($tree) {
        Write-Error "AIR-GAP VIOLATION: Found prohibited runtime network dependency: $crate"
    }
}

# 3. Verify zeroize coverage on sensitive secret types
cargo test -p shared-encryption -p devops-secret-vault
```

## 2. Criterion Performance & SLA Regression Gate

```powershell
# Run benchmark suite and validate against SLA gates
cargo bench --bench bench_suite
```

### Mandatory SLA Thresholds:
- **Search Queries (Tantivy / Sparse Index):** $< 50\text{ ms}$ ($50,000\text{ µs}$)
- **TOTP Generation (RFC 6238):** $< 10\text{ µs}$
- **Vault AES-256-GCM Payload Encrypt/Decrypt (4KB):** $< 25\text{ µs}$
- **3-Tier Duplicate Detection (1,000 files):** $< 50\text{ ms}$
- **Squarified Treemap Layout (10,000 nodes):** $< 5\text{ ms}$
- **Live Translator Text Tokenization + NMT Step:** $< 150\text{ ms}$
- **AV Editor Timeline Frame Evaluation & Audio Biquad DSP (48kHz stereo frame):** $< 2\text{ ms}$ per 16.6ms frame budget

