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

### 2.4 Chronological Timeline & Density Filtering
- Toggle the **Timeline** bar in the top navigation.
- View real-time stacked volume histograms colored by severity (Fatal, Error, Warn, Info).
- Click and drag directly on any time slice to filter the log stream to that specific time window in sub-50ms.
- Click **Clear Filter** to reset the window.

### 2.5 AI Root-Cause Intelligence & Cloud Importers
- **AI Insights Panel:** Press `Ctrl+I` or click **AI Insights** to open the root-cause intelligence drawer.
- **Log Template Mining:** The on-device Drain algorithm extracts recurring structural patterns, masking IPs, UUIDs, hex hashes, and numbers with `<*>` wildcards to group millions of lines into clean templates.
- **Statistical Anomaly Spikes:** Identifies sudden deviations in error density or volume surges (z-scores $> 2.5$).
- **Offline CloudWatch & Kubernetes Normalization:** Drag and drop exported CloudWatch JSON dumps or `/var/log/pods/` directory archives. Universal Log Viewer extracts namespace, pod, and container metadata and automatically correlates distributed error cascades.


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
- Weak cryptographic algorithms (MD5, SHA-1 signatures, RSA keys < 2048 bits) are automatically flagged with warning banners.

### 3.4 Security Posture Health Dashboard
- Click **Health Dashboard** in the navigation header to view your overall security posture.
- An interactive radial dial displays your overall score (0–100) and letter grade (A+ through F).
- Review 5 distinct health dimensions: Password Entropy, Zero-Knowledge Credential Reuse, Rotation Age (>90d, >180d), Default Password exposure, and Certificate Hygiene.
- Click **Jump to Secret** on any prioritized recommendation to remediate high-risk credentials directly.

### 3.5 Tamper-Evident Encrypted Backup & Restore
- Click **Backup & Restore** in the top navigation.
- **Export:** Enter a strong backup passphrase to generate an encrypted `.vaultbackup` bundle. All secret payloads and metadata are encrypted with Argon2id + AES-256-GCM and verified with an embedded SHA-256 integrity checksum.
- **Restore:** Select a `.vaultbackup` archive and input the passphrase. The system verifies the cryptographic signature and integrity checksum before applying updates, with optional overwrite protection.

---

## 4. AI File Explorer Guide

### 4.1 Indexing & Instant Search
- Navigate to the **Explorer** tab to browse folders or select any directory.
- Type in the omnibar (`Ctrl+F` or click the search box) to locate files across millions of indexed files in under 50ms.
- Results update live as you type, filtering on filename, extension, path, size, or modification date.
- Real-time filesystem watcher updates directory listings dynamically when files are added, modified, or deleted externally.

### 4.2 Finding & Cleaning Duplicate Files (3-Tier Engine)
1. Switch to the **Duplicates** view (`Duplicates` tab).
2. Enter or select the target directory and click **Scan for Duplicates**.
3. The deterministic 3-tier engine executes:
   - **Tier 1:** Exact file size grouping (eliminates unique files instantly with 0 disk reads).
   - **Tier 2:** 4KB header and 4KB footer boundary hashing (eliminates size collisions).
   - **Tier 3:** Full BLAKE3 cryptographic hash verification on surviving candidates.
4. Duplicate clusters display total wasted space, exact hashes, and all redundant copies.
5. Click **Select Redundant Copies** to auto-select all duplicate copies while preserving original files, or manually toggle files to delete.

### 4.3 Disk Usage Treemap & Storage Analytics
- Switch to the **Treemap** view (`Treemap` tab).
- An interactive squarified treemap renders storage hierarchies on an HTML5 hardware-accelerated canvas.
- Hover over any block to inspect exact folder paths, recursive byte sizes, and file counts.
- Click any directory block to drill down into subdirectories; click the parent navigation or breadcrumbs to return upward.
- Click the **Stats** toggle button (`Ctrl+S`) in the header to open the slide-over **Storage Breakdown Drawer**, viewing mime-type and extension distribution progress bars alongside the top largest files.

### 4.4 Transactional Batch Rename with Rollback Journal
1. In the Explorer view, select one or more files using checkboxes and click **Batch Rename** (or press `F2`).
2. Configure renaming transformations:
   - **Find & Replace:** Plain string or compiled regular expression matching.
   - **Prefix / Suffix:** Prepend or append text tokens.
   - **Sequential Numbering:** Specify starting number and zero-padding digits (`01, 02, ...`).
   - **Letter Casing:** Transform filenames to Lowercase, Uppercase, Title Case, or Camel Case.
3. Review the **Live Dry-Run Preview table**: highlights prospective names, detects collisions, and validates changes before touching disk.
4. Click **Apply Rename**: transactions execute atomically. If any error occurs or disk permissions fail midway, the reverse-order rollback journal automatically restores all previous filenames with zero data loss.
5. If you change your mind after applying, click **Undo Batch Rename** to immediately restore previous names.

---

## 5. Global Keyboard Shortcuts

| Shortcut (Win/Linux) | Shortcut (macOS) | Action | Scope |
| :--- | :--- | :--- | :--- |
| `Ctrl+O` | `Cmd+O` | Open file / workspace | Log Viewer / Explorer |
| `Ctrl+F` | `Cmd+F` | Focus search bar / Omnibar | All Apps |
| `Ctrl+T` | `Cmd+T` | Toggle live tail mode | Universal Log Viewer |
| `Ctrl+I` | `Cmd+I` | Toggle AI insights drawer | Universal Log Viewer |
| `Ctrl+K` | `Cmd+K` | Open global command palette | All Apps |
| `Ctrl+N` | `Cmd+N` | Create new secret / folder | DevOps Secret Vault |
| `F2` | `F2` | Open Batch Rename modal | AI File Explorer |
| `Ctrl+S` | `Cmd+S` | Toggle Storage Stats drawer | AI File Explorer |
| `Ctrl+B` | `Cmd+B` | Toggle sidebar / bookmarks | All Apps |
| `Ctrl+,` | `Cmd+,` | Open application settings | All Apps |

