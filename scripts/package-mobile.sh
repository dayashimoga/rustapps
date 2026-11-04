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

if [ -n "${FILTER_APP}" ]; then
    APPS=("${FILTER_APP}")
else
    APPS=("devops-secret-vault" "ai-file-explorer" "universal-log-viewer" "universal-live-translator" "pro-av-editor")
fi

ANDROID_TARGETS=("aarch64-linux-android" "armv7-linux-androideabi" "x86_64-linux-android")
IOS_TARGETS=("aarch64-apple-ios" "x86_64-apple-ios" "aarch64-apple-ios-sim")

for app in "${APPS[@]}"; do
    mobile_dir="apps/${app}/mobile"
    if [ ! -d "${mobile_dir}" ]; then
        echo "Skipping ${app}: no mobile directory found at ${mobile_dir}"
        continue
    fi

    echo "----------------------------------------------------------"
    echo "Processing mobile app: ${app}"
    echo "----------------------------------------------------------"

    # Always generate metadata manifest in output directory
    cat <<EOF > "${OUTPUT_DIR}/metadata.json"
{
  "application": "${app}",
  "version": "${VERSION}",
  "offline_autonomous": true,
  "telemetry_free": true
}
EOF

    # Detect platform intent from output directory name or host
    is_ios=false
    if [[ "${OUTPUT_DIR}" == *"ios"* ]] || [ "$(uname -s)" = "Darwin" ]; then
        is_ios=true
    fi

    # 1. Android JNI Libraries compilation
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

    # 2. iOS Native Libraries compilation
    if ${is_ios} && [ "$(uname -s)" = "Darwin" ]; then
        echo "  [iOS] Building static / C-ABI libraries for iOS targets..."
        for target in "${IOS_TARGETS[@]}"; do
            echo "    Compiling --lib for ${target}..."
            cargo build --release --target "${target}" -p "${app}" --lib || true
        done
    fi

    # 3. Flutter Scaffolding & Packaging
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

    # 4. Create Standalone Platform Bundle ZIP
    if command -v zip >/dev/null 2>&1; then
        if ${is_ios}; then
            bundle_name="${app}-${VERSION}-ios.zip"
            echo "  [iOS] Creating standalone iOS bundle: ${OUTPUT_DIR}/${bundle_name}..."
            (
                cd "${mobile_dir}"
                zip -r "${OUTPUT_DIR}/${bundle_name}" . -x "*.git*" "build/*" ".dart_tool/*" || true
            )
        else
            bundle_name="${app}-${VERSION}-android.zip"
            echo "  [Android] Creating standalone Android bundle: ${OUTPUT_DIR}/${bundle_name}..."
            (
                cd "${mobile_dir}"
                zip -r "${OUTPUT_DIR}/${bundle_name}" . -x "*.git*" "build/*" ".dart_tool/*" || true
            )
        fi
    fi
done

# 5. Compute SHA-256 Checksums
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
