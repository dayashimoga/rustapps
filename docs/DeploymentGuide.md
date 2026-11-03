# Standalone Deployment & Packaging Guide

**Document Version:** 2.0.0  
**Status:** Approved  
**Classification:** Operational Deployment Standard  

---

## 1. Deployment Philosophy & Distribution Model

All five applications in the monorepo (`universal-log-viewer`, `devops-secret-vault`, `ai-file-explorer`, `universal-live-translator`, `pro-av-editor`) are engineered as **100% standalone, zero-dependency, offline-first executables**. They require no external runtimes, no local database servers, no web servers, and no container virtualization engines.

### Guiding Principles
- **Air-Gapped Ready:** Distributed as self-contained binaries runnable on completely disconnected systems.
- **Zero-Footprint Portable Mode:** Users can run the executables directly from a USB stick without installing system files or touching the system registry.
- **Cryptographic Verification:** Every release artifact is accompanied by a SHA-256 checksum and an Ed25519 detached signature.
- **Independent Application Delivery:** Applications are packaged, versioned, and released independently.

---

## 2. Platform Packaging Matrix

| Target OS | Architecture | Artifact Types | Tooling / Packager |
| :--- | :--- | :--- | :--- |
| **Windows** | `x86_64`, `aarch64` | `.exe` / `.zip` (Portable), `.msi` (WiX Installer) | Tauri v2 CLI / WiX Toolset / `package-windows.ps1` |
| **Linux** | `x86_64`, `aarch64` | `.AppImage` (Universal), `.deb`, `.tar.gz` | `appimagetool` / `dpkg-deb` / `package-linux.sh` |
| **macOS** | Universal (`x86_64`, `aarch64`) | Universal `.dmg`, `.app.tar.gz` | `lipo` / `hdiutil` / `package-macos.sh` |
| **Android** | `arm64-v8a`, `x86_64` | Signed `.apk`, Play Store `.aab` | Flutter 3.x + Android NDK / `package-mobile.sh` |
| **iOS** | `arm64`, `sim-arm64` | Signed `.ipa`, `.xcframework` | Flutter 3.x + Xcode Archive / `package-mobile.sh` |

---

## 3. Storage Paths & Data Persistence

Each application persists its local SQLite databases, indexes, and settings into platform-standard data directories:

| OS | Application Data Directory | Configuration Directory | Cache Directory |
| :--- | :--- | :--- | :--- |
| **Windows** | `%LOCALAPPDATA%\<app-name>\data` | `%APPDATA%\<app-name>\config` | `%LOCALAPPDATA%\<app-name>\cache` |
| **Linux** | `~/.local/share/<app-name>` | `~/.config/<app-name>` | `~/.cache/<app-name>` |
| **macOS** | `~/Library/Application Support/<app-name>` | `~/Library/Preferences/<app-name>` | `~/Library/Caches/<app-name>` |

### Portable Mode Activation
Placing an empty file named `.portable` or `portable.flag` adjacent to the executable directs the application to store all databases, settings, and indexes inside a `./data/` subfolder immediately alongside the executable.

---

## 4. Multi-Platform Build & Packaging Automation

Automated packaging scripts in `scripts/` support building any of the 5 applications independently:

### 4.1 Windows Build & Packaging (`scripts/package-windows.ps1`)
```powershell
# Package all applications or a specific application
.\scripts\package-windows.ps1 -AppName "universal-log-viewer"
.\scripts\package-windows.ps1 -AppName "devops-secret-vault"
.\scripts\package-windows.ps1 -AppName "ai-file-explorer"
.\scripts\package-windows.ps1 -AppName "universal-live-translator"
.\scripts\package-windows.ps1 -AppName "pro-av-editor"
```
Produces standalone portable ZIP bundles, WiX MSI definitions, and SHA-256 checksums in `dist/windows/`.

### 4.2 Linux Build & Packaging (`scripts/package-linux.sh`)
```bash
# Package tarball and AppImage
./scripts/package-linux.sh universal-log-viewer x86_64-unknown-linux-gnu 0.1.0
./scripts/package-linux.sh devops-secret-vault x86_64-unknown-linux-gnu 0.1.0
./scripts/package-linux.sh ai-file-explorer x86_64-unknown-linux-gnu 0.1.0
./scripts/package-linux.sh universal-live-translator x86_64-unknown-linux-gnu 0.1.0
./scripts/package-linux.sh pro-av-editor x86_64-unknown-linux-gnu 0.1.0
```

### 4.3 macOS Build & Packaging (`scripts/package-macos.sh`)
```bash
# Package universal binary .app and DMG
./scripts/package-macos.sh universal-log-viewer 0.1.0
./scripts/package-macos.sh devops-secret-vault 0.1.0
./scripts/package-macos.sh ai-file-explorer 0.1.0
./scripts/package-macos.sh universal-live-translator 0.1.0
./scripts/package-macos.sh pro-av-editor 0.1.0
```

### 4.4 Mobile Build & Packaging (`scripts/package-mobile.sh`)
```bash
# Package Android APK/AAB and iOS XCFramework
./scripts/package-mobile.sh universal-log-viewer
./scripts/package-mobile.sh devops-secret-vault
./scripts/package-mobile.sh ai-file-explorer
./scripts/package-mobile.sh universal-live-translator
./scripts/package-mobile.sh pro-av-editor
```

---

## 5. Air-Gapped Verification Procedure

1. **Verify Checksum:**
   ```bash
   sha256sum -c <package-name>.sha256
   ```
2. **Verify Cryptographic Signature:**
   ```bash
   shared-updater verify --public-key keys/release.pub --file <package-name> --signature <package-name>.sig
   ```
3. **Air-Gap Transfer:** Copy the verified package to approved optical media or encrypted storage. Zero network requests will ever be initiated during execution.
