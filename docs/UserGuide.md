# End-User Manual & Operational Guide

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Public User Documentation  

---

## 1. Introduction & The Offline Guarantee

Welcome to the next generation of developer utilities. This suite comprises three high-performance, lightweight, standalone applications:
1. **DevOps Secret Vault:** Secure credential management with biometric authentication.
2. **AI File Explorer:** Blazing fast local file cataloging, deduplication, and semantic search.
3. **Universal Log Viewer:** Instant inspection and pattern mining for massive multi-gigabyte log files.

### 🛡️ The 100% Offline Guarantee
- **No Cloud:** All data remains strictly on your local machine.
- **No Analytics:** No tracking, phone-home telemetry, or external pings.
- **No Internet Required:** Works completely in air-gapped environments, on airplanes, or inside secure enclaves.
- **No Subscription:** Open source and free forever.

---

## 2. Universal Log Viewer Guide

### 2.1 Opening Massive Log Files
1. Launch **Universal Log Viewer**.
2. Drag and drop any log file (from 1KB up to 500GB) onto the window, or click **Open File** (`Ctrl+O` / `Cmd+O`).
3. The file opens **instantaneously**. Thanks to zero-copy memory mapping, 100GB files are indexed in under 2 seconds, consuming less than 250MB of RAM.

### 2.2 Search and Real-Time Filtering
- **Instant Search:** Press `Ctrl+F` and type your query. Filter results appear in under 50ms.
- **Regular Expressions:** Toggle the `.*` button in the search bar to activate compiled regex mode.
- **Structured Fields:** Query JSON fields directly using dot notation (e.g., `level:error service:auth`).

### 2.3 Live Tail Mode
- Click the **Tail Mode** toggle (`Ctrl+T`).
- The viewer continuously follows newly appended lines in real-time at native disk write speeds without locking or freezing the interface.

### 2.4 Timeline & Anomaly Detection
- Open the bottom drawer to view the **Severity Histogram**.
- Click on anomaly spikes to jump directly to correlated error clusters.

---

## 3. DevOps Secret Vault Guide

### 3.1 Creating Your Vault & Biometric Pairing
1. On initial launch, set a strong Master Passphrase.
2. When prompted, enable **Biometric Authentication** (Windows Hello, Touch ID, or Linux Secret Service).
3. The vault derives an encryption key using Argon2id and initializes an encrypted SQLCipher container.

### 3.2 Managing Secrets & TOTP Tokens
- **Adding Items:** Click **+ New Secret** (`Ctrl+N`). Select from standard templates: Password, SSH Key, Certificate, API Key, Database Connection, AWS/Azure Credentials.
- **Secure Copy:** Click the copy icon next to any secret. The secret is copied to the clipboard and automatically erased after 30 seconds.
- **TOTP Generation:** Add a TOTP secret key or scan a QR code; live 6-digit codes update automatically every 30 seconds offline.

### 3.3 Certificate Expiration Monitoring
- Import SSL/TLS or SSH certificates.
- The vault automatically parses validity dates, alerting you 30, 14, and 7 days prior to expiration.

---

## 4. AI File Explorer Guide

### 4.1 Indexing & Instant Search
- Select target drives or folders to index. The background crawler indexes over 100,000 files per second.
- Type in the omnibar (`Ctrl+K`) to locate files across millions of entries in under 50ms.

### 4.2 Finding Duplicate Files
1. Navigate to the **Deduplication** tab.
2. Select target directories and click **Scan for Duplicates**.
3. The engine uses size grouping and 4KB header/footer hashing before cryptographic BLAKE3 verification, saving massive disk I/O.
4. Review duplicate sets and select files for safe deletion or hardlinking.

### 4.3 Disk Usage Treemap & Storage Analytics
- Switch to the **Disk Treemap** view to visualize folder hierarchies proportionally.
- Click any block to drill down into largest space consumers.

### 4.4 Transactional Batch Rename
- Select multiple files and press `F2`.
- Apply regex replacement, numbering, or timestamping.
- Preview changes in real time. Click **Apply**; all renames execute atomically. If any error occurs, the rollback journal restores previous filenames instantly.

---

## 5. Global Keyboard Shortcuts

| Shortcut (Win/Linux) | Shortcut (macOS) | Action |
| :--- | :--- | :--- |
| `Ctrl+O` | `Cmd+O` | Open file / workspace |
| `Ctrl+F` | `Cmd+F` | Focus search bar |
| `Ctrl+T` | `Cmd+T` | Toggle live tail mode |
| `Ctrl+K` | `Cmd+K` | Open global command palette |
| `Ctrl+N` | `Cmd+N` | Create new entry |
| `Ctrl+B` | `Cmd+B` | Toggle sidebar / bookmarks |
| `Ctrl+,` | `Cmd+,` | Open application settings |
