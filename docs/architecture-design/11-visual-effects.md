# 11 — Visual Effects (particles)

> **Status:** Draft  
> **Last updated:** 2026-06

Short-lived presentation-only effects in `robbo-app`. The simulation (`robbo-core`) never knows about particles.

## Design

| Layer | Responsibility |
|-------|----------------|
| `robbo-core` | `GameEvent::Teleported`, `Exploded`, `Shot`, … |
| `bridge` | `CoreGameEvent` fan-out each tick |
| `effects` | Observer → spawn `FxParticle` sprites |
| `render` | Level sprites, tweens (`ExplosionEffect`, `TeleportReveal`) |

Sprite-based particles (Bevy `Sprite` + `Transform` + `Timer`) — no third-party crate. WASM-friendly; enough for Robbo-scale FX.

## Module layout (`crates/robbo-app/src/effects/`)

```
effects/
  mod.rs       — exports
  particle.rs  — FxParticle, FxParticleState, FX_Z_LAYER
  presets.rs   — burst recipes (teleport, explosion, shot trail, ambient)
  systems.rs   — fx_on_core_events, tick_fx_particles
  aura.rs      — TeleportAuraAnchor, sync + ambient sparkle tick
```

## FxPreset map

| Effect | Trigger | Implementation |
|--------|---------|----------------|
| Teleport burst | `GameEvent::Teleported` | `spawn_teleport_burst` at `from` + `to` |
| Teleport ambient | Level load + every ~0.14s per mirror | `TeleportAuraAnchor` + `spawn_ambient_teleport_pixel` |
| Explosion | `GameEvent::Exploded`, `Revealed` | `spawn_explosion_burst` (+ legacy `ExplosionEffect` flash in `render`) |
| Shot trail | `GameEvent::Shot` | `spawn_shot_trail` along first 5 cells ahead of shooter |

## Teleport ambient

- On `ReloadVisualsEvent`, `sync_teleport_auras` places one anchor per `ElementKind::Teleport`.
- `tick_teleport_auras` emits slow cyan/violet pixels that drift and fade (~1.75s).
- Anchors are children of `LevelRoot` (cleared on level rebuild).

## Adding a new effect

1. Add `spawn_*` in `presets.rs`.
2. Match arm in `fx_on_core_events` (`systems.rs`).
3. Document row in this file.

## Object pool

`pool.rs` holds a generic `Pool<T>` for future high-churn trails. Current bursts use spawn/despawn (low count).

## Future

- Trail on moving `Laser` segments (per-tick particles along bolt path).
- Optional swap to [bevy_particle_systems](https://docs.rs/bevy_particle_systems) when Bevy 0.15+ support exists.
