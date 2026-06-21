# ADR 0004: All three platforms as first-class targets

> **Status:** Accepted  
> **Date:** 2026-06-20

## Context

Desired deployment on desktop, web, and mobile.

## Decision

Design for all three from architecture start (object pools, save abstraction, touch input). **Validate desktop first**, then WASM, then mobile templates in M6.

## Consequences

- Avoid unbounded spawn/despawn in VFX
- Abstract persistence early
- Mobile may lag desktop in polish
