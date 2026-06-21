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

## Web (WASM)

```bash
rustup target add wasm32-unknown-unknown
# Clear RUSTFLAGS if your system linker uses mold (breaks wasm-ld)
RUSTFLAGS="" trunk serve crates/robbo-app/index.html
```

## Mobile

Use Bevy's Android/iOS templates. See `docs/architecture-design/adr/0004-all-three-platforms.md`.

## Docs

Architecture design: [docs/architecture-design/README.md](docs/architecture-design/README.md)

## Controls

- Arrow keys / WASD — move
- Space — shoot
- Z — undo
- Esc — pause / back
- Enter — confirm menu
- F9 — toggle editor stub (stretch)
