use bevy::prelude::*;
use robbo_core::{Cell, Direction, ElementKind};

use crate::app_state::AppState;
use crate::assets::magnet_direction_rotation;
use crate::bridge::{CoreBridge, ReloadVisualsEvent};
use crate::interpolation::{interpolated_pos, VisualEntityId, VisualMotion};
use crate::projection::ActiveProjection;
use crate::render::LevelRoot;

const BEAM_Z: f32 = 0.55;

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
pub(crate) struct MagnetBeamSegment(f32);

/// Tracks when beam geometry was last rebuilt (segments respawn each sim tick).
#[derive(Resource, Default)]
pub struct MagnetBeams {
    pub last_sim_tick: u64,
}

pub fn reset_magnet_beams(beams: &mut MagnetBeams) {
    beams.last_sim_tick = u64::MAX;
}

pub fn reset_magnet_beams_on_reload(
    mut reload: MessageReader<ReloadVisualsEvent>,
    mut beams: ResMut<MagnetBeams>,
) {
    if reload.read().next().is_some() {
        reset_magnet_beams(&mut beams);
    }
}

fn magnet_facing(kind: &ElementKind) -> Option<Direction> {
    match kind {
        ElementKind::Magnet { direction } => Some(*direction),
        _ => None,
    }
}

fn beam_angle(mag_cell: Cell, direction: Direction, projection: &ActiveProjection) -> f32 {
    let (dc, dr) = direction.delta();
    let from = projection.project(mag_cell, BEAM_Z);
    let to = projection.project(mag_cell.offset(dc, dr), BEAM_Z);
    (to.y - from.y).atan2(to.x - from.x)
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

/// Red attraction beam — one segment per illuminated cell; full rebuild each sim tick.
pub fn update_magnet_beams(
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut commands: Commands,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    level_roots: Query<Entity, With<LevelRoot>>,
    mut magnet_beams: ResMut<MagnetBeams>,
    segments: Query<(Entity, &MagnetBeamSegment), With<MagnetBeamSegment>>,
    mut sprites: Query<(&MagnetBeamSegment, &mut Sprite)>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    let pulse = 0.38 + 0.14 * (time.elapsed_secs() * 5.0).sin();

    for (fade, mut sprite) in &mut sprites {
        sprite.color = Color::srgba(
            1.0,
            0.1,
            0.06,
            (pulse * fade.0).clamp(0.12, 0.55),
        );
    }

    let sim_tick = bridge.world.tick;
    if sim_tick == magnet_beams.last_sim_tick {
        return;
    }
    magnet_beams.last_sim_tick = sim_tick;

    for (entity, _) in &segments {
        commands.entity(entity).despawn();
    }

    let Some(root) = level_roots.iter().next() else {
        return;
    };

    let tile = projection.tile_size();
    let segment_size = Vec2::new(tile * 0.92, tile * 0.4);

    for (mag_cell, el) in &bridge.world.elements {
        let ElementKind::Magnet { direction } = &el.kind else {
            continue;
        };
        let cells = bridge.world.magnet_beam_cells(*mag_cell, *direction);
        if cells.is_empty() {
            continue;
        }

        let rot = Quat::from_rotation_z(beam_angle(*mag_cell, *direction, &projection));
        let count = cells.len() as f32;

        for (i, cell) in cells.iter().enumerate() {
            let fade = 1.0 - (i as f32 / count) * 0.3;
            let pos = projection.project(*cell, BEAM_Z);
            let entity = commands
                .spawn((
                    Sprite {
                        color: Color::srgba(1.0, 0.1, 0.06, (pulse * fade).clamp(0.12, 0.55)),
                        custom_size: Some(segment_size),
                        ..default()
                    },
                    Transform::from_translation(pos).with_rotation(rot),
                    MagnetBeamSegment(fade),
                ))
                .id();
            commands.entity(root).add_child(entity);
        }
    }
}
