# 03 — Core Simulation

> **Status:** Draft  
> **Last updated:** M-DOC

## Principles

1. **Discrete grid** — positions are `(col, row)` integers; no `f32` in logic.
2. **Deterministic ticks** — `World::step(input)` advances one logical step.
3. **Event-driven output** — side effects are emitted as `GameEvent`, not hidden mutations.
4. **Cloneable snapshots** — full world clone for undo/redo.

## Core types (sketch)

```rust
pub struct Cell { pub col: i16, pub row: i16 }
pub struct World { /* grid, entities, counters, rng seed if needed */ }

pub enum PlayerInput {
    Move(Direction),
    Shoot(Direction),
    Wait,
}

pub enum GameEvent {
    Moved { entity: EntityId, from: Cell, to: Cell },
    Pushed { entity: EntityId, from: Cell, to: Cell },
    Collected { kind: Collectible, at: Cell },
    Shot { from: Cell, direction: Direction },
    Exploded { at: Cell },
    Teleported { entity: EntityId, from: Cell, to: Cell },
    Died { entity: EntityId, cause: DeathCause },
    LevelComplete,
    LevelFailed,
}

impl World {
    pub fn step(&mut self, input: PlayerInput) -> Vec<GameEvent> { /* ... */ }
    pub fn snapshot(&self) -> World { self.clone() }
    pub fn restore(&mut self, snap: World) { *self = snap; }
}
```

## Tick model

- **Player phase:** apply `PlayerInput` (move, push, shoot, or wait).
- **Projectile phase:** advance shots; resolve collisions.
- **Enemy phase:** bears, birds, butterflies per original delay rules.
- **Environment phase:** bombs, teleports, questionmarks, doors.
- **Win check:** screws collected ≥ required → capsule opens; Robbo on capsule → complete.

Enemy timing and AI ported from gnurobbo reference (`screen.c` behaviour).

## Undo / redo

- Before each player-initiated step, push `world.snapshot()` onto undo stack.
- Undo: pop snapshot, restore world, reset visual layer (no replay needed).
- Redo: optional second stack of forward snapshots.

Implemented in `robbo-core` as `CommandHistory`; `robbo-app` triggers on keybind.

## Determinism testing

- Run fixed command sequence; hash serialized world state.
- Same sequence must produce identical hash across runs and platforms.
- No `SystemTime`, no thread RNG in core.

## Coordinate system

- Origin: top-left of level grid.
- `col` increases right; `row` increases down.
- Level sizes: 16×31 or 32×31 (from pack metadata).
