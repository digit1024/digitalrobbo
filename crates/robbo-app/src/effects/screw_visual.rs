use bevy::prelude::*;

use crate::app_state::AppState;
use crate::bridge::CoreBridge;
use crate::interpolation::{VisualEntityId, VisualMotion, interpolated_pos};
use crate::projection::ActiveProjection;
use crate::render::LevelRoot;
use robbo_core::ElementKind;

use super::presets::spawn_ambient_screw_pixel;

const SCREW_SPIN_RAD_PER_SEC: f32 = 2.4;
const SCREW_BOB_PX: f32 = 4.0;
const SCREW_BOB_HZ: f32 = 2.2;
const SCREW_PARTICLE_INTERVAL_SECS: f32 = 0.11;

/// Idle spin, float, and orbiting sparkles on screw pickups.
#[derive(Component)]
pub struct ScrewVisual {
    pub phase: f32,
    pub particle_timer: Timer,
    pub particle_seed: u32,
}

impl ScrewVisual {
    pub fn from_cell(col: i16, row: i16) -> Self {
        let phase = (col as f32 * 1.7 + row as f32 * 2.9) * 0.31;
        Self {
            phase,
            particle_timer: Timer::from_seconds(
                SCREW_PARTICLE_INTERVAL_SECS * (0.8 + (col as u32 % 4) as f32 * 0.08),
                TimerMode::Repeating,
            ),
            particle_seed: (col as u32).wrapping_mul(31).wrapping_add(row as u32),
        }
    }
}

pub fn update_screw_visuals(
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut commands: Commands,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    level_roots: Query<Entity, With<LevelRoot>>,
    mut query: Query<(
        &VisualEntityId,
        &VisualMotion,
        &mut ScrewVisual,
        &mut Transform,
    )>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    let level_root = level_roots.iter().next();
    let tile = projection.tile_size();
    let t = time.elapsed_secs();

    for (entity_id, motion, mut screw, mut transform) in &mut query {
        let Some((cell, _)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == entity_id.0 && matches!(s.kind, ElementKind::Screw))
        else {
            continue;
        };

        screw.particle_timer.tick(time.delta());
        if screw.particle_timer.just_finished() {
            spawn_ambient_screw_pixel(
                &mut commands,
                &projection,
                level_root,
                *cell,
                tile,
                screw.particle_seed,
            );
            screw.particle_seed = screw.particle_seed.wrapping_add(1);
        }

        let bob = (t * SCREW_BOB_HZ + screw.phase).sin() * SCREW_BOB_PX;
        let pos = interpolated_pos(motion, &projection, 1.0);
        transform.translation = pos + Vec3::new(0.0, bob, 0.0);
        transform.rotation =
            Quat::from_rotation_z(t * SCREW_SPIN_RAD_PER_SEC + screw.phase);
        transform.scale = Vec3::ONE;
    }
}
