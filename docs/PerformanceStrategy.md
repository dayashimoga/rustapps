# Performance Engineering Strategy & SLA Enforcement Specification (PES)

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Performance & Engineering Standard  

---

## 1. Performance Axioms & Core SLAs

Performance is an explicit architectural constraint, not an afterthought. Every commit and release must satisfy the nonnegotiable Service Level Agreements (SLAs) defined below:

| Metric | DevOps Secret Vault | AI File Explorer | Universal Log Viewer | SLA Gate (CI Block) |
| :--- | :--- | :--- | :--- | :--- |
| **Cold Startup Time** | $< 1,500\,\text{ms}$ | $< 2,000\,\text{ms}$ | $< 1,500\,\text{ms}$ | **$\ge 2,000\,\text{ms}$ Fails** |
| **Search Query Latency** | $< 30\,\text{ms}$ (100k secrets) | $< 50\,\text{ms}$ (10M files) | $< 50\,\text{ms}$ (100GB log) | **$\ge 50\,\text{ms}$ Fails** |
| **Peak Resident RAM (RSS)**| $< 120\,\text{MB}$ | $< 280\,\text{MB}$ | $< 250\,\text{MB}$ (500GB file) | **$\ge 300\,\text{MB}$ Fails** |
| **Idle CPU Utilization**| $< 0.5\%$ | $< 1.0\%$ | $< 0.8\%$ | **$\ge 2.0\%$ Fails** |
| **Throughput / Scan** | N/A | $> 100,000\,\text{files/s}$ | 100GB in $< 2\,\text{s}$ | **$> 2\,\text{s}$ Fails** |
| **UI Render Frame Rate**| 60 FPS steady | 60 FPS steady | 60 FPS steady | **Any jank fails** |

---

## 2. Architectural Performance Patterns

### 2.1 Zero-Copy Memory-Mapped I/O (`memmap2`)
Universal Log Viewer never reads entire log files into heap buffers. It projects files into 64-bit virtual memory address space. The operating system page cache transparently loads only the 4KB pages containing lines currently rendered in the viewport.

### 2.2 SIMD-Accelerated Newline Scanning
Building line-offset indexes across gigabytes of log text uses SIMD vectorization (AVX2 on x86_64, NEON on ARM64) to scan 32 or 64 bytes per instruction cycle, achieving raw scan speeds exceeding $50\,\text{GB/s}$.

### 2.3 Lock-Free Sparse Indexing
Rather than storing pointers or strings for every single line in a 500GB log file (which would consume tens of gigabytes of RAM), the engine records sparse 64-bit byte offsets (e.g., every 64th or 256th line) and interpolates exact line boundaries on demand in O(1) time.

### 2.4 Optimized SQLite WAL Configuration
Shared storage enforces:
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -64000; -- 64MB cache
PRAGMA mmap_size = 268435456; -- 256MB mmap
```
This configuration yields sub-millisecond read transactions and prevents reader/writer contention.

### 2.5 Virtualized 60 FPS Viewports
Frontend viewports render exclusively the 30–60 lines visible on screen, reusing DOM nodes dynamically. Memory consumption in the webview remains constant regardless of whether the opened file contains 100 lines or 100 million lines.

---

## 3. Criterion Benchmarking Harness & Regression Detection

The `/benchmarks/` directory houses statistical Criterion benchmarks measuring:
1. **Startup Micro-Benchmarks:** Crate initialization, SQLite pool setup, Tantivy schema loading.
2. **Search Latency Benchmarks:** Full-text, regex, and structured queries under increasing corpus sizes.
3. **Throughput Benchmarks:** File traversal rate (files/sec), encryption/decryption throughput (MB/s), and SIMD scanning throughput (GB/s).
4. **Memory Profiling Harness:** Automated RSS measurement under synthetic load.

### Regression Threshold Enforcement
During CI execution:
- Criterion compares new results against the committed baseline in `benchmarks/baseline.json`.
- A statistically significant degradation $> 5\%$ in query latency or throughput triggers an immediate CI failure.
- A peak RSS exceeding $300\,\text{MB}$ under test conditions fails CI immediately.
