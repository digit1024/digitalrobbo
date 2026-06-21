use bevy::prelude::*;
use robbo_core::{Cell, Direction};

use crate::projection::ActiveProjection;

use super::particle::{FxParticle, FxParticleState, FX_Z_LAYER};

const TELEPORT_BURST_COUNT: u32 = 16;
const TELEPORT_BURST_DURATION: f32 = 0.65;
const TELEPORT_BURST_SPEED: f32 = 42.0;

const EXPLOSION_COUNT: u32 = 28;
const EXPLOSION_DURATION: f32 = 0.68;
const EXPLOSION_DRAG: f32 = 0.95;
/// How far particles fly from the blast center (3×3 sim area ≈ 1.5 tiles + overshoot).
const EXPLOSION_SPREAD_TILES: f32 = 2.85;

const SHOT_TRAIL_COUNT: u32 = 5;
const SHOT_TRAIL_DURATION: f32 = 0.22;
const SHOT_TRAIL_SPEED: f32 = 28.0;

const AMBIENT_DURATION: f32 = 1.75;
const AMBIENT_SPEED: f32 = 16.0;

fn spawn_particle(
    commands: &mut Commands,
    level_root: Option<Entity>,
    position: Vec3,
    velocity: Vec2,
    color: Color,
    size: f32,
    duration: f32,
    drag: f32,
) {
    let mut entity = commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::splat(size)),
            ..default()
        },
        Transform::from_translation(position),
        FxParticle,
        FxParticleState::new(velocity, duration, 1.0, drag),
    ));
    if let Some(root) = level_root {
        entity.set_parent(root);
    }
}

/// Cyan / violet sparkles when Robbo teleports (entry + exit).
pub fn spawn_teleport_burst(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    tile: f32,
) {
    let colors = [
        Color::srgba(0.35, 0.95, 1.0, 0.95),
        Color::srgba(0.75, 0.45, 1.0, 0.9),
        Color::srgba(0.95, 0.95, 1.0, 0.85),
    ];
    let origin = projection.project(at, FX_Z_LAYER);
    let size = tile * 0.22;

    for i in 0..TELEPORT_BURST_COUNT {
        let angle = i as f32 * 2.399_963_2;
        let wobble = ((i * 17) % 7) as f32 * 0.08;
        let velocity = Vec2::new(angle.cos(), angle.sin()) * TELEPORT_BURST_SPEED * (0.55 + wobble)
            + Vec2::new(0.0, TELEPORT_BURST_SPEED * 0.35);
        let offset = Vec2::new(
            (i as f32 * 1.7).sin() * tile * 0.15,
            (i as f32 * 2.3).cos() * tile * 0.15,
        );
        spawn_particle(
            commands,
            level_root,
            origin + offset.extend(0.0),
            velocity,
            colors[i as usize % colors.len()],
            size,
            TELEPORT_BURST_DURATION,
            1.8,
        );
    }
}

/// Slow floating pixel orbiting a teleport mirror (ambient).
pub fn spawn_ambient_teleport_pixel(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    tile: f32,
    seed: u32,
) {
    let colors = [
        Color::srgba(0.4, 0.92, 1.0, 0.55),
        Color::srgba(0.7, 0.5, 1.0, 0.5),
        Color::srgba(0.85, 0.95, 1.0, 0.45),
    ];
    let origin = projection.project(at, FX_Z_LAYER);
    let phase = (seed as f32 * 1.31).sin();
    let offset = Vec2::new(phase * tile * 0.28, ((seed as f32 * 2.17).cos()) * tile * 0.28);
    let angle = seed as f32 * 1.7 + phase;
    let velocity = Vec2::new(angle.cos(), angle.sin()) * AMBIENT_SPEED
        + Vec2::new(0.0, AMBIENT_SPEED * 0.45);

    spawn_particle(
        commands,
        level_root,
        origin + offset.extend(0.0),
        velocity,
        colors[seed as usize % colors.len()],
        tile * 0.1,
        AMBIENT_DURATION,
        0.35,
    );
}

/// Golden sparkles orbiting a screw pickup.
pub fn spawn_ambient_screw_pixel(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    tile: f32,
    seed: u32,
) {
    let colors = [
        Color::srgba(1.0, 0.88, 0.15, 0.62),
        Color::srgba(1.0, 0.58, 0.08, 0.55),
        Color::srgba(0.98, 0.95, 0.45, 0.5),
    ];
    let origin = projection.project(at, FX_Z_LAYER);
    let phase = (seed as f32 * 1.47).sin();
    let orbit = tile * 0.38;
    let offset = Vec2::new(phase.cos() * orbit, phase.sin() * orbit * 0.85);
    let tangent = Vec2::new(-offset.y, offset.x).normalize_or_zero();
    let velocity = tangent * (AMBIENT_SPEED * 1.15) + Vec2::new(0.0, AMBIENT_SPEED * 0.35);

    spawn_particle(
        commands,
        level_root,
        origin + offset.extend(0.0),
        velocity,
        colors[seed as usize % colors.len()],
        tile * 0.11,
        AMBIENT_DURATION * 0.9,
        0.4,
    );
}

/// Initial speed so particles coast out to roughly `spread_tiles` before fading (exponential drag).
fn explosion_speed_for_spread(tile: f32, spread_tiles: f32, duration: f32, drag: f32) -> f32 {
    let distance = tile * spread_tiles;
    let fade = 1.0 - (-drag * duration).exp();
    if fade > 0.001 {
        distance * drag / fade
    } else {
        distance / duration
    }
}

/// Orange burst for bombs and question-mark reveals.
pub fn spawn_explosion_burst(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    at: Cell,
    tile: f32,
) {
    let colors = [
        Color::srgba(1.0, 0.85, 0.2, 0.95),
        Color::srgba(1.0, 0.5, 0.08, 0.9),
        Color::srgba(1.0, 0.25, 0.05, 0.85),
    ];
    let origin = projection.project(at, FX_Z_LAYER);
    let size = tile * 0.2;
    let base_speed = explosion_speed_for_spread(
        tile,
        EXPLOSION_SPREAD_TILES,
        EXPLOSION_DURATION,
        EXPLOSION_DRAG,
    );

    for i in 0..EXPLOSION_COUNT {
        let angle = i as f32 * 2.399_963_2 + 0.4;
        // Inner sparks + outer embers: ~1.5 tiles (3×3 edge) up to full spread.
        let speed = base_speed * (0.55 + (i % 7) as f32 * 0.1);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
        let offset = Vec2::new(
            (i as f32 * 0.9).sin() * tile * 0.18,
            (i as f32 * 1.1).cos() * tile * 0.18,
        );
        spawn_particle(
            commands,
            level_root,
            origin + offset.extend(0.0),
            velocity,
            colors[i as usize % colors.len()],
            size * (0.75 + (i % 3) as f32 * 0.15),
            EXPLOSION_DURATION,
            EXPLOSION_DRAG,
        );
    }
}

/// Yellow spark trail along the first cells of a laser bolt path.
pub fn spawn_shot_trail(
    commands: &mut Commands,
    projection: &ActiveProjection,
    level_root: Option<Entity>,
    from: Cell,
    direction: Direction,
    tile: f32,
) {
    let (dc, dr) = direction.delta();
    let colors = [
        Color::srgba(1.0, 0.95, 0.55, 0.9),
        Color::srgba(1.0, 0.75, 0.2, 0.85),
        Color::srgba(1.0, 0.55, 0.1, 0.75),
    ];

    for i in 0..SHOT_TRAIL_COUNT {
        let step = i as i16 + 1;
        let cell = from.offset(dc * step, dr * step);
        let origin = projection.project(cell, FX_Z_LAYER);
        let along = Vec2::new(dc as f32, -dr as f32).normalize_or_zero() * SHOT_TRAIL_SPEED;
        let spread = Vec2::new(-along.y, along.x) * ((i as f32 * 1.3).sin() * 8.0);
        let velocity = along + spread;

        spawn_particle(
            commands,
            level_root,
            origin,
            velocity,
            colors[i as usize % colors.len()],
            tile * 0.14,
            SHOT_TRAIL_DURATION,
            1.5,
        );
    }
}
