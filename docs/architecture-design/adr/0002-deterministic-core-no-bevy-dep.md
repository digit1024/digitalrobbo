# ADR 0002: Deterministic core without Bevy dependency

> **Status:** Accepted  
> **Date:** 2026-06-20

## Context

Smooth animations require separating logical grid state from visual interpolation. Testing and fidelity require deterministic, headless simulation.

## Decision

All game logic lives in `robbo-core` with **no Bevy dependency**. Positions are integer cells; no floats in simulation.

## Consequences

- Full unit testing without GPU/window
- Clean boundary for future isometric renderer
- Bridge layer in `robbo-app` must sync ECS to core state each step
