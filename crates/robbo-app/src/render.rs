use bevy::prelude::*;
use robbo_core::{Cell, Direction, ElementKind, GameEvent, TileKind};

use crate::assets::{SpriteAssets, bear_direction_rotation};
use crate::bridge::{CoreBridge, EntityMap, GameSession, GameTickTimer, ReloadVisualsEvent, TileEntityMap, TICK_SECS};
use crate::effects::{
    attach_butterfly_visual, reset_magnet_beams, CapsuleVisual, CollectPopEffect, FxParticle,
    MagnetBeams, MagnetVisual, TeleportAuraAnchor, projectile_sprite_bundle, projectile_visual_for,
    ProjectileVisual, ScrewVisual, ButterflyVisual,
};
use crate::input::SteeringState;
use crate::interpolation::{VisualEntityId, VisualMotion, interpolated_pos, tick_phase, visual_step_ticks};
use crate::projection::ActiveProjection;
use crate::ui::{LevelCountdown, SpeedrunTimer};

// ─────────────────────────────────────────────────────────────────────────────
// Components
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct TileSprite {
    pub cell: Cell,
    pub tile_kind: TileKind,
}

#[derive(Component)]
pub struct TileVanishEffect {
    pub timer: Timer,
}

#[derive(Component)]
pub struct TeleportReveal {
    pub timer: Timer,
}

#[derive(Component)]
pub struct LevelRoot;

#[derive(Component)]
pub struct ExplosionEffect {
    pub timer: Timer,
}

/// Tick-synced rotation + move bob/scale for bears (single up-facing sprite).
#[derive(Component)]
pub struct BearVisual {
    pub rotation: f32,
    pub last_direction: Direction,
    pub turn_from: f32,
    pub turn_to: f32,
    pub turn_start_tick: f32,
    pub turning: bool,
}

impl BearVisual {
    pub fn new(direction: Direction, tick: f32) -> Self {
        let rotation = bear_direction_rotation(direction);
        Self {
            rotation,
            last_direction: direction,
            turn_from: rotation,
            turn_to: rotation,
            turn_start_tick: tick,
            turning: false,
        }
    }
}

const EXPLOSION_SECS: f32 = 0.35;
const TILE_VANISH_SECS: f32 = 0.28;
const BEAR_BOB_PX: f32 = 5.0;
const BEAR_SCALE_AMP: f32 = 0.05;
/// One sim tick to spin between facing directions.
const BEAR_TURN_TICKS: f32 = 1.0;

fn try_despawn(commands: &mut Commands, entity: Entity) {
    if commands.get_entity(entity).is_some() {
        commands.entity(entity).despawn_recursive();
    }
}

fn is_bear_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Bear { .. } | ElementKind::BlackBear { .. }
    )
}

fn magnet_direction(kind: &ElementKind) -> Option<Direction> {
    match kind {
        ElementKind::Magnet { direction } => Some(*direction),
        _ => None,
    }
}

fn spawn_element_sprite(
    sa: &SpriteAssets,
    kind: &ElementKind,
    direction: Direction,
    tile: f32,
) -> (Sprite, Option<ProjectileVisual>) {
    if projectile_visual_for(kind).is_some() {
        let (sprite, visual) = projectile_sprite_bundle(kind, direction, tile);
        (sprite, Some(visual))
    } else if matches!(kind, ElementKind::Robbo) {
        let (image, rect) = sa.player_sprite(direction, 0);
        (
            Sprite {
                image,
                rect: Some(rect),
                custom_size: Some(Vec2::splat(tile)),
                ..default()
            },
            None,
        )
    } else {
        (
            Sprite {
                image: sa.for_element(kind, direction),
                custom_size: Some(Vec2::splat(tile)),
                ..default()
            },
            None,
        )
    }
}

fn lerp_angle_shortest(from: f32, to: f32, t: f32) -> f32 {
    let mut delta = (to - from).rem_euclid(std::f32::consts::TAU);
    if delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    from + delta * t
}

/// Tick-synced bob (horizontal steps) or pulse-scale (vertical steps).
fn bear_move_fx(motion: &VisualMotion) -> (f32, f32) {
    if motion.from == motion.to || motion.progress <= 0.0 {
        return (0.0, 1.0);
    }
    let wave = (motion.progress * std::f32::consts::TAU).sin();
    let horizontal = motion.from.row == motion.to.row && motion.from.col != motion.to.col;
    let vertical = motion.from.col == motion.to.col && motion.from.row != motion.to.row;
    if horizontal {
        (wave * BEAR_BOB_PX, 1.0)
    } else if vertical {
        (0.0, 1.0 + wave * BEAR_SCALE_AMP)
    } else {
        (0.0, 1.0)
    }
}

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
    tile_map: Res<TileEntityMap>,
    mut commands: Commands,
    mut motion_q: Query<(&VisualEntityId, &mut VisualMotion)>,
    mut tile_q: Query<
        (&mut TileSprite, &mut Sprite, &mut Transform),
        Without<TileVanishEffect>,
    >,
    level_roots: Query<Entity, With<LevelRoot>>,
) {
    let has_events = !bridge.events_queue.is_empty();

    // Index moves before spawn so new projectiles/lasers start at the correct origin cell.
    let mut moves: std::collections::HashMap<u32, (Cell, Cell)> = std::collections::HashMap::new();
    let mut teleports: std::collections::HashMap<u32, (Cell, Cell)> = std::collections::HashMap::new();
    if has_events {
        for event in &bridge.events_queue {
            match event {
                GameEvent::Moved { entity_id, from, to }
                | GameEvent::Pushed { entity_id, from, to } => {
                    moves.insert(*entity_id, (*from, *to));
                }
                GameEvent::Teleported { entity_id, from, to } => {
                    teleports.insert(*entity_id, (*from, *to));
                }
                _ => {}
            }
        }
    }

    // Always reconcile entities — sim often removes/spawns without emitting events.
    let current_ids: std::collections::HashSet<u32> =
        bridge.world.elements.iter().map(|(_, s)| s.id).collect();

    entity_map.0.retain(|id, entity| {
        if current_ids.contains(id) {
            true
        } else {
            try_despawn(&mut commands, *entity);
            false
        }
    });

    let sim_tick = bridge.world.tick;
    let tile = projection.tile_size();
    let level_root = level_roots.iter().next();
    for (cell, state) in &bridge.world.elements {
        if entity_map.0.contains_key(&state.id) {
            continue;
        }
        let motion = if let Some((from, to)) = moves.get(&state.id) {
            VisualMotion::begin_step(
                *from,
                *to,
                sim_tick,
                visual_step_ticks(&state.kind, state),
            )
        } else {
            VisualMotion::settled(*cell, sim_tick)
        };
        let pos = projection.project(motion.from, 1.0);
        let spawn = if let Some(ref sa) = assets {
            let (sprite, projectile) =
                spawn_element_sprite(sa, &state.kind, state.direction, tile);
            let mut entity = commands.spawn((
                sprite,
                Transform::from_translation(pos),
                motion,
                VisualEntityId(state.id),
            ));
            if let Some(pv) = projectile {
                entity.insert(pv);
            }
            if is_bear_kind(&state.kind) {
                entity.insert(BearVisual::new(state.direction, sim_tick as f32));
            }
            if let Some(dir) = magnet_direction(&state.kind) {
                entity.insert(MagnetVisual::new(dir));
            }
            if matches!(state.kind, ElementKind::Screw) {
                entity.insert(ScrewVisual::from_cell(cell.col, cell.row));
            }
            if matches!(state.kind, ElementKind::Capsule) {
                entity.insert(CapsuleVisual::from_cell(cell.col, cell.row));
            }
            entity
        } else if projectile_visual_for(&state.kind).is_some() {
            let (sprite, pv) = projectile_sprite_bundle(&state.kind, state.direction, tile);
            let mut entity = commands.spawn((
                sprite,
                Transform::from_translation(pos),
                motion,
                VisualEntityId(state.id),
                pv,
            ));
            if is_bear_kind(&state.kind) {
                entity.insert(BearVisual::new(state.direction, sim_tick as f32));
            }
            if let Some(dir) = magnet_direction(&state.kind) {
                entity.insert(MagnetVisual::new(dir));
            }
            if matches!(state.kind, ElementKind::Screw) {
                entity.insert(ScrewVisual::from_cell(cell.col, cell.row));
            }
            if matches!(state.kind, ElementKind::Capsule) {
                entity.insert(CapsuleVisual::from_cell(cell.col, cell.row));
            }
            entity
        } else {
            let mut entity = commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::splat(tile * 0.8)),
                    color: fallback_entity_color(&state.kind),
                    ..default()
                },
                Transform::from_translation(pos),
                motion,
                VisualEntityId(state.id),
            ));
            if is_bear_kind(&state.kind) {
                entity.insert(BearVisual::new(state.direction, sim_tick as f32));
            }
            if let Some(dir) = magnet_direction(&state.kind) {
                entity.insert(MagnetVisual::new(dir));
            }
            if matches!(state.kind, ElementKind::Screw) {
                entity.insert(ScrewVisual::from_cell(cell.col, cell.row));
            }
            if matches!(state.kind, ElementKind::Capsule) {
                entity.insert(CapsuleVisual::from_cell(cell.col, cell.row));
            }
            entity
        };
        let id = spawn.id();
        if matches!(state.kind, ElementKind::Butterfly) {
            attach_butterfly_visual(&mut commands, id, cell.col, cell.row);
        }
        if let Some(root) = level_root {
            commands.entity(id).set_parent(root);
        }
        entity_map.0.insert(state.id, id);
    }

    sync_tile_visuals(
        &bridge.world,
        &tile_map,
        assets.as_deref(),
        tile,
        &mut tile_q,
    );

    if has_events {
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
                GameEvent::DoorOpened { at } | GameEvent::TileCleared { at, .. } => {
                    start_tile_vanish(&mut commands, &tile_map, *at);
                }
                GameEvent::Moved { entity_id, from, to }
                | GameEvent::Pushed { entity_id, from, to } => {
                    if let Some(&entity) = entity_map.0.get(entity_id) {
                        let step = bridge
                            .world
                            .elements
                            .iter()
                            .find(|(_, s)| s.id == *entity_id)
                            .map(|(_, s)| visual_step_ticks(&s.kind, s))
                            .unwrap_or(1);
                        if let Ok((_, mut motion)) = motion_q.get_mut(entity) {
                            motion.retarget(*from, *to, sim_tick, step);
                        }
                    }
                }
                _ => {}
            }
        }

        for (entity_id, (_from, to)) in teleports {
            if let Some(&entity) = entity_map.0.get(&entity_id) {
                if let Ok((_, mut motion)) = motion_q.get_mut(entity) {
                    *motion = VisualMotion::settled(to, sim_tick);
                }
                let pos = projection.project(to, 1.0);
                commands.entity(entity).insert((
                    Transform::from_translation(pos),
                    Visibility::Hidden,
                    TeleportReveal {
                        timer: Timer::from_seconds(TICK_SECS, TimerMode::Once),
                    },
                ));
            }
        }
    }

    // Anchor idle entities to their sim cell (bear turns, completed tweens, etc.).
    for (cell, state) in &bridge.world.elements {
        if moves.contains_key(&state.id) {
            continue;
        }
        if let Some(entity) = entity_map.0.get(&state.id) {
            if let Ok((_, mut motion)) = motion_q.get_mut(*entity) {
                if motion.from == motion.to || motion.progress >= 1.0 {
                    motion.from = *cell;
                    motion.to = *cell;
                    motion.progress = 1.0;
                    motion.start_tick = sim_tick;
                }
            }
        }
    }

    bridge.events_queue.clear();
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
    let frame = bridge.walk_frame;
    sprite.image = sa.player_sheet.clone();
    sprite.rect = Some(SpriteAssets::player_frame_rect(bridge.facing_direction, frame));
}

/// Keep directional sprites in sync with sim state (bears turning, guns rotating, etc.).
pub fn update_entity_sprites(
    bridge: Res<CoreBridge>,
    entity_map: Res<EntityMap>,
    assets: Option<Res<SpriteAssets>>,
    mut query: Query<(&VisualEntityId, &mut Sprite)>,
) {
    let Some(ref sa) = assets else { return; };
    let robbo_id = bridge.world.robbo_id;
    for (_, state) in &bridge.world.elements {
        if state.id == robbo_id {
            continue;
        }
        let Some(entity) = entity_map.0.get(&state.id) else {
            continue;
        };
        let Ok((_, mut sprite)) = query.get_mut(*entity) else {
            continue;
        };
        if is_bear_kind(&state.kind) {
            continue;
        }
        if magnet_direction(&state.kind).is_some() {
            continue;
        }
        if matches!(state.kind, ElementKind::Capsule) {
            continue;
        }
        if matches!(state.kind, ElementKind::Butterfly) {
            continue;
        }
        if projectile_visual_for(&state.kind).is_some() {
            continue;
        }
        sprite.image = sa.for_element(&state.kind, state.direction);
    }
}

/// Bears use a single up-facing sprite; rotation + bob/scale are applied here.
pub fn update_bear_visuals(
    bridge: Res<CoreBridge>,
    tick_timer: Res<GameTickTimer>,
    projection: Res<ActiveProjection>,
    mut query: Query<(
        &VisualEntityId,
        &VisualMotion,
        &mut BearVisual,
        &mut Transform,
    )>,
) {
    let tick_now = bridge.world.tick as f32 + tick_phase(&tick_timer);
    for (entity_id, motion, mut bear, mut transform) in &mut query {
        let Some((_, state)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == entity_id.0)
        else {
            continue;
        };

        if state.direction != bear.last_direction {
            bear.turn_from = bear.rotation;
            bear.turn_to = bear_direction_rotation(state.direction);
            bear.turn_start_tick = tick_now;
            bear.turning = true;
            bear.last_direction = state.direction;
        }

        if bear.turning {
            let t = ((tick_now - bear.turn_start_tick) / BEAR_TURN_TICKS).clamp(0.0, 1.0);
            bear.rotation = lerp_angle_shortest(bear.turn_from, bear.turn_to, t);
            if t >= 1.0 {
                bear.turning = false;
            }
        }

        let pos = interpolated_pos(motion, &projection, 1.0);
        let (bob_y, scale) = bear_move_fx(motion);
        transform.translation = pos + Vec3::new(0.0, bob_y, 0.0);
        transform.rotation = Quat::from_rotation_z(bear.rotation);
        transform.scale = Vec3::splat(scale);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Transform update from interpolated position
// ─────────────────────────────────────────────────────────────────────────────

pub fn update_entity_transforms(
    projection: Res<ActiveProjection>,
    mut query: Query<
        (&VisualMotion, &mut Transform),
        (With<VisualEntityId>, Without<BearVisual>, Without<ButterflyVisual>, Without<MagnetVisual>, Without<CapsuleVisual>, Without<ProjectileVisual>, Without<ScrewVisual>),
    >,
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
    mut tile_map: ResMut<TileEntityMap>,
    mut magnet_beams: ResMut<MagnetBeams>,
    level_roots: Query<Entity, With<LevelRoot>>,
    mut reload: EventReader<ReloadVisualsEvent>,
) {
    if reload.read().next().is_none() {
        return;
    }

    reset_magnet_beams(&mut magnet_beams);
    despawn_level(&mut commands, &level_roots, &mut entity_map, &mut tile_map);
    spawn_level_visuals(
        &mut commands,
        &bridge.world,
        &projection,
        assets.as_deref(),
        &mut entity_map,
        &mut tile_map,
    );
}

pub fn spawn_level_visuals(
    commands: &mut Commands,
    world: &robbo_core::World,
    projection: &ActiveProjection,
    assets: Option<&SpriteAssets>,
    entity_map: &mut EntityMap,
    tile_map: &mut TileEntityMap,
) {
    entity_map.0.clear();
    tile_map.0.clear();
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

    let mut butterfly_attach = Vec::new();
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
                        let id = parent
                            .spawn((
                                Sprite {
                                    image: img,
                                    custom_size: Some(Vec2::splat(tile)),
                                    ..default()
                                },
                                Transform::from_translation(pos),
                                TileSprite {
                                    cell,
                                    tile_kind,
                                },
                            ))
                            .id();
                        tile_map.0.insert(cell, id);
                        continue;
                    }
                }
                let id = parent
                    .spawn((
                        Sprite {
                            custom_size: Some(Vec2::splat(tile)),
                            color: fallback_tile_color(tile_kind),
                            ..default()
                        },
                        Transform::from_translation(pos),
                        TileSprite {
                            cell,
                            tile_kind,
                        },
                    ))
                    .id();
                tile_map.0.insert(cell, id);
            }
        }

        // entity layer
        for (cell, state) in &world.elements {
            let pos = projection.project(*cell, 1.0);
            let entity_id = if let Some(sa) = assets {
                let (sprite, projectile) =
                    spawn_element_sprite(sa, &state.kind, state.direction, tile);
                let mut spawn = parent.spawn((
                    sprite,
                    Transform::from_translation(pos),
                    VisualMotion::settled(*cell, world.tick),
                    VisualEntityId(state.id),
                ));
                if let Some(pv) = projectile {
                    spawn.insert(pv);
                }
                if is_bear_kind(&state.kind) {
                    spawn.insert(BearVisual::new(state.direction, world.tick as f32));
                }
                if let Some(dir) = magnet_direction(&state.kind) {
                    spawn.insert(MagnetVisual::new(dir));
                }
                if matches!(state.kind, ElementKind::Screw) {
                    spawn.insert(ScrewVisual::from_cell(cell.col, cell.row));
                }
                if matches!(state.kind, ElementKind::Capsule) {
                    spawn.insert(CapsuleVisual::from_cell(cell.col, cell.row));
                }
                let id = spawn.id();
                if matches!(state.kind, ElementKind::Butterfly) {
                    butterfly_attach.push((id, cell.col, cell.row));
                }
                id
            } else if projectile_visual_for(&state.kind).is_some() {
                let (sprite, pv) =
                    projectile_sprite_bundle(&state.kind, state.direction, tile);
                parent
                    .spawn((
                        sprite,
                        Transform::from_translation(pos),
                        VisualMotion::settled(*cell, world.tick),
                        VisualEntityId(state.id),
                        pv,
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
                        VisualMotion::settled(*cell, world.tick),
                        VisualEntityId(state.id),
                    ))
                    .id()
            };
            entity_map.0.insert(state.id, entity_id);
        }
    });
    for (id, col, row) in butterfly_attach {
        attach_butterfly_visual(commands, id, col, row);
    }
}

fn despawn_level(
    commands: &mut Commands,
    level_roots: &Query<Entity, With<LevelRoot>>,
    entity_map: &mut EntityMap,
    tile_map: &mut TileEntityMap,
) {
    for root in level_roots.iter() {
        commands.entity(root).despawn_recursive();
    }
    // Orphan runtime spawns (projectiles etc.) may live outside LevelRoot.
    for entity in entity_map.0.values() {
        try_despawn(commands, *entity);
    }
    entity_map.0.clear();
    tile_map.0.clear();
}

/// Remove the active level scene and transient FX (menu / level-select transitions).
pub fn teardown_level_scene(
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut tile_map: ResMut<TileEntityMap>,
    mut magnet_beams: ResMut<MagnetBeams>,
    level_roots: Query<Entity, With<LevelRoot>>,
    fx_particles: Query<Entity, With<FxParticle>>,
    teleport_anchors: Query<Entity, With<TeleportAuraAnchor>>,
    explosions: Query<Entity, With<ExplosionEffect>>,
    collect_pops: Query<Entity, With<CollectPopEffect>>,
) {
    reset_magnet_beams(&mut magnet_beams);
    for entity in fx_particles
        .iter()
        .chain(teleport_anchors.iter())
        .chain(explosions.iter())
        .chain(collect_pops.iter())
    {
        try_despawn(&mut commands, entity);
    }
    despawn_level(&mut commands, &level_roots, &mut entity_map, &mut tile_map);
}

/// Reset simulation resources when leaving gameplay for a menu scene.
pub fn reset_sim_on_menu(
    mut bridge: ResMut<CoreBridge>,
    mut session: ResMut<GameSession>,
    mut timer: ResMut<SpeedrunTimer>,
    mut countdown: ResMut<LevelCountdown>,
    mut steering: ResMut<SteeringState>,
) {
    *bridge = CoreBridge::default();
    *session = GameSession::default();
    timer.elapsed_ms = 0;
    timer.running = false;
    *countdown = LevelCountdown::default();
    *steering = SteeringState::default();
}

/// Keep tile sprites aligned with sim tiles (barrier slides, stop clears, ?→ground, etc.).
fn sync_tile_visuals(
    world: &robbo_core::World,
    tile_map: &TileEntityMap,
    assets: Option<&SpriteAssets>,
    tile_size: f32,
    tile_q: &mut Query<
        (&mut TileSprite, &mut Sprite, &mut Transform),
        Without<TileVanishEffect>,
    >,
) {
    for (&cell, &entity) in &tile_map.0 {
        let Some(sim_kind) = world.tile_at(cell) else {
            continue;
        };
        let Ok((mut tile_sprite, mut sprite, mut transform)) = tile_q.get_mut(entity) else {
            continue;
        };
        if tile_sprite.tile_kind == sim_kind {
            continue;
        }
        tile_sprite.tile_kind = sim_kind;
        transform.scale = Vec3::ONE;
        if let Some(sa) = assets {
            if let Some(img) = sa.for_tile(sim_kind) {
                sprite.image = img;
                sprite.color = Color::WHITE;
            } else {
                sprite.color = fallback_tile_color(sim_kind);
            }
        } else {
            sprite.color = fallback_tile_color(sim_kind);
        }
        sprite.custom_size = Some(Vec2::splat(tile_size));
    }
}

fn start_tile_vanish(commands: &mut Commands, tile_map: &TileEntityMap, cell: Cell) {
    let Some(&entity) = tile_map.0.get(&cell) else {
        return;
    };
    commands.entity(entity).insert(TileVanishEffect {
        timer: Timer::from_seconds(TILE_VANISH_SECS, TimerMode::Once),
    });
}

pub fn update_tile_vanish_effects(
    time: Res<Time>,
    mut commands: Commands,
    assets: Option<Res<SpriteAssets>>,
    projection: Res<ActiveProjection>,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut TileVanishEffect, &mut TileSprite)>,
) {
    let tile = projection.tile_size();
    for (entity, mut transform, mut sprite, mut effect, mut tile_sprite) in &mut query {
        effect.timer.tick(time.delta());
        let t = (effect.timer.elapsed_secs() / TILE_VANISH_SECS).clamp(0.0, 1.0);
        let scale = 1.0 - t * 0.55;
        transform.scale = Vec3::splat(scale);
        let alpha = 1.0 - t;
        sprite.color = sprite.color.with_alpha(alpha);

        if effect.timer.finished() {
            tile_sprite.tile_kind = TileKind::Empty;
            transform.scale = Vec3::ONE;
            if let Some(ref sa) = assets {
                sprite.image = sa.tile_empty.clone();
                sprite.color = Color::WHITE;
            } else {
                sprite.color = fallback_tile_color(TileKind::Empty);
            }
            sprite.custom_size = Some(Vec2::splat(tile));
            commands.entity(entity).remove::<TileVanishEffect>();
        }
    }
}

pub fn update_teleport_reveal(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut TeleportReveal, &mut Visibility)>,
) {
    for (entity, mut reveal, mut visibility) in &mut query {
        reveal.timer.tick(time.delta());
        if reveal.timer.finished() {
            *visibility = Visibility::Inherited;
            commands.entity(entity).remove::<TeleportReveal>();
        }
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
