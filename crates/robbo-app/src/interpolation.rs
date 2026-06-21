use bevy::prelude::*;
use robbo_core::Cell;

use crate::bridge::CoreBridge;
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
    mut bridge: ResMut<CoreBridge>,
    mut query: Query<&mut VisualMotion, With<VisualEntityId>>,
) {
    if !bridge.animating {
        return;
    }
    let dt = time.delta_secs();
    let mut all_done = true;
    for mut motion in &mut query {
        if motion.progress < 1.0 {
            motion.progress = (motion.progress + dt / motion.duration).min(1.0);
            if motion.progress < 1.0 {
                all_done = false;
            }
        }
    }
    // Clear the animation lock when all tweens have finished (or when no
    // entity was tweening at all — e.g. a tick that only spawned projectiles
    // or emitted Shot/Collected events with no Moved events).
    if all_done {
        bridge.animating = false;
        bridge.events_queue.clear(); // safety clear; sync_visuals already drained
        if bridge.pending_input.is_none() {
            bridge.pending_input = bridge.queued_input.take();
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
