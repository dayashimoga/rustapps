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

---

## 6. Multi-Platform Build & Packaging Commands (ADR-0030)

Release packages are generated using dedicated, reproducible scripts in `/scripts/`:

### 6.1 Windows Build & Packaging
```powershell
# Build Windows x64 MSI & Portable ZIP
powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1 -Target x86_64-pc-windows-msvc -Version 0.1.0

# Build Windows ARM64 (Snapdragon X / Copilot+ PCs)
powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1 -Target aarch64-pc-windows-msvc -Version 0.1.0
```

### 6.2 Linux Build & Packaging
```bash
# Build Linux x64 (AppImage, DEB, tar.gz)
bash scripts/package-linux.sh x86_64-unknown-linux-gnu 0.1.0

# Build Linux ARM64 (using containerized cross-compilation)
bash scripts/package-linux.sh aarch64-unknown-linux-gnu 0.1.0
```

### 6.3 macOS Build & Packaging
```bash
# Build Universal Binary (Apple Silicon + Intel), .app bundle, and DMG
bash scripts/package-macos.sh 0.1.0
```

---

## 7. Containerized Cross-Compilation with Cross-rs

Cross-compilation for non-host Linux targets (`aarch64-unknown-linux-gnu`) is managed via `cross` and the root `Cross.toml` configuration:
```bash
# Install cross tool
cargo install cross --git https://github.com/cross-rs/cross

# Build release workspace inside hardened target container
cross build --workspace --release --target aarch64-unknown-linux-gnu
```
This produces clean, fully static GLIBC-compatible binaries runnable on any modern Linux ARM64 distribution without native ARM hardware or cross-toolchain installation on the host.

---

## 8. Mobile Vault Build & Deployment (Android & iOS)

The mobile client for DevOps Secret Vault resides in `apps/devops-secret-vault/mobile/` and builds against compiled native `vault_core` shared libraries.

### 8.1 Compile Rust Core for Mobile Architectures
```bash
# Add mobile targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
rustup target add aarch64-apple-ios x86_64-apple-ios

# Build Android native libraries
cargo build -p devops-secret-vault --release --target aarch64-linux-android
cargo build -p devops-secret-vault --release --target armv7-linux-androideabi

# Build iOS native framework
cargo build -p devops-secret-vault --release --target aarch64-apple-ios
```

### 8.2 Flutter Mobile Assembly
```bash
cd apps/devops-secret-vault/mobile

# Fetch Flutter dependencies
flutter pub get

# Run mobile tests
flutter test

# Build Android APK / App Bundle (AAB)
flutter build apk --release
flutter build appbundle --release

# Build iOS Archive (IPA)
flutter build ipa --release
```


