#!/usr/bin/env bash
# Build DigitalRobbo APK for Android (arm64-v8a + armeabi-v7a).
# Auto-detects SDK/NDK on Pop!_OS / Android Studio installs.
#
# Usage:
#   ./scripts/build_android.sh           # build debug APK
#   ./scripts/build_android.sh --install # build + adb install on connected device
#   ./scripts/build_android.sh --run     # build + install + launch
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ANDROID_DIR="$ROOT/mobile/android"
JNILIBS="$ANDROID_DIR/app/src/main/jniLibs"
INSTALL=0
RUN=0
ABIS="arm64-v8a armeabi-v7a"

for arg in "$@"; do
    case "$arg" in
        --install) INSTALL=1 ;;
        --run) INSTALL=1; RUN=1 ;;
        --arm64-only) ABIS="arm64-v8a" ;;
        -h|--help)
            echo "Usage: $0 [--install] [--run] [--arm64-only]"
            echo "  --install     adb install after build"
            echo "  --run         install + launch MainActivity"
            echo "  --arm64-only  skip armeabi-v7a (faster for modern phones)"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg (try --install or --run)" >&2
            exit 1
            ;;
    esac
done

# shellcheck source=android_env.sh
source "$ROOT/scripts/android_env.sh"

if ! command -v cargo-ndk >/dev/null 2>&1; then
    echo "==> Installing cargo-ndk..."
    unset ARGV0
    cargo install cargo-ndk --locked
fi

unset ARGV0
export RUSTFLAGS="${RUSTFLAGS:-}"

echo "==> Adding Rust Android targets..."
if [[ "$ABIS" == *arm64-v8a* ]]; then
    rustup target add aarch64-linux-android
fi
if [[ "$ABIS" == *armeabi-v7a* ]]; then
    rustup target add armv7-linux-androideabi
fi

echo "==> Verifying game assets..."
"$ROOT/scripts/verify_assets.sh"

mkdir -p "$JNILIBS"
rm -rf "$JNILIBS"/*

echo "==> Building Rust libs for Android (release, ABIs: $ABIS)..."
NDK_ARGS=(-P 31 -o "$JNILIBS" build --release -p robbo-app)
for abi in $ABIS; do
    NDK_ARGS=(-t "$abi" "${NDK_ARGS[@]}")
done
cargo ndk "${NDK_ARGS[@]}"

echo "==> Assembling APK with Gradle..."
cd "$ANDROID_DIR"
chmod +x ./gradlew
./gradlew assembleDebug --no-daemon

APK="$ANDROID_DIR/app/build/outputs/apk/debug/app-debug.apk"
echo ""
echo "APK ready: $APK"

if [[ "$INSTALL" -eq 1 ]]; then
    if ! command -v adb >/dev/null 2>&1; then
        echo "adb not found in PATH" >&2
        exit 1
    fi
  devices="$(adb devices | awk 'NR>1 && $2=="device" {print $1}')"
  if [[ -z "$devices" ]]; then
        echo "No Android device connected (adb devices)." >&2
        exit 1
    fi
    echo "==> Installing on device..."
    if ! adb install -r -t "$APK"; then
        echo "Retrying after disabling ADB install verification (Samsung Auto Blocker)..."
        adb shell settings put global verifier_verify_adb_installs 0 2>/dev/null || true
        adb shell settings put global package_verifier_enable 0 2>/dev/null || true
        adb install -r -t "$APK"
    fi
    echo "Installed."
    if [[ "$RUN" -eq 1 ]]; then
        echo "==> Launching DigitalRobbo..."
        adb shell am start --user 0 -n org.bevyengine.digitalrobbo/.MainActivity
    fi
else
    echo "Install: ./scripts/build_android.sh --install"
    echo "   or:   adb install -r \"$APK\""
fi
