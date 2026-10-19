# Standalone Deployment & Packaging Guide

**Document Version:** 1.0.0  
**Status:** Approved  
**Classification:** Operational Deployment Standard  

---

## 1. Deployment Philosophy & Distribution Model

All three applications in the monorepo are engineered as **100% standalone, zero-dependency, offline-first executables**. They require no external runtimes, no local database engines, no web servers, and no container virtualization platforms (Docker, Podman).

### Guiding Principles
- **Air-Gapped Ready:** Distributed as self-contained binaries runnable on completely disconnected systems.
- **Zero-Footprint Portable Mode:** Users can run the executables directly from a USB stick without installing system files or touching the system registry.
- **Cryptographic Verification:** Every release artifact is accompanied by a SHA-256 checksum and an Ed25519 detached cryptographic signature.

---

## 2. Platform Packaging Matrix

| Target OS | Architecture | Artifact Types | Tooling / Packager |
| :--- | :--- | :--- | :--- |
| **Windows** | `x86_64`, `aarch64` | `.exe` (Portable), `.msi` (Enterprise Installer) | Tauri v2 CLI / WiX Toolset |
| **Linux** | `x86_64`, `aarch64` | `.AppImage` (Universal), `.deb` (Debian/Ubuntu), `.rpm` (RHEL/Fedora) | Tauri v2 Bundler, `cargo-appimage` |
| **macOS** | `x86_64`, `aarch64` | `.dmg` (Drag-and-Drop Disk Image), `.tar.gz` | Tauri v2 Bundler (Universal Binary) |
| **Android** | `aarch64` (Phase 8)| `.apk` (Sideloadable), `.aab` (App Bundle) | Flutter Build Toolchain + NDK |
| **iOS** | `aarch64` (Phase 8)| `.ipa` | Flutter / Xcode Archive |

---

## 3. Storage Paths & Data Persistence

Each application persists its local SQLite/SQLCipher databases, indexes, and settings into platform-standard data directories:

| OS | Application Data Directory | Configuration Directory | Cache Directory |
| :--- | :--- | :--- | :--- |
| **Windows** | `%LOCALAPPDATA%\<app-name>\data` | `%APPDATA%\<app-name>\config` | `%LOCALAPPDATA%\<app-name>\cache` |
| **Linux** | `~/.local/share/<app-name>` | `~/.config/<app-name>` | `~/.cache/<app-name>` |
| **macOS** | `~/Library/Application Support/<app-name>` | `~/Library/Preferences/<app-name>` | `~/Library/Caches/<app-name>` |

### Portable Mode Activation
Placing an empty file named `.portable` or `portable.flag` adjacent to the executable directs the application to store all databases, settings, and indexes inside a `./data/` subfolder immediately alongside the executable.

---

## 4. Air-Gapped / Disconnected Installation Procedure

1. **Download Release Bundle:** Download the application package, `.sha256` checksum file, and `.sig` Ed25519 signature file on an internet-connected workstation.
2. **Verify Checksum:**
   ```bash
   sha256sum -c universal-log-viewer-v1.0.0-x86_64-pc-windows-msvc.msi.sha256
   ```
3. **Verify Cryptographic Signature:**
   ```bash
   shared-updater verify --public-key keys/release.pub --file universal-log-viewer-v1.0.0.msi --signature universal-log-viewer-v1.0.0.msi.sig
   ```
4. **Air-Gap Transfer:** Copy the verified package to approved optical media or an encrypted USB drive.
5. **Execution:** Transfer to the air-gapped target system and execute or install. Zero network connectivity or post-install package downloads will be attempted.

---

## 5. Clean Uninstallation & Data Scrubbing

To perform a clean uninstallation with zero remnants:
1. **Windows:** Uninstall via Settings / Control Panel, then remove `%LOCALAPPDATA%\<app-name>`.
2. **Linux:** Remove the executable / package (`dpkg -r <app-name>` or `rpm -e <app-name>`), then remove `~/.local/share/<app-name>` and `~/.config/<app-name>`.
3. **macOS:** Drag the application from `/Applications` to Trash, then delete `~/Library/Application Support/<app-name>`.
