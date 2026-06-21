use bevy::prelude::*;
use robbo_core::{Direction, ElementKind};

use crate::app_state::AppState;
use crate::bridge::CoreBridge;
use crate::bridge::GameTickTimer;
use crate::interpolation::{interpolated_pos, tick_phase, VisualEntityId, VisualMotion};
use crate::projection::ActiveProjection;

/// Procedural laser / blaster rendering (no bullet-pickup sprite).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileVisual {
    /// gnurobbo `LASER_*` with `solidlaser == 0` — moving bolt, toggles state each tick.
    Bolt,
    /// gnurobbo solid laser beam segment — fixed red pipe from laser gun.
    SolidBeam,
    /// gnurobbo `BLASTER` — expanding wave, `state` 0..3 then removed.
    Blaster,
}

pub fn projectile_visual_for(kind: &ElementKind) -> Option<ProjectileVisual> {
    match kind {
        ElementKind::Laser { solid: false, .. } => Some(ProjectileVisual::Bolt),
        ElementKind::Laser { solid: true, .. } => Some(ProjectileVisual::SolidBeam),
        ElementKind::BlasterCell { .. } => Some(ProjectileVisual::Blaster),
        _ => None,
    }
}

fn axis_size(direction: Direction, tile: f32, length: f32, thickness: f32) -> Vec2 {
    match direction {
        Direction::Left | Direction::Right => Vec2::new(tile * length, tile * thickness),
        Direction::Up | Direction::Down => Vec2::new(tile * thickness, tile * length),
    }
}

/// Spawn sprite for a laser bolt / beam cell / blaster wave (color quad, no texture).
pub fn projectile_sprite_bundle(
    kind: &ElementKind,
    direction: Direction,
    tile: f32,
) -> (Sprite, ProjectileVisual) {
    let visual = projectile_visual_for(kind).expect("projectile kind");
    let (color, size) = match visual {
        ProjectileVisual::Bolt => (
            Color::srgb(1.0, 0.92, 0.55),
            axis_size(direction, tile, 0.82, 0.26),
        ),
        ProjectileVisual::SolidBeam => (
            Color::srgba(1.0, 0.12, 0.06, 0.92),
            axis_size(direction, tile, 0.94, 0.24),
        ),
        ProjectileVisual::Blaster => (
            Color::srgba(1.0, 0.55, 0.12, 0.88),
            Vec2::splat(tile * 0.62),
        ),
    };
    (
        Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        visual,
    )
}

pub fn update_projectile_visuals(
    state: Res<State<AppState>>,
    time: Res<Time>,
    tick_timer: Res<GameTickTimer>,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    mut query: Query<(
        &VisualEntityId,
        &VisualMotion,
        &ProjectileVisual,
        &mut Sprite,
        &mut Transform,
    )>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    let tile = projection.tile_size();
    let t = time.elapsed_secs();
    let tick_now = bridge.world.tick as f32 + tick_phase(&tick_timer);

    for (entity_id, motion, visual, mut sprite, mut transform) in &mut query {
        let Some((_, el)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == entity_id.0)
        else {
            continue;
        };

        let direction = match &el.kind {
            ElementKind::Laser { direction, .. } => *direction,
            ElementKind::BlasterCell { direction, .. } => *direction,
            _ => Direction::Right,
        };

        transform.translation = interpolated_pos(motion, &projection, 1.15);
        transform.rotation = Quat::IDENTITY;

        match *visual {
            ProjectileVisual::Bolt => {
                // GNU `negate_state` on moving lasers — blink each sim tick.
                let blink = if (bridge.world.tick & 1) == 0 { 1.0 } else { 0.72 };
                let head = motion.progress.clamp(0.0, 1.0);
                let lead_boost = 0.85 + head * 0.15;
                sprite.custom_size = Some(axis_size(direction, tile, 0.82, 0.26));
                sprite.color = Color::srgb(1.0 * blink * lead_boost, 0.88 * blink, 0.45 * blink);
                transform.scale = Vec3::splat(1.0);
            }
            ProjectileVisual::SolidBeam => {
                let pulse = 0.78 + 0.22 * (t * 7.5 + tick_now * 0.3).sin();
                sprite.custom_size = Some(axis_size(direction, tile, 0.94, 0.24));
                sprite.color = Color::srgba(1.0, 0.1 + pulse * 0.08, 0.05, 0.55 + pulse * 0.4);
                transform.scale = Vec3::splat(1.0);
            }
            ProjectileVisual::Blaster => {
                let frame = match el.kind {
                    ElementKind::BlasterCell { frame, .. } => frame,
                    _ => 0,
                };
                let spread = 0.52 + frame as f32 * 0.16;
                let fade = 1.0 - frame as f32 * 0.18;
                sprite.custom_size = Some(Vec2::splat(tile * 0.55));
                sprite.color = Color::srgba(1.0, 0.45 + frame as f32 * 0.06, 0.08, 0.75 * fade);
                transform.scale = Vec3::splat(spread);
                transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
            }
        }
    }
}
