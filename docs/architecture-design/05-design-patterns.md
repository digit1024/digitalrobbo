# 05 — Design Patterns

> **Status:** Draft  
> **Last updated:** M-DOC

## 1. Command pattern (undo/redo)

**Where:** `robbo-core` (`CommandHistory`), triggered from `robbo-app`

```rust
pub struct CommandHistory {
    undo: Vec<World>,
    redo: Vec<World>,
}

impl CommandHistory {
    pub fn record(&mut self, before: World) { /* push undo, clear redo */ }
    pub fn undo(&mut self, current: &mut World) -> Option<World> { /* ... */ }
    pub fn redo(&mut self, current: &mut World) -> Option<World> { /* ... */ }
}
```

Every player action is one logical step = one undo point.

## 2. State machine

**App states** (`bevy_state` in `robbo-app`):

```rust
#[derive(States, Clone, Eq, PartialEq, Hash, Debug, Default)]
enum AppState {
    #[default]
    Boot,
    MainMenu,
    LevelSelect,
    Playing,
    Paused,
    LevelComplete,
    GameOver,
}
```

**Entity micro-states** (component on visual entities):

```rust
enum EntityAnimState {
    Idle,
    Moving,
    Pushing,
    Shooting,
    Dying,
    Teleporting,
}
```

Micro-states drive sprite frame selection; they do not affect simulation.

## 3. Observer pattern

Core emits `GameEvent` on each step. Bevy bridge converts to app events:

```rust
// robbo-app
fn bridge_events(mut reader: EventReader<CoreGameEvent>, /* audio, particles */) {
    for e in reader.read() {
        match e { /* spawn sounds, particles, UI updates */ }
    }
}
```

Decouples simulation from presentation side effects.

## 4. Object pool

**Where:** `robbo-app` — projectiles, explosion particles, teleport effects

```rust
struct Pool<T> {
    free: Vec<T>,
    active: Vec<T>,
}
```

Avoid spawn/despawn churn on WASM/mobile. Pools pre-warm on level load.

## Pattern placement summary

| Pattern | Crate | Purpose |
|---------|-------|---------|
| Command | robbo-core + app | Undo/redo |
| State machine | robbo-app | Screens + anim states |
| Observer | core events → app | Audio/VFX/UI reactions |
| Object pool | robbo-app | Projectiles and particles |
