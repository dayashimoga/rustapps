#!/usr/bin/env bash
# Linux Packaging Automation Script (ADR-0030)
# Packages DevOps Secret Vault, AI File Explorer, Universal Log Viewer, Universal Live Translator, and Pro AV Editor for Linux (x64 / ARM64)
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
VERSION="${2:-0.1.0}"
OUTPUT_DIR="${3:-dist/linux}"

echo "=========================================================="
echo "  Packaging Monorepo Apps for Linux (${TARGET}) v${VERSION}"
echo "=========================================================="

APPS=("devops-secret-vault" "ai-file-explorer" "universal-log-viewer" "universal-live-translator" "pro-av-editor")
mkdir -p "${OUTPUT_DIR}"

# 1. Compile Release Binaries
echo "[1/5] Building release binaries for target: ${TARGET}..."
if [ "${TARGET}" = "aarch64-unknown-linux-gnu" ] && command -v cross &>/dev/null; then
    cross build --workspace --release --target "${TARGET}"
else
    cargo build --workspace --release --target "${TARGET}"
fi

# 2. Package Standalone Tarball Bundles
echo "[2/5] Creating tar.gz standalone distribution archives..."
for app in "${APPS[@]}"; do
    src_bin="target/${TARGET}/release/${app}"
    if [ -f "${src_bin}" ]; then
        stage_dir="${OUTPUT_DIR}/stage/${app}"
        mkdir -p "${stage_dir}"
        cp "${src_bin}" "${stage_dir}/${app}"
        chmod +x "${stage_dir}/${app}"
        touch "${stage_dir}/portable.flag"
        [ -f "LICENSE" ] && cp "LICENSE" "${stage_dir}/"
        [ -f "README.md" ] && cp "README.md" "${stage_dir}/"

        tar_name="${app}_${VERSION}_${TARGET}.tar.gz"
        tar -czf "${OUTPUT_DIR}/${tar_name}" -C "${OUTPUT_DIR}/stage" "${app}"
        echo "  -> Created tarball: ${tar_name}"
        rm -rf "${OUTPUT_DIR}/stage"
    fi
done

# 3. Assemble AppImage Bundle Hierarchy
echo "[3/5] Generating AppImage bundle structure..."
for app in "${APPS[@]}"; do
    appdir="${OUTPUT_DIR}/${app}.AppDir"
    mkdir -p "${appdir}/usr/bin"
    mkdir -p "${appdir}/usr/share/icons/hicolor/256x256/apps"

    src_bin="target/${TARGET}/release/${app}"
    if [ -f "${src_bin}" ]; then
        cp "${src_bin}" "${appdir}/usr/bin/${app}"
    fi

    # Create standard desktop file
    cat > "${appdir}/${app}.desktop" <<EOF
[Desktop Entry]
Name=${app}
Exec=${app}
Icon=${app}
Type=Application
Categories=Utility;Development;
EOF

    # Create AppRun entrypoint script
    cat > "${appdir}/AppRun" <<EOF
#!/bin/sh
SELF=\$(readlink -f "\$0")
HERE=\$(dirname "\$SELF")
export PATH="\${HERE}/usr/bin:\${PATH}"
exec "\${HERE}/usr/bin/${app}" "\$@"
EOF
    chmod +x "${appdir}/AppRun"
    echo "  -> Generated AppImage directory: ${appdir}"
done

# 4. Generate Debian (.deb) Package Hierarchy
echo "[4/5] Generating Debian package structures..."
for app in "${APPS[@]}"; do
    deb_dir="${OUTPUT_DIR}/${app}_${VERSION}_amd64"
    mkdir -p "${deb_dir}/DEBIAN"
    mkdir -p "${deb_dir}/usr/bin"

    cat > "${deb_dir}/DEBIAN/control" <<EOF
Package: ${app}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Offline Apps Engineering <maintainers@offlineapps.local>
Description: Ultra-fast, offline-first production-grade application: ${app}
EOF

    src_bin="target/${TARGET}/release/${app}"
    if [ -f "${src_bin}" ]; then
        cp "${src_bin}" "${deb_dir}/usr/bin/${app}"
        chmod 755 "${deb_dir}/usr/bin/${app}"
    fi
    echo "  -> Scaffolded Debian package layout: ${deb_dir}"
done

# 5. Compute Cryptographic Checksums (SHA-256)
echo "[5/5] Computing SHA-256 cryptographic checksums..."
cd "${OUTPUT_DIR}"
for file in *.tar.gz; do
    if [ -f "${file}" ]; then
        sha256sum "${file}" > "${file}.sha256"
        echo "  -> Checksum generated for: ${file}"
    fi
done

echo "Linux packaging completed successfully!"
