use bevy::prelude::*;
use robbo_core::ElementKind;

use crate::bridge::{CoreBridge, GameTickTimer};
use crate::interpolation::{interpolated_pos, tick_phase, VisualEntityId, VisualMotion};
use crate::projection::ActiveProjection;

/// Full rotations per sim tick (not per frame). 0.05 → one turn every 20 sim ticks (~2s @ 10Hz).
const BUTTERFLY_SPIN_PER_TICK: f32 = 0.05;
/// Sim ticks for one red↔orange glow colour cycle.
const BUTTERFLY_GLOW_CYCLE_TICKS: f32 = 24.0;
/// Sim ticks for one beam size pulse.
const BUTTERFLY_BEAM_CYCLE_TICKS: f32 = 18.0;
const BUTTERFLY_GLOW_PX: f32 = 14.0;

/// Tick-synced spin + center glow for the single `butterfly.png` sprite.
#[derive(Component)]
pub struct ButterflyVisual {
    pub phase: f32,
}

impl ButterflyVisual {
    pub fn from_cell(col: i16, row: i16) -> Self {
        Self {
            phase: (col as f32 * 1.3 + row as f32 * 2.1) * 0.37,
        }
    }
}

#[derive(Component)]
pub struct ButterflyGlow;

pub fn attach_butterfly_visual(commands: &mut Commands, entity: Entity, col: i16, row: i16) {
    commands.entity(entity).insert(ButterflyVisual::from_cell(col, row));
    let glow = commands
        .spawn((
            Sprite {
                color: Color::srgba(1.0, 0.45, 0.08, 0.7),
                custom_size: Some(Vec2::splat(BUTTERFLY_GLOW_PX)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.05)),
            ButterflyGlow,
        ))
        .id();
    commands.entity(entity).add_child(glow);
}

pub fn update_butterfly_visuals(
    bridge: Res<CoreBridge>,
    tick_timer: Res<GameTickTimer>,
    projection: Res<ActiveProjection>,
    mut queries: ParamSet<(
        Query<(
            &VisualEntityId,
            &VisualMotion,
            &ButterflyVisual,
            &mut Transform,
            &Children,
        )>,
        Query<(&mut Transform, &mut Sprite), With<ButterflyGlow>>,
    )>,
) {
    let tick_now = bridge.world.tick as f32 + tick_phase(&tick_timer);
    let mut glow_updates: Vec<(Entity, Quat, Color, f32)> = Vec::new();

    for (entity_id, motion, visual, mut transform, children) in queries.p0().iter_mut() {
        let Some((_, state)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == entity_id.0)
        else {
            continue;
        };
        if !matches!(state.kind, ElementKind::Butterfly) {
            continue;
        }

        let spin = tick_now * BUTTERFLY_SPIN_PER_TICK * std::f32::consts::TAU + visual.phase;
        let pos = interpolated_pos(motion, &projection, 1.0);
        transform.translation = pos;
        transform.rotation = Quat::from_rotation_z(spin);
        transform.scale = Vec3::ONE;

        let glow_phase =
            tick_now * std::f32::consts::TAU / BUTTERFLY_GLOW_CYCLE_TICKS + visual.phase;
        let pulse = glow_phase.sin();
        let blend = pulse * 0.5 + 0.5;
        let red = 0.95 * (1.0 - blend) + 1.0 * blend;
        let green = 0.1 * (1.0 - blend) + 0.52 * blend;
        let alpha = 0.55 + 0.3 * pulse.abs();
        let beam_phase =
            tick_now * std::f32::consts::TAU / BUTTERFLY_BEAM_CYCLE_TICKS + visual.phase;
        let beam = 0.85 + 0.15 * beam_phase.sin().powi(2);
        let color = Color::srgba(red, green, 0.06, alpha);

        for child in children.iter() {
            glow_updates.push((*child, Quat::from_rotation_z(-spin), color, beam));
        }
    }

    for (child, rotation, color, beam) in glow_updates {
        if let Ok((mut glow_tf, mut glow_sprite)) = queries.p1().get_mut(child) {
            glow_tf.rotation = rotation;
            glow_sprite.color = color;
            glow_sprite.custom_size = Some(Vec2::splat(BUTTERFLY_GLOW_PX * beam));
        }
    }
}
