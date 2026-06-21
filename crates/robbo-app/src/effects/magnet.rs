use bevy::prelude::*;
use robbo_core::{Direction, ElementKind};

use crate::app_state::AppState;
use crate::assets::magnet_direction_rotation;
use crate::bridge::{CoreBridge, ReloadVisualsEvent};
use crate::interpolation::{interpolated_pos, VisualEntityId, VisualMotion};
use crate::projection::ActiveProjection;
use crate::render::LevelRoot;

/// Horseshoe magnet art faces up; rotation tracks sim `Magnet::direction`.
#[derive(Component)]
pub struct MagnetVisual {
    pub rotation: f32,
    pub last_direction: Direction,
}

impl MagnetVisual {
    pub fn new(direction: Direction) -> Self {
        let rotation = magnet_direction_rotation(direction);
        Self {
            rotation,
            last_direction: direction,
        }
    }
}

#[derive(Component)]
struct MagnetBeamSegment;

#[derive(Resource, Default)]
pub struct MagnetBeamCache(pub(crate) Vec<Entity>);

fn magnet_facing(kind: &ElementKind) -> Option<Direction> {
    match kind {
        ElementKind::Magnet { direction } => Some(*direction),
        _ => None,
    }
}

pub fn clear_magnet_beams_on_reload(
    mut reload: EventReader<ReloadVisualsEvent>,
    mut cache: ResMut<MagnetBeamCache>,
    mut commands: Commands,
) {
    if reload.read().next().is_none() {
        return;
    }
    for entity in cache.0.drain(..) {
        commands.entity(entity).despawn();
    }
}

pub fn update_magnet_visuals(
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    mut query: Query<(
        &VisualEntityId,
        &VisualMotion,
        &mut MagnetVisual,
        &mut Transform,
    )>,
) {
    for (entity_id, motion, mut visual, mut transform) in &mut query {
        let Some((_, state)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == entity_id.0)
        else {
            continue;
        };
        let Some(dir) = magnet_facing(&state.kind) else {
            continue;
        };

        if dir != visual.last_direction {
            visual.rotation = magnet_direction_rotation(dir);
            visual.last_direction = dir;
        }

        transform.translation = interpolated_pos(motion, &projection, 1.0);
        transform.rotation = Quat::from_rotation_z(visual.rotation);
    }
}

/// Red attraction beam — same ray as `World::magnet_beam_cells` (blocked by walls/objects).
pub fn update_magnet_beams(
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut commands: Commands,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    level_roots: Query<Entity, With<LevelRoot>>,
    mut cache: ResMut<MagnetBeamCache>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    for entity in cache.0.drain(..) {
        commands.entity(entity).despawn();
    }

    let Some(root) = level_roots.iter().next() else {
        return;
    };

    let tile = projection.tile_size();
    let pulse = 0.38 + 0.14 * (time.elapsed_secs() * 5.0).sin();

    for (mag_cell, state) in &bridge.world.elements {
        let ElementKind::Magnet { direction } = &state.kind else {
            continue;
        };
        let cells = bridge.world.magnet_beam_cells(*mag_cell, *direction);
        let count = cells.len().max(1) as f32;
        let beam_width = tile * 0.4;
        let segment_size = match direction {
            Direction::Left | Direction::Right => Vec2::new(tile * 0.92, beam_width),
            Direction::Up | Direction::Down => Vec2::new(beam_width, tile * 0.92),
        };
        let rot = Quat::from_rotation_z(magnet_direction_rotation(*direction));

        for (i, cell) in cells.iter().enumerate() {
            let fade = 1.0 - (i as f32 / count) * 0.3;
            let alpha = (pulse * fade).clamp(0.12, 0.55);
            let pos = projection.project(*cell, 0.55);
            let entity = commands
                .spawn((
                    Sprite {
                        color: Color::srgba(1.0, 0.1, 0.06, alpha),
                        custom_size: Some(segment_size),
                        ..default()
                    },
                    Transform::from_translation(pos).with_rotation(rot),
                    MagnetBeamSegment,
                ))
                .id();
            commands.entity(root).add_child(entity);
            cache.0.push(entity);
        }
    }
}
