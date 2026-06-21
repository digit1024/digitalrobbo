use bevy::prelude::*;
use robbo_core::Cell;

use crate::projection::ActiveProjection;

use super::particle::{FxParticle, FxParticleState, FX_Z_LAYER};

/// Identifies a reusable burst recipe. Extend when adding explosion / shot trails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxPreset {
    Teleport,
    // Explosion,
    // ShotTrail,
}

const TELEPORT_PARTICLE_COUNT: u32 = 16;
const TELEPORT_DURATION_SECS: f32 = 0.65;
const TELEPORT_SPEED: f32 = 42.0;

/// Cyan / violet sparkles at entry and exit portals.
pub fn spawn_teleport_burst(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    tile: f32,
) {
    spawn_burst(
        commands,
        projection,
        level_root,
        at,
        tile,
        FxPreset::Teleport,
    );
}

fn spawn_burst(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    tile: f32,
    preset: FxPreset,
) {
    let (count, duration, speed, size, colors, drag) = match preset {
        FxPreset::Teleport => (
            TELEPORT_PARTICLE_COUNT,
            TELEPORT_DURATION_SECS,
            TELEPORT_SPEED,
            tile * 0.22,
            [
                Color::srgba(0.35, 0.95, 1.0, 0.95),
                Color::srgba(0.75, 0.45, 1.0, 0.9),
                Color::srgba(0.95, 0.95, 1.0, 0.85),
            ],
            1.8,
        ),
    };

    let origin = projection.project(at, FX_Z_LAYER);
    for i in 0..count {
        let angle = i as f32 * 2.399_963_2;
        let wobble = ((i * 17) % 7) as f32 * 0.08;
        let speed_scale = 0.55 + wobble;
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed * speed_scale;
        // Drift upward on screen (projection Y increases upward).
        let velocity = velocity + Vec2::new(0.0, speed * 0.35);

        let color = colors[i as usize % colors.len()];
        let offset = Vec2::new(
            (i as f32 * 1.7).sin() * tile * 0.15,
            (i as f32 * 2.3).cos() * tile * 0.15,
        );

        let mut entity = commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            Transform::from_translation(origin + offset.extend(0.0)),
            FxParticle,
            FxParticleState::new(velocity, duration, 1.0, drag),
        ));
        if let Some(root) = level_root {
            entity.set_parent(root);
        }
    }
}
