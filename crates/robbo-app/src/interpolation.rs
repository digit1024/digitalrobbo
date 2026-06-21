use bevy::prelude::*;
use robbo_core::Cell;

use crate::bridge::TICK_SECS;
use crate::projection::ActiveProjection;

#[derive(Component)]
pub struct VisualEntityId(pub u32);

#[derive(Component, Default)]
pub struct VisualMotion {
    pub from: Cell,
    pub to: Cell,
    pub progress: f32,
    /// Wall-clock length of one grid step — equals one sim tick.
    pub duration: f32,
}

/// Advance each entity's step tween independently (Robbo, enemies, bullets, boxes).
/// Progress only resets when `sync_visuals` assigns a new move — not on every sim tick.
pub fn advance_interpolation_system(
    time: Res<Time>,
    mut query: Query<&mut VisualMotion, With<VisualEntityId>>,
) {
    let dt = time.delta_secs();
    for mut motion in &mut query {
        if motion.from == motion.to {
            continue;
        }
        let duration = motion.duration.max(TICK_SECS);
        motion.progress = (motion.progress + dt / duration).min(1.0);
        if motion.progress >= 1.0 {
            motion.from = motion.to;
            motion.progress = 1.0;
        }
    }
}

pub fn eased(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn interpolated_pos(motion: &VisualMotion, projection: &ActiveProjection, layer: f32) -> Vec3 {
    let t = eased(motion.progress);
    let from = projection.project(motion.from, layer);
    let to = projection.project(motion.to, layer);
    from.lerp(to, t)
}
