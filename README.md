# DigitalRobbo

Modern GNU Robbo remake in Rust + Bevy.

## Crates

- `robbo-core` — deterministic simulation (no Bevy)
- `robbo-formats` — gnurobbo `.dat` level parser
- `robbo-app` — Bevy front-end

## Build

```bash
unset ARGV0 && cargo check --workspace
unset ARGV0 && cargo run -p robbo-app
```

## Automated game smoke test

Headless end-to-end check: boot → skip intro → load level 1 → move Robbo → capture PNG screenshots.

```bash
chmod +x scripts/run_game_smoke_test.sh
./scripts/run_game_smoke_test.sh              # binary mode (writes target/smoke/*.png)
ROBBO_SMOKE_MODE=test ./scripts/run_game_smoke_test.sh   # integration test mode
cargo run -p robbo-app -- --smoke-test        # same scenario, manual run
```

Uses `xvfb-run` when available (CI and headless Linux). Logic tests remain in `robbo-core` / `robbo-formats` via `cargo test --workspace`.

## Web (WASM)

```bash
rustup target add wasm32-unknown-unknown
# Clear RUSTFLAGS if your system linker uses mold (breaks wasm-ld)
RUSTFLAGS="" trunk serve crates/robbo-app/index.html
```

## Android

Requires Android SDK + NDK (API 31+), `cargo-ndk`, and `ANDROID_SDK_ROOT` set.

```bash
chmod +x scripts/build_android.sh scripts/verify_assets.sh
./scripts/verify_assets.sh
./scripts/build_android.sh
adb install -r mobile/android/app/build/outputs/apk/debug/app-debug.apk
```

See [docs/mobile.md](docs/mobile.md) for details. Touch pads (move left, shoot right) appear only on Android.

## Mobile (iOS)

Use Bevy's iOS templates. See `docs/mobile.md`.

## Docs

Architecture design: [docs/architecture-design/README.md](docs/architecture-design/README.md)

## Controls

- Arrow keys / WASD — move
- Space — shoot
- Z — undo
- Esc — pause / back
- Enter — confirm menu
- F9 — toggle editor stub (stretch)
