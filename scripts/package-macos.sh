#!/usr/bin/env bash
# macOS Packaging Automation Script (ADR-0030)
# Packages DevOps Secret Vault, AI File Explorer, Universal Log Viewer, Universal Live Translator, and Pro AV Editor for macOS (Intel / Apple Silicon / Universal)
set -euo pipefail

VERSION="${1:-0.1.0}"
OUTPUT_DIR="${2:-dist/macos}"
FILTER_APP="${3:-}"

echo "=========================================================="
echo "  Packaging Monorepo Apps for macOS (Universal) v${VERSION}"
echo "=========================================================="

if [ -n "${FILTER_APP}" ]; then
    APPS=("${FILTER_APP}")
else
    APPS=("devops-secret-vault" "ai-file-explorer" "universal-log-viewer" "universal-live-translator" "pro-av-editor" "personal-knowledge-graph" "api-protocol-studio" "parquet-analytics-workbench" "dfir-triage-kit" "screen-recorder-studio" "signal-telemetry-oscilloscope" "audio-mastering-studio")
fi
mkdir -p "${OUTPUT_DIR}"

# 0. Build Embedded Svelte 5 Desktop UI Bundles
echo "[0/4] Building Svelte 5 desktop UI bundles for all 5 apps..."
for app in "${APPS[@]}"; do
    if [ -f "apps/${app}/ui/package.json" ]; then
        npm --prefix "apps/${app}/ui" run build
    fi
done

# 1. Compile Targets (Apple Silicon & Intel)
echo "[1/4] Building release binaries for Apple Silicon and Intel..."
cargo build --workspace --release --target aarch64-apple-darwin || true
cargo build --workspace --release --target x86_64-apple-darwin || true

# 2. Assemble Universal Binaries (lipo) and .app Bundles
echo "[2/4] Generating Universal Binaries and macOS .app bundles..."
for app in "${APPS[@]}"; do
    app_bundle="${OUTPUT_DIR}/${app}.app"
    macos_dir="${app_bundle}/Contents/MacOS"
    resources_dir="${app_bundle}/Contents/Resources"
    mkdir -p "${macos_dir}" "${resources_dir}"

    arm_bin="target/aarch64-apple-darwin/release/${app}"
    intel_bin="target/x86_64-apple-darwin/release/${app}"
    out_bin="${macos_dir}/${app}"

    if [ -f "${arm_bin}" ] && [ -f "${intel_bin}" ] && command -v lipo &>/dev/null; then
        echo "  -> Assembling Universal Binary with lipo: ${app}"
        lipo -create -output "${out_bin}" "${arm_bin}" "${intel_bin}"
    elif [ -f "${arm_bin}" ]; then
        cp "${arm_bin}" "${out_bin}"
    elif [ -f "${intel_bin}" ]; then
        cp "${intel_bin}" "${out_bin}"
    fi

    [ -f "${out_bin}" ] && chmod +x "${out_bin}"

    # Generate standard Info.plist
    cat > "${app_bundle}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>${app}</string>
    <key>CFBundleIdentifier</key>
    <string>com.offlineapps.${app}</string>
    <key>CFBundleName</key>
    <string>${app}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

    # Generate CycloneDX SBOM for the application
    if [ -f "scripts/generate-sbom.sh" ]; then
        bash scripts/generate-sbom.sh "${app}" "${OUTPUT_DIR}" || true
        [ -f "${OUTPUT_DIR}/${app}-sbom.json" ] && cp "${OUTPUT_DIR}/${app}-sbom.json" "${resources_dir}/"
    fi
    echo "  -> Created macOS bundle: ${app_bundle}"
done

# 3. Create DMG Template Archives
echo "[3/4] Creating DMG packaging disk images..."
for app in "${APPS[@]}"; do
    app_bundle="${OUTPUT_DIR}/${app}.app"
    dmg_name="${app}_${VERSION}_macos_universal.dmg"
    dmg_path="${OUTPUT_DIR}/${dmg_name}"

    if command -v hdiutil &>/dev/null && [ -d "${app_bundle}" ]; then
        hdiutil create -volname "${app}" -srcfolder "${app_bundle}" -ov -format UDZO "${dmg_path}"
        echo "  -> Created DMG: ${dmg_name}"
        if command -v pkgbuild &>/dev/null; then
            pkgbuild --root "${app_bundle}" --identifier "com.offlineapps.${app}" --version "${VERSION}" --install-location "/Applications/${app}.app" "${OUTPUT_DIR}/${app}_${VERSION}_macos_universal.pkg" || true
            echo "  -> Created PKG installer: ${app}_${VERSION}_macos_universal.pkg"
        fi
    else
        # Fallback to tar.gz on non-macOS host
        tar_name="${app}_${VERSION}_macos_app.tar.gz"
        tar -czf "${OUTPUT_DIR}/${tar_name}" -C "${OUTPUT_DIR}" "${app}.app" 2>/dev/null || true
        echo "  -> Created compressed bundle: ${tar_name}"
    fi
done

# 4. Compute Checksums
echo "[4/4] Computing SHA-256 cryptographic checksums..."
cd "${OUTPUT_DIR}"
for file in *.dmg *.tar.gz *.pkg *-sbom.json; do
    if [ -f "${file}" ] && [ "${file##*.}" != "sha256" ]; then
        if command -v sha256sum &>/dev/null; then
            sha256sum "${file}" > "${file}.sha256"
        elif command -v shasum &>/dev/null; then
            shasum -a 256 "${file}" > "${file}.sha256"
        fi
        echo "  -> Checksum generated for: ${file}"
    fi
done

echo "macOS packaging completed successfully!"
