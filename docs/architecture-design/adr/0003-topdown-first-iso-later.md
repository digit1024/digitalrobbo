# ADR 0003: Top-down first, isometric later

> **Status:** Accepted  
> **Date:** 2026-06-20

## Context

End goal is 2.5D isometric presentation. Building iso art and camera now delays playable prototype.

## Decision

Ship v1 with **TopDownProjection**. Introduce **IsometricProjection** in M7 as a swappable `GridProjection` implementation + iso skin pack. No changes to `robbo-core`.

## Consequences

- Faster path to playable original levels
- Asset pipeline must use logical names from day one
- Camera and depth sort abstracted via trait early
