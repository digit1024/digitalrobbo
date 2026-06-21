use bevy::prelude::*;
use robbo_core::GameEvent;

use crate::app_state::AppState;
use crate::bridge::CoreGameEvent;
use crate::projection::ActiveProjection;
use crate::render::LevelRoot;

use super::particle::{FxParticle, FxParticleState};
use super::presets::spawn_teleport_burst;

/// Observer: react to core events with short-lived particles (no sim impact).
pub fn fx_on_core_events(
    state: Res<State<AppState>>,
    mut commands: Commands,
    mut reader: EventReader<CoreGameEvent>,
    projection: Res<ActiveProjection>,
    level_roots: Query<Entity, With<LevelRoot>>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    let level_root = level_roots.iter().next();
    let tile = projection.tile_size();

    for CoreGameEvent(event) in reader.read() {
        match event {
            GameEvent::Teleported { from, to, .. } => {
                spawn_teleport_burst(&mut commands, &projection, level_root, *from, tile);
                spawn_teleport_burst(&mut commands, &projection, level_root, *to, tile);
            }
            // GameEvent::Exploded { at } => { ... FxPreset::Explosion }
            // GameEvent::Shot { from, direction } => { ... FxPreset::ShotTrail }
            _ => {}
        }
    }
}

pub fn tick_fx_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut FxParticleState), With<FxParticle>>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut sprite, mut state) in &mut query {
        state.lifetime.tick(time.delta());
        let t = state.lifetime.fraction().clamp(0.0, 1.0);

        transform.translation += state.velocity.extend(0.0) * dt;
        if state.drag > 0.0 {
            let damp = (1.0 - state.drag * dt).clamp(0.0, 1.0);
            state.velocity *= damp;
        }

        let scale = state.start_scale * (1.0 - t * 0.6);
        transform.scale = Vec3::splat(scale.max(0.05));
        sprite.color = sprite.color.with_alpha((1.0 - t).max(0.0));

        if state.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}
