use bevy::prelude::*;
use robbo_core::{Cell, ElementKind, ElementState, enemy_move_delay, push_box_slide_delay};

use crate::bridge::{CoreBridge, GameTickTimer, TICK_SECS};
use crate::projection::ActiveProjection;

#[derive(Component)]
pub struct VisualEntityId(pub u32);

#[derive(Component, Clone, Copy)]
pub struct VisualMotion {
    pub from: Cell,
    pub to: Cell,
    pub progress: f32,
    /// Sim tick when this step began (`world.tick` at the `Moved` event).
    pub start_tick: u64,
    /// How many sim ticks this A→B glide spans (matches time until the next sim move).
    pub step_ticks: u32,
}

impl Default for VisualMotion {
    fn default() -> Self {
        Self {
            from: Cell::new(0, 0),
            to: Cell::new(0, 0),
            progress: 1.0,
            start_tick: 0,
            step_ticks: 1,
        }
    }
}

impl VisualMotion {
    pub fn settled(cell: Cell, tick: u64) -> Self {
        Self {
            from: cell,
            to: cell,
            progress: 1.0,
            start_tick: tick,
            step_ticks: 1,
        }
    }

    pub fn begin_step(from: Cell, to: Cell, start_tick: u64, step_ticks: u32) -> Self {
        Self {
            from,
            to,
            progress: 0.0,
            start_tick,
            step_ticks: step_ticks.max(1),
        }
    }

    pub fn retarget(&mut self, from: Cell, to: Cell, start_tick: u64, step_ticks: u32) {
        self.from = if self.from != self.to && self.progress < 1.0 {
            self.to
        } else {
            from
        };
        self.to = to;
        self.start_tick = start_tick;
        self.step_ticks = step_ticks.max(1);
        self.progress = 0.0;
    }
}

/// Sim ticks between moves for each element — must mirror robbo-core cadence.
pub fn visual_step_ticks(kind: &ElementKind, state: &ElementState) -> u32 {
    match kind {
        ElementKind::Bear { .. } | ElementKind::BlackBear { .. } | ElementKind::Bird { .. } => {
            enemy_move_delay(kind)
        }
        ElementKind::Butterfly => enemy_move_delay(kind),
        ElementKind::PushBox if state.sliding => push_box_slide_delay(),
        ElementKind::Gun { movable: true, .. } => 8,
        // Regular / gun bolts — one cell per sim tick (Robbo also steps every tick).
        ElementKind::Laser { solid: false, .. } => 1,
        // Robbo / bird / push-box shots use `Laser` + `tick_lasers_and_blasters` every tick.
        ElementKind::Projectile { .. } => 1,
        _ => 1,
    }
}

/// Fraction elapsed through the current sim tick (0 at tick start, →1 just before next tick).
pub fn tick_phase(tick_timer: &GameTickTimer) -> f32 {
    if TICK_SECS <= 0.0 {
        return 1.0;
    }
    (1.0 - tick_timer.0.remaining_secs() / TICK_SECS).clamp(0.0, 1.0)
}

/// Linear progress locked to the sim tick clock — same for Robbo, bears, bullets, lasers, guns.
pub fn advance_interpolation_system(
    bridge: Res<CoreBridge>,
    tick_timer: Res<GameTickTimer>,
    mut query: Query<&mut VisualMotion, With<VisualEntityId>>,
) {
    let tick_now = bridge.world.tick as f32 + tick_phase(&tick_timer);
    for mut motion in &mut query {
        if motion.from == motion.to {
            continue;
        }
        let elapsed = tick_now - motion.start_tick as f32;
        motion.progress = (elapsed / motion.step_ticks as f32).min(1.0);
        if motion.progress >= 1.0 {
            motion.from = motion.to;
            motion.progress = 1.0;
        }
    }
}

pub fn interpolated_pos(motion: &VisualMotion, projection: &ActiveProjection, layer: f32) -> Vec3 {
    let t = motion.progress.clamp(0.0, 1.0);
    let from = projection.project(motion.from, layer);
    let to = projection.project(motion.to, layer);
    from.lerp(to, t)
}
