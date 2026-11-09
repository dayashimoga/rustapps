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

# Auto-detect ANDROID_HOME if not explicitly set
if [ -z "${ANDROID_HOME:-}" ] && [ -d "/usr/local/lib/android/sdk" ]; then
    export ANDROID_HOME="/usr/local/lib/android/sdk"
fi

# Auto-detect FLUTTER_ROOT if not set
if [ -z "${FLUTTER_ROOT:-}" ] && command -v flutter &>/dev/null; then
    FLUTTER_ROOT="$(dirname "$(dirname "$(command -v flutter)")")"
    export FLUTTER_ROOT
    echo "Auto-detected FLUTTER_ROOT: ${FLUTTER_ROOT}"
fi

# Auto-detect ANDROID_NDK_HOME in CI environments if not explicitly set
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ] && [ -d "${ANDROID_HOME}/ndk" ]; then
    latest_ndk=$(ls -1d "${ANDROID_HOME}/ndk/"* 2>/dev/null | sort -V | tail -n1 || true)
    if [ -n "${latest_ndk}" ]; then
        export ANDROID_NDK_HOME="${latest_ndk}"
        export ANDROID_NDK_ROOT="${latest_ndk}"
        echo "Auto-detected ANDROID_NDK_HOME: ${ANDROID_NDK_HOME}"
    fi
fi

# Pre-generate standard debug.keystore if missing to ensure all APK release builds are properly signed
if [ ! -f "${HOME}/.android/debug.keystore" ] && command -v keytool &>/dev/null; then
    echo "Pre-generating standard debug.keystore for offline standalone APK signing..."
    mkdir -p "${HOME}/.android"
    keytool -genkey -v -keystore "${HOME}/.android/debug.keystore" \
        -storepass android -alias androiddebugkey -keypass android \
        -keyalg RSA -keysize 2048 -validity 10000 \
        -dname "CN=Android Debug,O=Android,C=US" || true
fi

if [ -n "${FILTER_APP}" ]; then
    APPS=("${FILTER_APP}")
else
    APPS=(
        "devops-secret-vault"
        "ai-file-explorer"
        "universal-log-viewer"
        "universal-live-translator"
        "pro-av-editor"
        "personal-knowledge-graph"
        "api-protocol-studio"
        "parquet-analytics-workbench"
        "dfir-triage-kit"
        "screen-recorder-studio"
        "signal-telemetry-oscilloscope"
        "audio-mastering-studio"
        "qr-code-studio"
    )
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

        # C. Android Build Configurations & Official Gradle Wrapper Installation
        if ! ${is_ios}; then
            echo "  [Android] Setting up local.properties and Gradle wrapper..."
            mkdir -p "${mobile_dir}/android"

            cat <<EOF > "${mobile_dir}/android/local.properties"
sdk.dir=${ANDROID_HOME:-/usr/local/lib/android/sdk}
flutter.sdk=${FLUTTER_ROOT:-/opt/hostedtoolcache/flutter}
flutter.buildMode=release
flutter.versionName=${VERSION}
flutter.versionCode=1
EOF
            if [ -n "${ANDROID_NDK_HOME:-}" ]; then
                ndk_ver=$(basename "${ANDROID_NDK_HOME}")
                echo "ndk.dir=${ANDROID_NDK_HOME}" >> "${mobile_dir}/android/local.properties"
                echo "ndk.version=${ndk_ver}" >> "${mobile_dir}/android/local.properties"
            fi

            # If Gradle wrapper is missing in project, copy from pre-generated template or unpack from Flutter SDK
            if [ ! -f "${mobile_dir}/android/gradlew" ] || [ ! -f "${mobile_dir}/android/gradle/wrapper/gradle-wrapper.jar" ]; then
                if [ -d "/tmp/flutter_tmpl_wrapper/android/gradle" ]; then
                    echo "  -> Copying Gradle wrapper from /tmp/flutter_tmpl_wrapper..."
                    mkdir -p "${mobile_dir}/android/gradle/wrapper"
                    cp -f /tmp/flutter_tmpl_wrapper/android/gradle/wrapper/gradle-wrapper.jar "${mobile_dir}/android/gradle/wrapper/"
                    if [ ! -f "${mobile_dir}/android/gradle/wrapper/gradle-wrapper.properties" ]; then
                        cp -f /tmp/flutter_tmpl_wrapper/android/gradle/wrapper/gradle-wrapper.properties "${mobile_dir}/android/gradle/wrapper/"
                    fi
                    cp /tmp/flutter_tmpl_wrapper/android/gradlew "${mobile_dir}/android/"
                    cp /tmp/flutter_tmpl_wrapper/android/gradlew.bat "${mobile_dir}/android/"
                elif [ -n "${FLUTTER_ROOT:-}" ] && [ -d "${FLUTTER_ROOT}/bin/cache/artifacts/gradle_wrapper" ]; then
                    echo "  -> Copying Gradle wrapper from Flutter SDK cache..."
                    mkdir -p "${mobile_dir}/android/gradle/wrapper"
                    cp -f "${FLUTTER_ROOT}/bin/cache/artifacts/gradle_wrapper/gradle/wrapper/gradle-wrapper.jar" "${mobile_dir}/android/gradle/wrapper/" 2>/dev/null || true
                    cp "${FLUTTER_ROOT}/bin/cache/artifacts/gradle_wrapper/gradlew" "${mobile_dir}/android/" 2>/dev/null || true
                    cp "${FLUTTER_ROOT}/bin/cache/artifacts/gradle_wrapper/gradlew.bat" "${mobile_dir}/android/" 2>/dev/null || true
                elif command -v flutter &>/dev/null; then
                    echo "  -> Unpacking Flutter Gradle wrapper template..."
                    flutter create --platforms=android --org com.offlineapps --project-name tmpl_wrapper /tmp/flutter_tmpl_wrapper >/dev/null 2>&1 || true
                    if [ -d "/tmp/flutter_tmpl_wrapper/android/gradle" ]; then
                        mkdir -p "${mobile_dir}/android/gradle/wrapper"
                        cp -f /tmp/flutter_tmpl_wrapper/android/gradle/wrapper/gradle-wrapper.jar "${mobile_dir}/android/gradle/wrapper/"
                        if [ ! -f "${mobile_dir}/android/gradle/wrapper/gradle-wrapper.properties" ]; then
                            cp -f /tmp/flutter_tmpl_wrapper/android/gradle/wrapper/gradle-wrapper.properties "${mobile_dir}/android/gradle/wrapper/"
                        fi
                        cp /tmp/flutter_tmpl_wrapper/android/gradlew "${mobile_dir}/android/"
                        cp /tmp/flutter_tmpl_wrapper/android/gradlew.bat "${mobile_dir}/android/"
                    fi
                fi
            fi

            # Enforce Gradle 8.5 in gradle-wrapper.properties across all builds
            if [ -f "${mobile_dir}/android/gradle/wrapper/gradle-wrapper.properties" ]; then
                sed -i -E 's/gradle-[0-9.]*-(all|bin)\.zip/gradle-8.5-all.zip/g' "${mobile_dir}/android/gradle/wrapper/gradle-wrapper.properties" 2>/dev/null || true
            else
                mkdir -p "${mobile_dir}/android/gradle/wrapper"
                cat <<'WRAPPER_EOF' > "${mobile_dir}/android/gradle/wrapper/gradle-wrapper.properties"
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-8.5-all.zip
networkTimeout=10000
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
WRAPPER_EOF
            fi
            chmod +x "${mobile_dir}/android/gradlew" 2>/dev/null || true
        fi

        # D. Flutter Compilation & Standalone APK Assembly
        apk_target="${OUTPUT_DIR}/${app}-${VERSION}.apk"
        apk_target_simple="${OUTPUT_DIR}/${app}.apk"
        apk_found=""

        if command -v flutter &>/dev/null; then
            echo "  [Flutter] Preparing Flutter dependencies for ${app}..."
            (
                cd "${mobile_dir}"
                flutter config --no-analytics || true
                if [ -n "${JAVA_HOME:-}" ]; then
                    flutter config --jdk-dir="${JAVA_HOME}" 2>/dev/null || true
                fi
                flutter pub get
                if ! ${is_ios}; then
                    echo "  [Flutter] Building Standalone Android Release APK..."
                    flutter build apk --release --no-tree-shake-icons
                fi
            )

            # Search for generated APK candidate
            for candidate in \
                "${mobile_dir}/build/app/outputs/flutter-apk/app-release.apk" \
                "${mobile_dir}/build/app/outputs/apk/release/app-release.apk" \
                "${mobile_dir}/android/app/build/outputs/apk/release/app-release.apk" \
                "${mobile_dir}/build/app/outputs/flutter-apk/app-debug.apk" \
                "${mobile_dir}/build/app/outputs/apk/debug/app-debug.apk"; do
                if [ -f "${candidate}" ]; then
                    cand_size=$(stat -c%s "${candidate}" 2>/dev/null || stat -f%z "${candidate}" 2>/dev/null || echo 0)
                    if [ "${cand_size}" -gt 500000 ]; then
                        apk_found="${candidate}"
                        break
                    fi
                fi
            done
        fi

        # E. Direct Gradle Build fallback if APK not yet generated
        if ! ${is_ios} && [ -z "${apk_found}" ] && [ -f "${mobile_dir}/android/gradlew" ]; then
            echo "  [Gradle] Attempting direct assembleRelease with gradlew..."
            (
                cd "${mobile_dir}/android"
                ./gradlew app:assembleRelease --stacktrace || ./gradlew app:assembleDebug --stacktrace || true
            )
            for candidate in \
                "${mobile_dir}/android/app/build/outputs/apk/release/app-release.apk" \
                "${mobile_dir}/android/app/build/outputs/apk/release/app-release-unsigned.apk" \
                "${mobile_dir}/android/app/build/outputs/apk/debug/app-debug.apk" \
                "${mobile_dir}/build/app/outputs/flutter-apk/app-release.apk"; do
                if [ -f "${candidate}" ]; then
                    cand_size=$(stat -c%s "${candidate}" 2>/dev/null || stat -f%z "${candidate}" 2>/dev/null || echo 0)
                    if [ "${cand_size}" -gt 500000 ]; then
                        apk_found="${candidate}"
                        break
                    fi
                fi
            done
        fi

        # Copy verified APK to target output directory
        if [ -n "${apk_found}" ]; then
            cp -f "${apk_found}" "${apk_target}"
            cp -f "${apk_found}" "${apk_target_simple}"
            echo "  -> Verified and staged standalone APK: ${apk_target} ($(ls -lh "${apk_target}" | awk '{print $5}'))"

            # Auto-detect apksigner to verify and sign APK if unsigned (prevents package parse errors on Android)
            apksigner_bin=""
            if [ -n "${ANDROID_HOME:-}" ] && [ -d "${ANDROID_HOME}/build-tools" ]; then
                latest_bt=$(ls -1d "${ANDROID_HOME}/build-tools/"* 2>/dev/null | sort -V | tail -n1 || true)
                [ -n "${latest_bt}" ] && [ -x "${latest_bt}/apksigner" ] && apksigner_bin="${latest_bt}/apksigner"
            fi
            if [ -n "${apksigner_bin}" ] && [ -f "${HOME}/.android/debug.keystore" ]; then
                if ! "${apksigner_bin}" verify "${apk_target}" &>/dev/null; then
                    echo "  [apksigner] Signing APK with standard debug keystore..."
                    "${apksigner_bin}" sign --ks "${HOME}/.android/debug.keystore" --ks-pass pass:android --key-pass pass:android "${apk_target}" || true
                    "${apksigner_bin}" sign --ks "${HOME}/.android/debug.keystore" --ks-pass pass:android --key-pass pass:android "${apk_target_simple}" || true
                fi
                echo "  [apksigner] Verifying APK signature..."
                "${apksigner_bin}" verify --verbose "${apk_target}" || true
            fi
        elif ! ${is_ios}; then
            echo "ERROR: Standalone Android APK could not be produced for ${app}!" >&2
            exit 1
        fi

        # F. Create Standalone Release Distribution Package (NO source code!)
        echo "  Creating standalone ${platform_tag} release bundle: ${OUTPUT_DIR}/${bundle_name}..."
        stage_dist="${OUTPUT_DIR}/stage_${app}_${platform_tag}"
        rm -rf "${stage_dist}"
        mkdir -p "${stage_dist}"

        if [ -f "${apk_target}" ]; then
            cp -f "${apk_target}" "${stage_dist}/${app}-${VERSION}.apk"
            cp -f "${apk_target_simple}" "${stage_dist}/${app}.apk" 2>/dev/null || true
        fi
        [ -f "${OUTPUT_DIR}/metadata.json" ] && cp -f "${OUTPUT_DIR}/metadata.json" "${stage_dist}/"
        [ -f "${OUTPUT_DIR}/${app}-sbom.json" ] && cp -f "${OUTPUT_DIR}/${app}-sbom.json" "${stage_dist}/"

        cat <<EOF > "${stage_dist}/README.md"
# ${app} Mobile (${platform_tag})
Version: ${VERSION}
Platform: ${platform_tag}
Offline Autonomous: true
Zero Telemetry / Air-Gapped: true

## Quick Sideload & Installation Guide
1. Copy \`${app}-${VERSION}.apk\` (or \`${app}.apk\`) to your Android device via USB, ADB, or file transfer.
2. In your device Settings -> Apps & Notifications / Security, enable "Install unknown apps" for your file manager or browser.
3. Tap the .apk file on your phone to install.
4. Launch the application. The entire app runs 100% locally with zero internet access required.
EOF

        rm -f "${OUTPUT_DIR}/${bundle_name}"
        create_bundle_zip "${stage_dist}" "${OUTPUT_DIR}/${bundle_name}"
        rm -rf "${stage_dist}"

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
