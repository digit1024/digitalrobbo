# 04 — View Decoupling

> **Status:** Draft  
> **Last updated:** M-DOC

## Problem

Original Robbo is instantaneous grid jumps. Modern UX expects smooth movement. Isometric 2.5D is a future goal. **Solution:** separate logical state from visual interpolation via a projection trait.

## GridProjection trait

```rust
pub trait GridProjection: Send + Sync {
    fn cell_to_world(&self, cell: Cell, layer: f32) -> Vec3;
    fn sort_key(&self, cell: Cell) -> f32;
    fn tile_size(&self) -> f32;
}
```

Implementations:

- **`TopDownProjection`** (v1) — orthographic, Y-down or Y-up consistent with Bevy
- **`IsometricProjection`** (M7) — dimetric/isometric cell layout + depth sort

## Interpolation model

Each visual entity carries:

```rust
struct VisualMotion {
    from: Cell,
    to: Cell,
    progress: f32,  // 0.0 .. 1.0
    easing: Easing,
}
```

Each frame (render):

```
world_pos = lerp(
    projection.cell_to_world(from, layer),
    projection.cell_to_world(to, layer),
    ease(progress)
)
```

When `progress >= 1.0`, logical position equals `to`; next buffered input may commit.

## Input buffering

While `progress < 1.0`, queue at most one pending `PlayerInput`. On completion, auto-apply if queue non-empty. Keeps responsiveness without overlapping logical steps.

## Forbidden in logic layer

- `Vec2`, `Vec3`, pixel coordinates
- Bevy `Transform`, `Sprite`
- Frame delta time affecting simulation outcomes

## Camera contract

- Camera reads bounds from active `GridProjection` over level width/height.
- **Fit-to-viewport** with letterboxing for small levels.
- **Smooth follow** optional for 32-wide levels.
- **Pixel-perfect** integer scale option for crisp pixel art.
- Same camera code works for top-down and iso — only projection changes.

## Animation timings (defaults, tunable)

| Action | Duration | Notes |
|--------|----------|-------|
| Walk | 140 ms | ease-in-out |
| Push | 140 ms | Robbo + box synchronized |
| Shoot arm | 80 ms | then spawn projectile |
| Projectile | per-cell | original speed parity |
| Teleport | 200 ms | fade out + in |
| Death | 300 ms | before respawn/restart |
