use bevy::prelude::*;
use robbo_core::{Cell, GameEvent, TileKind};

use crate::assets::{SpriteAssets, dir_to_idx, TILE_PX};
use crate::bridge::{CoreBridge, EntityMap, ReloadVisualsEvent};
use crate::interpolation::{VisualEntityId, VisualMotion, interpolated_pos};
use crate::projection::ActiveProjection;

// ─────────────────────────────────────────────────────────────────────────────
// Components
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct TileSprite {
    pub tile_kind: TileKind,
}

#[derive(Component)]
pub struct LevelRoot;

#[derive(Component)]
pub struct ExplosionEffect {
    pub timer: Timer,
}

const EXPLOSION_SECS: f32 = 0.35;

fn spawn_explosion_effect(
    commands: &mut Commands,
    projection: &ActiveProjection,
    cell: Cell,
    level_root: Option<Entity>,
) {
    let pos = projection.project(cell, 2.0);
    let tile = projection.tile_size();
    let mut entity = commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 0.55, 0.05, 0.95),
            custom_size: Some(Vec2::splat(tile * 1.4)),
            ..default()
        },
        Transform::from_translation(pos).with_scale(Vec3::splat(0.15)),
        ExplosionEffect {
            timer: Timer::from_seconds(EXPLOSION_SECS, TimerMode::Once),
        },
    ));
    if let Some(root) = level_root {
        entity.set_parent(root);
    }
}

pub fn update_explosion_effects(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut ExplosionEffect)>,
) {
    for (entity, mut transform, mut sprite, mut effect) in &mut query {
        effect.timer.tick(time.delta());
        let t = (effect.timer.elapsed_secs() / EXPLOSION_SECS).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(0.15 + t * 1.35);
        sprite.color = Color::srgba(1.0, 0.55, 0.05, 0.95 * (1.0 - t));
        if effect.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn sync_visuals(
    mut bridge: ResMut<CoreBridge>,
    projection: Res<ActiveProjection>,
    assets: Option<Res<SpriteAssets>>,
    mut entity_map: ResMut<EntityMap>,
    mut commands: Commands,
    mut motion_q: Query<(&VisualEntityId, &mut VisualMotion)>,
    level_roots: Query<Entity, With<LevelRoot>>,
) {
    // Only run when a new tick just fired (events waiting).
    // advance_interpolation_system handles tween progression every frame.
    if bridge.events_queue.is_empty() {
        return;
    }

    let current_ids: std::collections::HashSet<u32> =
        bridge.world.elements.iter().map(|(_, s)| s.id).collect();

    // despawn removed elements
    entity_map.0.retain(|id, entity| {
        if current_ids.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });

    // spawn new elements that appeared this tick
    let tile = projection.tile_size();
    let level_root = level_roots.iter().next();
    for (cell, state) in &bridge.world.elements {
        if entity_map.0.contains_key(&state.id) {
            continue;
        }
        let pos = projection.project(*cell, 1.0);
        let spawn = if let Some(ref sa) = assets {
            commands.spawn((
                Sprite {
                    image: sa.for_element(&state.kind, state.direction),
                    custom_size: Some(Vec2::splat(tile)),
                    ..default()
                },
                Transform::from_translation(pos),
                VisualMotion { from: *cell, to: *cell, progress: 1.0, duration: crate::bridge::ANIM_SECS },
                VisualEntityId(state.id),
            ))
        } else {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::splat(tile * 0.8)),
                    color: fallback_entity_color(&state.kind),
                    ..default()
                },
                Transform::from_translation(pos),
                VisualMotion { from: *cell, to: *cell, progress: 1.0, duration: crate::bridge::ANIM_SECS },
                VisualEntityId(state.id),
            ))
        };
        let id = spawn.id();
        if let Some(root) = level_root {
            commands.entity(id).set_parent(root);
        }
        entity_map.0.insert(state.id, id);
    }

    // Set motion targets from events, then drain the queue.
    if !bridge.events_queue.is_empty() {
        for event in &bridge.events_queue {
            match event {
                GameEvent::Exploded { at } | GameEvent::Revealed { at } => {
                    spawn_explosion_effect(
                        &mut commands,
                        &projection,
                        *at,
                        level_roots.iter().next(),
                    );
                }
                GameEvent::Moved { entity_id, from, to }
                | GameEvent::Pushed { entity_id, from, to }
                | GameEvent::Teleported { entity_id, from, to } => {
                    if let Some(entity) = entity_map.0.get(entity_id) {
                        if let Ok((_, mut motion)) = motion_q.get_mut(*entity) {
                            // Chain from current visual cell when mid-step or already arrived.
                            motion.from = if motion.from != motion.to && motion.progress < 1.0 {
                                motion.to
                            } else {
                                *from
                            };
                            motion.to = *to;
                            motion.progress = 0.0;
                            motion.duration = crate::bridge::ANIM_SECS;
                        }
                    }
                }
                _ => {}
            }
        }
        bridge.events_queue.clear();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Robbo sprite animation (runs every frame, independent of ticks)
// ─────────────────────────────────────────────────────────────────────────────

/// Updates the Robbo sprite image every frame to reflect the current facing
/// direction and walk-cycle frame.  Runs after sync_visuals so the entity is
/// guaranteed to exist in entity_map.
pub fn update_robbo_sprite(
    bridge: Res<CoreBridge>,
    entity_map: Res<EntityMap>,
    assets: Option<Res<SpriteAssets>>,
    mut query: Query<(&VisualEntityId, &mut Sprite)>,
) {
    let Some(ref sa) = assets else { return; };
    let robbo_id = bridge.world.robbo_id;
    if robbo_id == 0 {
        return;
    }
    let Some(entity) = entity_map.0.get(&robbo_id) else {
        return;
    };
    let Ok((_, mut sprite)) = query.get_mut(*entity) else {
        return;
    };
    let dir_idx = dir_to_idx(bridge.facing_direction);
    let frame = bridge.walk_frame;
    sprite.image = sa.player[dir_idx][frame].clone();
}

// ─────────────────────────────────────────────────────────────────────────────
// Transform update from interpolated position
// ─────────────────────────────────────────────────────────────────────────────

pub fn update_entity_transforms(
    projection: Res<ActiveProjection>,
    mut query: Query<(&VisualMotion, &mut Transform), With<VisualEntityId>>,
) {
    for (motion, mut transform) in &mut query {
        *transform = Transform::from_translation(interpolated_pos(motion, &projection, 1.0));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Full level rebuild (on ReloadVisualsEvent, after load_level_system)
// ─────────────────────────────────────────────────────────────────────────────

pub fn rebuild_level_visuals(
    mut commands: Commands,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    assets: Option<Res<SpriteAssets>>,
    mut entity_map: ResMut<EntityMap>,
    level_roots: Query<Entity, With<LevelRoot>>,
    mut reload: EventReader<ReloadVisualsEvent>,
) {
    if reload.read().next().is_none() {
        return;
    }

    despawn_level(&mut commands, &level_roots, &mut entity_map);
    spawn_level_visuals(
        &mut commands,
        &bridge.world,
        &projection,
        assets.as_deref(),
        &mut entity_map,
    );
}

pub fn spawn_level_visuals(
    commands: &mut Commands,
    world: &robbo_core::World,
    projection: &ActiveProjection,
    assets: Option<&SpriteAssets>,
    entity_map: &mut EntityMap,
) {
    entity_map.0.clear();
    let tile = projection.tile_size();
    // Parent must carry Transform + Visibility or child sprites won't render (Bevy B0004).
    let root = commands
        .spawn((
            Name::new("LevelRoot"),
            LevelRoot,
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        // tile layer
        for row in 0..world.height {
            for col in 0..world.width {
                let cell = Cell::new(col as i16, row as i16);
                let idx = row as usize * world.width as usize + col as usize;
                let tile_kind = world.tiles[idx];
                let pos = projection.project(cell, 0.0);
                if let Some(sa) = assets {
                    if let Some(img) = sa.for_tile(tile_kind) {
                        parent.spawn((
                            Sprite {
                                image: img,
                                custom_size: Some(Vec2::splat(tile)),
                                ..default()
                            },
                            Transform::from_translation(pos),
                            TileSprite { tile_kind },
                        ));
                        continue;
                    }
                }
                parent.spawn((
                    Sprite {
                        custom_size: Some(Vec2::splat(tile)),
                        color: fallback_tile_color(tile_kind),
                        ..default()
                    },
                    Transform::from_translation(pos),
                    TileSprite { tile_kind },
                ));
            }
        }

        // entity layer
        for (cell, state) in &world.elements {
            let pos = projection.project(*cell, 1.0);
            let entity_id = if let Some(sa) = assets {
                parent
                    .spawn((
                        Sprite {
                            image: sa.for_element(&state.kind, state.direction),
                            custom_size: Some(Vec2::splat(tile)),
                            ..default()
                        },
                        Transform::from_translation(pos),
                        VisualMotion {
                            from: *cell,
                            to: *cell,
                            progress: 1.0,
                            duration: crate::bridge::ANIM_SECS,
                        },
                        VisualEntityId(state.id),
                    ))
                    .id()
            } else {
                parent
                    .spawn((
                        Sprite {
                            custom_size: Some(Vec2::splat(tile * 0.8)),
                            color: fallback_entity_color(&state.kind),
                            ..default()
                        },
                        Transform::from_translation(pos),
                        VisualMotion {
                            from: *cell,
                            to: *cell,
                            progress: 1.0,
                            duration: crate::bridge::ANIM_SECS,
                        },
                        VisualEntityId(state.id),
                    ))
                    .id()
            };
            entity_map.0.insert(state.id, entity_id);
        }
    });
}

fn despawn_level(
    commands: &mut Commands,
    level_roots: &Query<Entity, With<LevelRoot>>,
    entity_map: &mut EntityMap,
) {
    // Orphan runtime spawns (projectiles etc.) live outside LevelRoot — despawn explicitly.
    for entity in entity_map.0.values() {
        commands.entity(*entity).despawn();
    }
    entity_map.0.clear();
    for root in level_roots {
        commands.entity(root).despawn_recursive();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fallback colours (used before sprites load or on error)
// ─────────────────────────────────────────────────────────────────────────────

fn fallback_tile_color(tile: TileKind) -> Color {
    match tile {
        TileKind::Empty   => Color::srgb(0.1, 0.1, 0.18),
        TileKind::WallGrey | TileKind::WallSolid => Color::srgb(0.29, 0.42, 0.54),
        TileKind::WallGreen => Color::srgb(0.18, 0.49, 0.2),
        TileKind::WallBlack => Color::srgb(0.05, 0.05, 0.05),
        TileKind::WallRed   => Color::srgb(0.91, 0.3, 0.24),
        TileKind::Ground    => Color::srgb(0.49, 0.31, 0.2),
        TileKind::DoorClosed | TileKind::DoorOpen => Color::srgb(0.6, 0.4, 0.2),
        TileKind::Barrier   => Color::srgb(0.9, 0.5, 0.13),
        TileKind::Stop      => Color::srgb(0.8, 0.1, 0.1),
    }
}

fn fallback_entity_color(kind: &robbo_core::ElementKind) -> Color {
    use robbo_core::ElementKind;
    match kind {
        ElementKind::Robbo              => Color::srgb(0.29, 0.56, 0.89),
        ElementKind::Screw              => Color::srgb(0.95, 0.77, 0.06),
        ElementKind::Capsule            => Color::srgb(0.18, 0.8, 0.44),
        ElementKind::Box | ElementKind::PushBox => Color::srgb(0.58, 0.36, 0.2),
        ElementKind::Bomb               => Color::srgb(0.2, 0.2, 0.2),
        ElementKind::Bear { .. } | ElementKind::BlackBear { .. } => Color::srgb(0.6, 0.3, 0.1),
        ElementKind::Bird { .. }        => Color::srgb(0.9, 0.9, 0.9),
        ElementKind::Butterfly          => Color::srgb(0.9, 0.4, 0.9),
        ElementKind::Projectile { .. }  => Color::srgb(1.0, 0.9, 0.2),
        ElementKind::QuestionMark { .. } => Color::srgb(0.8, 0.2, 0.8),
        ElementKind::Gun { .. }         => Color::srgb(0.5, 0.5, 0.5),
        ElementKind::Magnet { .. }      => Color::srgb(0.2, 0.6, 0.9),
        ElementKind::Teleport { .. }    => Color::srgb(0.3, 0.9, 0.9),
        ElementKind::Laser { .. } | ElementKind::BlasterCell { .. } => {
            Color::srgb(1.0, 0.2, 0.2)
        }
        ElementKind::BigBoom { .. }   => Color::srgb(1.0, 0.5, 0.1),
        ElementKind::BarbedWire         => Color::srgb(0.4, 0.8, 0.2),
        ElementKind::Stop               => Color::srgb(0.8, 0.1, 0.1),
        _                               => Color::srgb(0.7, 0.7, 0.7),
    }
}
