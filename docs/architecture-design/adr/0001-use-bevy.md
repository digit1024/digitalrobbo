# ADR 0001: Use Bevy as game engine

> **Status:** Accepted  
> **Date:** 2026-06-20

## Context

Need a Rust game engine with ECS, 2D rendering, cross-platform (desktop/web/mobile), and active ecosystem.

## Decision

Use **Bevy 0.18.x** for `robbo-app`. Keep simulation in a separate crate without Bevy dependency.

## Consequences

- Fast iteration with ECS patterns
- Breaking Bevy releases ~every 3 months; pin version and use migration guides
- Mobile support requires extra validation vs desktop
