# 02 — Architecture

> **Status:** Implemented (M7 gap-closure)  
> **Last updated:** 2026-06-20  
> **Stack:** Bevy 0.15, Rust stable

## Workspace layout

```
digitalrobbo/
├── crates/
│   ├── robbo-core/      # Pure simulation (no Bevy)
│   ├── robbo-formats/   # .dat parser/serializer
│   └── robbo-app/       # Bevy front-end
├── assets/
│   ├── levels/          # original.dat, robbo01.dat (GPL packs)
│   └── skins/
├── docs/architecture-design/
└── .github/workflows/
```

## C4 containers

```mermaid
flowchart TB
  subgraph app [robbo-app Bevy]
    Input[Input systems]
    Bridge[Core bridge]
    View[View and animation]
    FX[Effects particles]
    UI[Bevy UI]
    Audio[Audio]
  end

  subgraph formats [robbo-formats]
    Parser[DatParser]
  end

  subgraph core [robbo-core]
    World[World simulation]
    Commands[Command and undo]
    Events[Event stream]
  end

  Input --> Bridge
  Bridge --> World
  World --> Events
  Events --> View
  Events --> FX
  Parser --> World
  Bridge --> View
  UI --> Bridge
```

## Dependency rules

| Crate | May depend on | Must NOT depend on |
|-------|---------------|-------------------|
| `robbo-core` | std, serde (optional) | bevy, wgpu, windowing |
| `robbo-formats` | robbo-core | bevy |
| `robbo-app` | robbo-core, robbo-formats, bevy | — |

**Rule:** simulation logic never imports rendering types. Pixel/world conversion lives only in `robbo-app` behind `GridProjection`.

## Data flow (one player move)

```mermaid
sequenceDiagram
  participant Input
  participant App as robbo-app
  participant Core as robbo-core
  participant View

  Input->>App: Direction key pressed
  App->>App: Buffer input if anim in progress
  App->>Core: World::step(Move dir)
  Core-->>App: Vec of GameEvent
  App->>App: Push undo snapshot
  App->>View: Sync ECS from World + events
  View->>View: Start interpolation tweens
  View-->>Input: Ready for next input when progress >= 1.0
```

## Layer boundaries

1. **Logic layer** (`robbo-core`) — grid, entities, tick, win/lose, AI
2. **Format layer** (`robbo-formats`) — bytes/text → `Level` → `World`
3. **Presentation layer** (`robbo-app`) — Bevy ECS, sprites, camera, UI, audio, [`effects/`](11-visual-effects.md) particles
4. **Data layer** — `.dat` packs, skin RON, save files

## App state machine (high level)

`Boot → MainMenu → LevelSelect → Playing → Paused → LevelComplete | GameOver`

Managed via `bevy_state`. See [09-ui-and-states.md](09-ui-and-states.md).
