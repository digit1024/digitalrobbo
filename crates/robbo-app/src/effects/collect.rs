use bevy::prelude::*;
use robbo_core::{Cell, ElementKind};
use crate::projection::ActiveProjection;

const COLLECT_POP_SECS: f32 = 0.42;
const COLLECT_POP_Z: f32 = 2.2;

/// Short grow-and-fade sprite when Robbo collects a pickup.
#[derive(Component)]
pub struct CollectPopEffect {
    pub timer: Timer,
    pub base_y: f32,
}

pub fn is_collect_pop_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Screw | ElementKind::Key | ElementKind::BulletPickup
    )
}

pub fn spawn_collect_pop(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    _kind: &ElementKind,
    tile: f32,
    image: Handle<Image>,
) {
    let pos = projection.project(at, COLLECT_POP_Z);
    let mut entity = commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::splat(tile)),
            color: Color::WHITE,
            ..default()
        },
        Transform::from_translation(pos),
        CollectPopEffect {
            timer: Timer::from_seconds(COLLECT_POP_SECS, TimerMode::Once),
            base_y: pos.y,
        },
    ));
    if let Some(root) = level_root {
        entity.set_parent(root);
    }
}

pub fn tick_collect_pop_effects(
    time: Res<Time>,
    mut commands: Commands,
    projection: Res<ActiveProjection>,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut CollectPopEffect)>,
) {
    let tile = projection.tile_size();
    for (entity, mut transform, mut sprite, mut effect) in &mut query {
        effect.timer.tick(time.delta());
        let t = (effect.timer.elapsed_secs() / COLLECT_POP_SECS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(2);

        transform.scale = Vec3::splat(1.0 + eased * 0.85);
        transform.translation.y = effect.base_y + eased * tile * 0.22;
        sprite.color = sprite.color.with_alpha((1.0 - t).max(0.0));

        if effect.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}
