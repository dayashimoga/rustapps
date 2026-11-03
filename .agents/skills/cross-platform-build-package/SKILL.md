---
name: cross-platform-build-package
description: >-
  Use this skill when building, cross-compiling, packaging, or validating standalone
  installable artifacts for any of the 5 applications across Windows, Linux, macOS,
  Android, and iOS.
---

# Cross-Platform Standalone Build & Packaging Skill

Use this skill to build and package any of the 5 standalone applications (`ai-file-explorer`, `devops-secret-vault`, `universal-log-viewer`, `universal-live-translator`, `pro-av-editor`) without inter-app coupling.

## 1. Per-Application Standalone Build Verification

Each application must compile independently via `-p <app-crate-name>`:

```powershell
# Verify independent compilation of each app
cargo check -p universal-log-viewer --all-targets
cargo check -p devops-secret-vault --all-targets
cargo check -p ai-file-explorer --all-targets
cargo check -p universal-live-translator --all-targets
cargo check -p pro-av-editor --all-targets
```

## 2. Desktop Frontend + Tauri Bundle Build

For any desktop app (`apps/<app-name>`):

```powershell
# 1. Build frontend static bundle
Push-Location "apps/<app-name>/ui"
npm ci
npm run check
npm run build
Pop-Location

# 2. Build Rust release binary (embeds ../ui/dist/index.html, assets/app.js, assets/app.css)
cargo build -p <app-name> --release

# 3. Mandatory live GUI & IPC self-test verification
./target/release/<app-name> --self-test
```

## 3. Mobile FFI (`cdylib` / `staticlib`) & Flutter Packaging

For mobile-enabled apps (`apps/<app-name>/mobile`):

1. Compile the Rust core crate as `cdylib` (Android `.so`) and `staticlib` (iOS `.a` / `.xcframework`):
   - Android targets: `aarch64-linux-android`, `x86_64-linux-android`
   - iOS targets: `aarch64-apple-ios`, `aarch64-apple-ios-sim`
2. Run Flutter verification and build:
   ```powershell
   Push-Location "apps/<app-name>/mobile"
   flutter pub get
   flutter analyze
   flutter test
   # Android APK / AAB
   flutter build apk --release
   flutter build appbundle --release
   Pop-Location
   ```

## 4. Artifact Integrity & Signature Verification

Every generated installer/bundle in `dist/<app-name>/` must have:
1. `<artifact>.sha256` checksum file.
2. `<artifact>.sig` Ed25519 signature verified by `shared-updater`.
3. `<app-name>-sbom.cdx.json` CycloneDX SBOM.

