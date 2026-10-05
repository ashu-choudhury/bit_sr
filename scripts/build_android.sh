#!/usr/bin/env bash
# Automated build and packaging pipeline for bit_sr Android APK and native shared libraries.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "==> bit_sr Android Build Pipeline (Bash)"
echo "    Workspace Root: ${ROOT_DIR}"

# 1. Locate Android SDK and NDK
ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [ -z "${ANDROID_HOME}" ]; then
    echo "ERROR: ANDROID_HOME or ANDROID_SDK_ROOT must be set."
    exit 1
fi

ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}"
if [ -z "${ANDROID_NDK_HOME}" ]; then
    if [ -d "${ANDROID_HOME}/ndk" ]; then
        LATEST_NDK=$(ls -1 "${ANDROID_HOME}/ndk" | sort -V | tail -n 1)
        ANDROID_NDK_HOME="${ANDROID_HOME}/ndk/${LATEST_NDK}"
    fi
fi

if [ -z "${ANDROID_NDK_HOME}" ] || [ ! -d "${ANDROID_NDK_HOME}" ]; then
    echo "ERROR: Android NDK not found. Please set ANDROID_NDK_HOME."
    exit 1
fi

echo "    Android SDK: ${ANDROID_HOME}"
echo "    Android NDK: ${ANDROID_NDK_HOME}"

TOOLCHAIN_BIN="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin"
if [ ! -d "${TOOLCHAIN_BIN}" ]; then
    TOOLCHAIN_BIN="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/darwin-x86_64/bin"
fi

# 2. Compile native libraries
TARGETS=("aarch64-linux-android" "armv7-linux-androideabi" "x86_64-linux-android")
ABIS=("arm64-v8a" "armeabi-v7a" "x86_64")

for i in "${!TARGETS[@]}"; do
    TARGET="${TARGETS[$i]}"
    ABI="${ABIS[$i]}"
    echo "--> Compiling libbit_sr.so for ${TARGET} (${ABI})..."

    if [ "${TARGET}" == "aarch64-linux-android" ]; then
        export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${TOOLCHAIN_BIN}/aarch64-linux-android28-clang"
        export CC_aarch64_linux_android="${TOOLCHAIN_BIN}/aarch64-linux-android28-clang"
        export CXX_aarch64_linux_android="${TOOLCHAIN_BIN}/aarch64-linux-android28-clang++"
        export AR_aarch64_linux_android="${TOOLCHAIN_BIN}/llvm-ar"
    elif [ "${TARGET}" == "armv7-linux-androideabi" ]; then
        export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${TOOLCHAIN_BIN}/armv7a-linux-androideabi28-clang"
        export CC_armv7_linux_androideabi="${TOOLCHAIN_BIN}/armv7a-linux-androideabi28-clang"
        export CXX_armv7_linux_androideabi="${TOOLCHAIN_BIN}/armv7a-linux-androideabi28-clang++"
        export AR_armv7_linux_androideabi="${TOOLCHAIN_BIN}/llvm-ar"
    else
        export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${TOOLCHAIN_BIN}/x86_64-linux-android28-clang"
        export CC_x86_64_linux_android="${TOOLCHAIN_BIN}/x86_64-linux-android28-clang"
        export CXX_x86_64_linux_android="${TOOLCHAIN_BIN}/x86_64-linux-android28-clang++"
        export AR_x86_64_linux_android="${TOOLCHAIN_BIN}/llvm-ar"
    fi

    (cd "${ROOT_DIR}" && cargo build -p bit_sr_engine --no-default-features --lib --target "${TARGET}")

    DEST_DIR="${ROOT_DIR}/android/app/src/main/jniLibs/${ABI}"
    mkdir -p "${DEST_DIR}"
    cp "${ROOT_DIR}/target/${TARGET}/debug/libbit_sr.so" "${DEST_DIR}/libbit_sr.so"
    echo "    Installed: ${DEST_DIR}/libbit_sr.so"
done

# 3. Assemble APK with Gradle
echo "--> Assembling Android APK with Gradle..."
if [ ! -f "${ROOT_DIR}/android/local.properties" ]; then
    echo "sdk.dir=${ANDROID_HOME}" > "${ROOT_DIR}/android/local.properties"
    echo "ndk.dir=${ANDROID_NDK_HOME}" >> "${ROOT_DIR}/android/local.properties"
fi
chmod +x "${ROOT_DIR}/android/gradlew"
(cd "${ROOT_DIR}/android" && ./gradlew assembleDebug)

# 4. Copy final APK to target
mkdir -p "${ROOT_DIR}/target"
cp "${ROOT_DIR}/android/app/build/outputs/apk/debug/app-debug.apk" "${ROOT_DIR}/target/bit_sr-debug.apk"

echo "==> Android Build Complete!"
echo "    Output APK: ${ROOT_DIR}/target/bit_sr-debug.apk"
