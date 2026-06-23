# Mobile build notes

DigitalRobbo targets Android (and eventually iOS) via Bevy's mobile template.

## Requirements

- Rust stable + Android targets: `aarch64-linux-android`, `armv7-linux-androideabi`
- [Android SDK](https://developer.android.com/studio) with NDK (API 31+)
- `cargo-ndk`: `cargo install cargo-ndk`
- Environment variables:
  - `ANDROID_SDK_ROOT` or `ANDROID_HOME` → SDK root
  - `ANDROID_NDK_ROOT` → NDK path (e.g. `$ANDROID_SDK_ROOT/ndk/<version>`)

## Build APK

From repo root (auto-detects `~/Android/Sdk` and newest NDK):

```bash
chmod +x scripts/build_android.sh scripts/android_env.sh scripts/verify_assets.sh
source scripts/android_env.sh   # optional — build script sources this automatically
./scripts/build_android.sh --install   # build + install on connected device
./scripts/build_android.sh --run       # build + install + launch
```

Output: `mobile/android/app/build/outputs/apk/debug/app-debug.apk`

Detected on this machine:
- SDK: `~/Android/Sdk`
- NDK: `~/Android/Sdk/ndk/<version>` (newest installed, e.g. `28.2.13676358`)

### Manual steps

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi
unset ARGV0
cargo ndk -t arm64-v8a -t armeabi-v7a -P 31 \
  -o mobile/android/app/src/main/jniLibs build --release -p robbo-app
cd mobile/android && ./gradlew assembleDebug
```

## Install on device

```bash
adb install -r -t mobile/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start --user 0 -n org.bevyengine.digitalrobbo/.MainActivity
```

**Samsung / Auto Blocker:** if install fails with `INSTALL_FAILED_VERIFICATION_FAILURE`:

```bash
adb shell settings put global verifier_verify_adb_installs 0
adb shell settings put global package_verifier_enable 0
```

Or disable **Settings → Security and privacy → Auto Blocker → block app installs via USB**.

`./scripts/build_android.sh --install` retries automatically after toggling those settings.

## Platform notes

| Feature | Android behavior |
|---------|------------------|
| Assets | Full `assets/` tree bundled in APK |
| Saves | App internal storage (`files/digitalrobbo/save.ron`) |
| Touch | Left pad = move (WASD semantics), right pad = turn + shoot |
| Audio | Unlocks on first touch (like WASM) |
| Min API | 31 (Android 12+) via GameActivity |
| ABIs | `arm64-v8a`, `armeabi-v7a` |

Touch pads are **Android-only** (`#[cfg(target_os = "android")]`). Desktop builds are unchanged.

## iOS

Not yet wired. Follow Bevy iOS mobile guide when needed.

## App icon

Launcher icons are generated from [`assets/icon.png`](../assets/icon.png) into `mobile/android/app/src/main/res/mipmap-*`.

After changing the source image:

```bash
./scripts/generate_android_icons.sh
```

Then rebuild the APK (`./scripts/build_android.sh`).
