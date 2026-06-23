use bevy::prelude::*;
use robbo_core::{Cell, ElementKind};

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, ReloadVisualsEvent};
use crate::projection::ActiveProjection;
use crate::render::LevelRoot;

use super::presets::spawn_ambient_teleport_pixel;

/// Marks a teleport mirror tile; emits slow floating pixels while the level is active.
#[derive(Component)]
pub struct TeleportAuraAnchor {
    pub cell: Cell,
}

#[derive(Component)]
pub struct TeleportAuraState {
    pub spawn_timer: Timer,
    pub seed: u32,
}

const TELEPORT_AURA_SPAWN_SECS: f32 = 0.14;

/// Rebuild teleport aura anchors when the level visuals reload.
pub fn sync_fx_auras(
    mut commands: Commands,
    mut reload: MessageReader<ReloadVisualsEvent>,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    level_roots: Query<Entity, With<LevelRoot>>,
    teleports: Query<Entity, With<TeleportAuraAnchor>>,
) {
    if reload.read().next().is_none() {
        return;
    }

    for entity in &teleports {
        commands.entity(entity).despawn();
    }

    let Some(root) = level_roots.iter().next() else {
        return;
    };

    let mut seed = 0u32;
    for (cell, state) in &bridge.world.elements {
        if !matches!(state.kind, ElementKind::Teleport { .. }) {
            continue;
        }
        let pos = projection.project(*cell, 1.0);
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Transform::from_translation(pos),
                Visibility::default(),
                TeleportAuraAnchor { cell: *cell },
                TeleportAuraState {
                    spawn_timer: Timer::from_seconds(
                        TELEPORT_AURA_SPAWN_SECS * (0.7 + (seed % 5) as f32 * 0.08),
                        TimerMode::Repeating,
                    ),
                    seed,
                },
            ));
        });
        seed += 1;
    }
}

/// Emit ambient sparkles around each teleport mirror.
pub fn tick_teleport_auras(
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut commands: Commands,
    projection: Res<ActiveProjection>,
    level_roots: Query<Entity, With<LevelRoot>>,
    mut anchors: Query<(&TeleportAuraAnchor, &mut TeleportAuraState)>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    let level_root = level_roots.iter().next();
    let tile = projection.tile_size();

    for (anchor, mut aura) in &mut anchors {
        aura.spawn_timer.tick(time.delta());
        if !aura.spawn_timer.just_finished() {
            continue;
        }
        spawn_ambient_teleport_pixel(
            &mut commands,
            &projection,
            level_root,
            anchor.cell,
            tile,
            aura.seed.wrapping_add((aura.spawn_timer.elapsed_secs() * 1000.0) as u32),
        );
        aura.seed = aura.seed.wrapping_add(1);
    }
}
