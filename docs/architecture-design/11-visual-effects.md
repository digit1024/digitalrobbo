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

We use a **lightweight sprite particle** pipeline (Bevy `Sprite` + `Transform` + `Timer`), not a third-party crate yet:

- Works on **Bevy 0.15** today (no `bevy_particle_systems` 0.15 / Hanabi setup).
- WASM-friendly, few draw calls for Robbo-scale bursts.
- Same observer hook can later drive [bevy_particle_systems](https://docs.rs/bevy_particle_systems) or Hanabi if we outgrow this.

## Module layout (`crates/robbo-app/src/effects/`)

```
effects/
  mod.rs       — exports
  particle.rs  — FxParticle, FxParticleState, FX_Z_LAYER
  presets.rs   — FxPreset recipes (Teleport implemented first)
  systems.rs   — fx_on_core_events, tick_fx_particles
```

## FxPreset roadmap

| Preset | Trigger | Status |
|--------|---------|--------|
| `Teleport` | `GameEvent::Teleported` | **Implemented** — burst at `from` + `to` |
| `Explosion` | `GameEvent::Exploded` / `Revealed` | Planned — augment `ExplosionEffect` |
| `ShotTrail` | `GameEvent::Shot` | Planned — sparse trail along bolt path |

## Teleport (v1)

- 16 particles per portal, ~0.65 s lifetime.
- Cyan / violet palette, outward + upward drift.
- Spawned as children of `LevelRoot` (cleared on level rebuild).
- Complements `TeleportReveal` (hides Robbo sprite for one sim tick).

## Adding a new effect

1. Add variant to `FxPreset` in `presets.rs`.
2. Implement `spawn_*` using `FxParticle` + `FxParticleState`.
3. Match arm in `fx_on_core_events` (`systems.rs`).
4. Register `tick_fx_particles` if not already in the app schedule.

## Object pool

`pool.rs` holds a generic `Pool<T>` for future reuse. Particles currently spawn/despawn entities per burst (low count). Pool when shot trails add sustained churn.
