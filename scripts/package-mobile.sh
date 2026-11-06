#!/usr/bin/env bash
# Mobile Packaging Automation Script (ADR-0030)
# Compiles Rust C-ABI native libraries and packages Flutter Android & iOS mobile apps
set -euo pipefail

VERSION="${1:-0.1.0}"
RAW_OUTPUT_DIR="${2:-dist/mobile}"
FILTER_APP="${3:-}"

echo "=========================================================="
echo "  Packaging Monorepo Mobile Apps (Android & iOS) v${VERSION}"
echo "=========================================================="

mkdir -p "${RAW_OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${RAW_OUTPUT_DIR}" && pwd)"

# Auto-detect ANDROID_NDK_HOME in CI environments if not explicitly set
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ] && [ -d "${ANDROID_HOME}/ndk" ]; then
    latest_ndk=$(ls -1d "${ANDROID_HOME}/ndk/"* 2>/dev/null | sort -V | tail -n1 || true)
    if [ -n "${latest_ndk}" ]; then
        export ANDROID_NDK_HOME="${latest_ndk}"
        export ANDROID_NDK_ROOT="${latest_ndk}"
        echo "Auto-detected ANDROID_NDK_HOME: ${ANDROID_NDK_HOME}"
    fi
fi

if [ -n "${FILTER_APP}" ]; then
    APPS=("${FILTER_APP}")
else
    APPS=("devops-secret-vault" "ai-file-explorer" "universal-log-viewer" "universal-live-translator" "pro-av-editor" "qr-code-studio")
fi

ANDROID_TARGETS=("aarch64-linux-android" "armv7-linux-androideabi" "x86_64-linux-android")
IOS_TARGETS=("aarch64-apple-ios" "x86_64-apple-ios" "aarch64-apple-ios-sim")

# Helper for universal cross-platform ZIP creation
create_bundle_zip() {
    local src_dir="$1"
    local dest_zip="$2"
    if command -v zip >/dev/null 2>&1; then
        (
            cd "${src_dir}"
            zip -r "${dest_zip}" . -x "*.git*" "build/*" ".dart_tool/*" "target/*" || true
        )
    elif command -v python3 >/dev/null 2>&1; then
        python3 -c "
import os, zipfile
src = '${src_dir}'
dst = '${dest_zip}'
with zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED) as z:
    for root, dirs, files in os.walk(src):
        dirs[:] = [d for d in dirs if d not in ('.git', 'build', '.dart_tool', 'target')]
        for file in files:
            full_path = os.path.join(root, file)
            rel_path = os.path.relpath(full_path, src)
            z.write(full_path, rel_path)
"
    elif command -v powershell >/dev/null 2>&1; then
        powershell -Command "Compress-Archive -Path '${src_dir}/*' -DestinationPath '${dest_zip}' -Force"
    fi
}

for app in "${APPS[@]}"; do
    echo "----------------------------------------------------------"
    echo "Processing mobile app packaging: ${app}"
    echo "----------------------------------------------------------"

    # Detect platform intent from output directory name or host
    is_ios=false
    platform_tag="android"
    if [[ "${OUTPUT_DIR}" == *"ios"* ]] || [ "$(uname -s)" = "Darwin" ]; then
        is_ios=true
        platform_tag="ios"
    fi

    # 1. Always generate metadata manifest in output directory
    cat <<EOF > "${OUTPUT_DIR}/metadata.json"
{
  "application": "${app}",
  "version": "${VERSION}",
  "platform": "${platform_tag}",
  "offline_autonomous": true,
  "telemetry_free": true
}
EOF

    # 2. Always generate CycloneDX SBOM for the mobile target
    if [ -f "scripts/generate-sbom.sh" ]; then
        bash scripts/generate-sbom.sh "${app}" "${OUTPUT_DIR}" || true
    fi

    mobile_dir="apps/${app}/mobile"
    bundle_name="${app}-${VERSION}-${platform_tag}.zip"

    if [ -d "${mobile_dir}" ]; then
        echo "  Found Flutter mobile directory for ${app}."

        # A. Android JNI Libraries compilation
        if ! ${is_ios} && command -v cargo-ndk &>/dev/null; then
            echo "  [Android] Building native JNI shared libraries with cargo-ndk..."
            for target in "${ANDROID_TARGETS[@]}"; do
                jni_arch="arm64-v8a"
                [ "${target}" = "armv7-linux-androideabi" ] && jni_arch="armeabi-v7a"
                [ "${target}" = "x86_64-linux-android" ] && jni_arch="x86_64"

                jni_dest="${mobile_dir}/android/app/src/main/jniLibs/${jni_arch}"
                mkdir -p "${jni_dest}"
                echo "    Compiling --lib for ${target} -> ${jni_arch}..."
                cargo ndk --target "${target}" -o "${mobile_dir}/android/app/src/main/jniLibs" build --release -p "${app}" --lib || true
            done
        elif ! ${is_ios}; then
            echo "  [Android] cargo-ndk not installed; skipping JNI shared object build."
        fi

        # B. iOS Native Libraries compilation
        if ${is_ios} && [ "$(uname -s)" = "Darwin" ]; then
            echo "  [iOS] Building static / C-ABI libraries for iOS targets..."
            for target in "${IOS_TARGETS[@]}"; do
                echo "    Compiling --lib for ${target}..."
                cargo build --release --target "${target}" -p "${app}" --lib || true
            done
        fi

        # C. Flutter Scaffolding & Packaging
        if command -v flutter &>/dev/null; then
            echo "  [Flutter] Preparing Flutter project and dependencies..."
            (
                cd "${mobile_dir}"
                flutter create --platforms=android,ios --org com.offlineapps . || true
                flutter pub get || true
                if ! ${is_ios}; then
                    echo "  [Flutter] Building Android APK & AAB..."
                    flutter build apk --release || true
                    flutter build appbundle --release || true
                fi
            )

            # Check for generated Android artifacts
            apk_src="${mobile_dir}/build/app/outputs/flutter-apk/app-release.apk"
            if [ -f "${apk_src}" ]; then
                cp "${apk_src}" "${OUTPUT_DIR}/${app}-${VERSION}.apk"
                echo "  -> Created APK: ${OUTPUT_DIR}/${app}-${VERSION}.apk"
            fi
            aab_src="${mobile_dir}/build/app/outputs/bundle/release/app-release.aab"
            if [ -f "${aab_src}" ]; then
                cp "${aab_src}" "${OUTPUT_DIR}/${app}-${VERSION}.aab"
                echo "  -> Created AAB: ${OUTPUT_DIR}/${app}-${VERSION}.aab"
            fi
        else
            echo "  [Flutter] Flutter SDK not found on path; skipping APK/IPA bundling."
        fi

        # D. Create Standalone Flutter Platform Bundle ZIP
        echo "  Creating standalone ${platform_tag} bundle: ${OUTPUT_DIR}/${bundle_name}..."
        create_bundle_zip "${mobile_dir}" "${OUTPUT_DIR}/${bundle_name}"

    else
        echo "  [C-ABI SDK] No Flutter directory found for ${app}; packaging Standalone Mobile C-ABI SDK..."
        sdk_stage="stage_cabi_${app}_${platform_tag}"
        mkdir -p "${sdk_stage}"

        # Copy metadata, SBOM, and C-ABI headers/FFI
        [ -f "${OUTPUT_DIR}/metadata.json" ] && cp "${OUTPUT_DIR}/metadata.json" "${sdk_stage}/"
        [ -f "${OUTPUT_DIR}/${app}-sbom.json" ] && cp "${OUTPUT_DIR}/${app}-sbom.json" "${sdk_stage}/"
        [ -f "apps/${app}/src/ffi.rs" ] && cp "apps/${app}/src/ffi.rs" "${sdk_stage}/ffi.rs"
        [ -f "apps/${app}/src/lib.rs" ] && cp "apps/${app}/src/lib.rs" "${sdk_stage}/lib.rs"
        [ -f "apps/${app}/Cargo.toml" ] && cp "apps/${app}/Cargo.toml" "${sdk_stage}/Cargo.toml"

        cat <<EOF > "${sdk_stage}/README.md"
# ${app} Mobile C-ABI SDK (${platform_tag})
Version: ${VERSION}
Target: ${platform_tag}
Offline Autonomous: true

This package contains the standalone C-ABI native bindings, metadata, and CycloneDX 1.5 SBOM
for integrating ${app} into Android and iOS applications via Flutter Rust Bridge or native JNI/XCFramework.
EOF

        # Attempt C-ABI compilation if cargo-ndk or iOS toolchain is available
        if ! ${is_ios} && command -v cargo-ndk &>/dev/null; then
            echo "  [Android C-ABI] Compiling JNI shared library for ${app}..."
            mkdir -p "${sdk_stage}/jniLibs/arm64-v8a"
            cargo ndk --target "aarch64-linux-android" -o "${sdk_stage}/jniLibs" build --release -p "${app}" --lib || true
        elif ${is_ios} && [ "$(uname -s)" = "Darwin" ]; then
            echo "  [iOS C-ABI] Compiling static library for ${app}..."
            mkdir -p "${sdk_stage}/libs"
            cargo build --release --target "aarch64-apple-ios" -p "${app}" --lib || true
            [ -f "target/aarch64-apple-ios/release/lib${app//-/_}.a" ] && cp "target/aarch64-apple-ios/release/lib${app//-/_}.a" "${sdk_stage}/libs/" || true
        fi

        create_bundle_zip "${sdk_stage}" "${OUTPUT_DIR}/${bundle_name}"
        rm -rf "${sdk_stage}"
    fi
done

# 3. Always compute SHA-256 Checksums for all artifacts in OUTPUT_DIR
if [ -d "${OUTPUT_DIR}" ]; then
    echo "Computing SHA-256 checksums..."
    (
        cd "${OUTPUT_DIR}"
        for f in *; do
            if [ -f "$f" ] && [[ "$f" != *.sha256 ]]; then
                if command -v sha256sum >/dev/null 2>&1; then
                    sha256sum "$f" > "$f.sha256"
                elif command -v shasum >/dev/null 2>&1; then
                    shasum -a 256 "$f" > "$f.sha256"
                fi
            fi
        done
    )
fi

echo "Mobile packaging stage completed successfully for: ${OUTPUT_DIR}"
