#!/usr/bin/env bash
# Source this to set Android SDK/NDK for DigitalRobbo builds.
# Usage: source scripts/android_env.sh

_android_env_die() {
    echo "android_env: $*" >&2
    return 1
}

# SDK: env var → common Pop!_OS / Android Studio paths
if [[ -z "${ANDROID_SDK_ROOT:-}" && -z "${ANDROID_HOME:-}" ]]; then
    for candidate in \
        "$HOME/Android/Sdk" \
        "$HOME/Android/sdk" \
        "/opt/android-sdk" \
        "/usr/lib/android-sdk"; do
        if [[ -d "$candidate" ]]; then
            export ANDROID_SDK_ROOT="$candidate"
            export ANDROID_HOME="$candidate"
            break
        fi
    done
elif [[ -n "${ANDROID_HOME:-}" && -z "${ANDROID_SDK_ROOT:-}" ]]; then
    export ANDROID_SDK_ROOT="$ANDROID_HOME"
elif [[ -n "${ANDROID_SDK_ROOT:-}" && -z "${ANDROID_HOME:-}" ]]; then
    export ANDROID_HOME="$ANDROID_SDK_ROOT"
fi

if [[ -z "${ANDROID_SDK_ROOT:-}" ]]; then
    _android_env_die "Android SDK not found. Install Android Studio or set ANDROID_SDK_ROOT."
    return 1
fi

# NDK: env var → newest version under $SDK/ndk/
if [[ -z "${ANDROID_NDK_ROOT:-}" && -d "$ANDROID_SDK_ROOT/ndk" ]]; then
  latest_ndk=""
  latest_ver=0
  for ndk_dir in "$ANDROID_SDK_ROOT/ndk"/*; do
    [[ -d "$ndk_dir" ]] || continue
    ver_name="$(basename "$ndk_dir")"
    # Pick highest semver-ish folder name (28.2 > 27.0 > 26.1 …)
    if [[ "$ver_name" > "$(basename "${latest_ndk:-/0}")" ]] || [[ -z "$latest_ndk" ]]; then
      latest_ndk="$ndk_dir"
    fi
  done
  if [[ -n "$latest_ndk" ]]; then
    export ANDROID_NDK_ROOT="$latest_ndk"
  fi
fi

if [[ -z "${ANDROID_NDK_ROOT:-}" || ! -d "$ANDROID_NDK_ROOT" ]]; then
    _android_env_die "Android NDK not found under $ANDROID_SDK_ROOT/ndk — install via Android Studio SDK Manager."
    return 1
fi

export PATH="$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$PATH"

echo "ANDROID_SDK_ROOT=$ANDROID_SDK_ROOT"
echo "ANDROID_NDK_ROOT=$ANDROID_NDK_ROOT"
