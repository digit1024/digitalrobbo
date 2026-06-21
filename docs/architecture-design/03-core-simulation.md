# 03 — Core Simulation

> **Status:** Draft  
> **Last updated:** 2025-06

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

DigitalRobbo matches **gnurobbo puzzle rules**, not frame-accurate `DELAY_*` cycle counts. Environment and enemy timing use `World::tick` with modulo intervals (e.g. bears `% 4`, guns `% 8` with 1/8 fire roll). Robbo still gets one move or shoot per player step.

Per-tick pipeline (after player input):

1. **Pending spawns** — BigBoom countdown → reveal question-mark content  
2. **Projectiles** — advance and collide  
3. **Lasers / blasters** — beam extension, orphan cleanup, contact damage  
4. **Push boxes** — slide every 4 ticks while `sliding`  
5. **Enemies** — bears, birds, butterflies  
6. **Guns** — move, rotate, fire  
7. **Barriers** — conveyor shift every 4 ticks  
8. **Magnets** — lock + pull Robbo  
9. **Delayed bombs** — blaster-triggered detonations  

Visual-only delays (teleport shimmer, reveal crack) live in `robbo-app` and do not block core state.

## Undo / redo

- Before each player-initiated step, push `world.snapshot()` onto undo stack.
- Undo: pop snapshot, restore world, reset visual layer (no replay needed).
- Redo: optional second stack of forward snapshots.

Implemented in `robbo-core` as `CommandHistory`; `robbo-app` triggers on keybind.

## Coordinate system

- Origin: top-left of level grid.
- `col` increases right; `row` increases down.
- Level sizes: 16×31 or 32×31 (from pack metadata).

## Determinism testing

- Run fixed command sequence; hash serialized world state.
- Same sequence must produce identical hash across runs and platforms.
- No `SystemTime`, no thread RNG in core.
