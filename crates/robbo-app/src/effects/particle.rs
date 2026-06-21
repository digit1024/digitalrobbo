use bevy::prelude::*;

/// Z layer for free-floating FX sprites (above tiles, alongside explosions).
pub const FX_Z_LAYER: f32 = 2.5;

/// Marker for pooled-style sprite particles (despawned when lifetime ends).
#[derive(Component)]
pub struct FxParticle;

#[derive(Component)]
pub struct FxParticleState {
    pub velocity: Vec2,
    pub lifetime: Timer,
    pub start_scale: f32,
    /// Per-second velocity damping (0 = none).
    pub drag: f32,
}

impl FxParticleState {
    pub fn new(velocity: Vec2, duration_secs: f32, start_scale: f32, drag: f32) -> Self {
        Self {
            velocity,
            lifetime: Timer::from_seconds(duration_secs, TimerMode::Once),
            start_scale,
            drag,
        }
    }
}
