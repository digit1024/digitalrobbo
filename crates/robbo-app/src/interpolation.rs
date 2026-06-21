use bevy::prelude::*;
use robbo_core::Cell;

use crate::projection::ActiveProjection;

#[derive(Component)]
pub struct VisualEntityId(pub u32);

#[derive(Component, Default)]
pub struct VisualMotion {
    pub from: Cell,
    pub to: Cell,
    pub progress: f32,
    pub duration: f32,
}

pub fn advance_interpolation_system(
    time: Res<Time>,
    mut query: Query<&mut VisualMotion, With<VisualEntityId>>,
) {
    let dt = time.delta_secs();
    for mut motion in &mut query {
        if motion.progress < 1.0 {
            motion.progress = (motion.progress + dt / motion.duration).min(1.0);
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
