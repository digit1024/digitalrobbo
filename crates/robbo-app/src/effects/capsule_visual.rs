use bevy::prelude::*;
use robbo_core::ElementKind;

use crate::app_state::AppState;
use crate::assets::SpriteAssets;
use crate::bridge::CoreBridge;
use crate::interpolation::{interpolated_pos, VisualEntityId, VisualMotion};
use crate::projection::ActiveProjection;

const CAPSULE_BREATHE_HZ: f32 = 1.35;
const CAPSULE_SCALE_AMP: f32 = 0.1;
const CAPSULE_GLOW_AMP: f32 = 0.18;

/// Breathing “exit ready” animation once all screws are collected.
#[derive(Component)]
pub struct CapsuleVisual {
    pub phase: f32,
}

impl CapsuleVisual {
    pub fn from_cell(col: i16, row: i16) -> Self {
        Self {
            phase: (col as f32 * 2.1 + row as f32 * 1.3) * 0.4,
        }
    }
}

pub fn update_capsule_visuals(
    state: Res<State<AppState>>,
    time: Res<Time>,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    assets: Option<Res<SpriteAssets>>,
    mut query: Query<(
        &VisualEntityId,
        &VisualMotion,
        &CapsuleVisual,
        &mut Sprite,
        &mut Transform,
    )>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    let Some(sa) = assets else {
        return;
    };

    let ready = bridge.world.capsule_open;
    let t = time.elapsed_secs();

    for (entity_id, motion, capsule, mut sprite, mut transform) in &mut query {
        let Some((_, _el)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == entity_id.0 && matches!(s.kind, ElementKind::Capsule))
        else {
            continue;
        };

        let pos = interpolated_pos(motion, &projection, 1.0);
        transform.translation = pos;

        if ready {
            let wave = (t * CAPSULE_BREATHE_HZ * std::f32::consts::TAU + capsule.phase).sin();
            let breathe = (wave + 1.0) * 0.5;
            transform.scale = Vec3::splat(1.0 + CAPSULE_SCALE_AMP * breathe);

            let glow = 1.0 + CAPSULE_GLOW_AMP * breathe;
            sprite.image = sa.capsule_ready.clone();
            sprite.color = Color::srgb(glow, glow * 0.98, glow * 0.88);
        } else {
            transform.scale = Vec3::ONE;
            sprite.image = sa.capsule.clone();
            sprite.color = Color::WHITE;
        }
    }
}
