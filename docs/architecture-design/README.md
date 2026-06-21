# DigitalRobbo — Architecture Design

> **Status:** Draft  
> **Last updated:** M7

Living architecture documentation for the GNU Robbo remake. These docs evolve with the project — **update the relevant doc at the end of every milestone**.

## How to read

1. Start with [01-overview.md](01-overview.md) for vision and constraints.
2. Read [02-architecture.md](02-architecture.md) for crate boundaries and data flow.
3. Dive into topic docs as needed during implementation.

## How to keep docs current

| When | Action |
|------|--------|
| End of milestone | Update status header + "Last updated" line in affected docs |
| New ADR | Add file under `adr/` with next sequential number |
| New game element | Add row to [07-element-catalog.md](07-element-catalog.md) |
| API change in core | Update [03-core-simulation.md](03-core-simulation.md) |

## Status legend

- **Draft** — initial scaffold, may change freely
- **Stable** — reviewed and aligned with implemented code
- **Deprecated** — superseded; kept for history

## Table of contents

| Doc | Topic |
|-----|-------|
| [01-overview.md](01-overview.md) | Vision, goals, C4 context, quality attributes |
| [02-architecture.md](02-architecture.md) | Crates, layers, dependency rules, data flow |
| [03-core-simulation.md](03-core-simulation.md) | Deterministic tick model, World::step, undo |
| [04-view-decoupling.md](04-view-decoupling.md) | GridProjection seam, interpolation, camera |
| [05-design-patterns.md](05-design-patterns.md) | Command, State machine, Observer, Object pool |
| [06-level-format.md](06-level-format.md) | gnurobbo `.dat` format, tile legend, parser |
| [07-element-catalog.md](07-element-catalog.md) | All game elements and parity notes |
| [08-assets-skins.md](08-assets-skins.md) | Skin manifest, atlases, palette |
| [09-ui-and-states.md](09-ui-and-states.md) | Screen flow, HUD, settings |
| [10-persistence.md](10-persistence.md) | Save model, progress, best times |
| [glossary.md](glossary.md) | Domain terms |
| [adr/](adr/) | Architecture Decision Records |
