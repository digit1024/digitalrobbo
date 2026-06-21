use bevy::prelude::*;
use robbo_core::Direction;

use crate::app_state::AppState;
use crate::audio::AudioGate;
use crate::bridge::{CoreBridge, LoadLevelEvent, ReloadVisualsEvent};
use crate::levels::{LevelRegistry, LevelSelection};
use crate::menu::MenuSelection;
use crate::persistence::{GameSave, persist_save};
use crate::ui::LevelCountdown;

/// Swallow menu keys for a few frames after entering gameplay (prevents Enter bleed).
#[derive(Resource, Default)]
pub struct InputCooldown {
    pub frames_remaining: u8,
}

/// Queued key events for integration tests (applied in PreUpdate after winit).
#[derive(Resource, Default)]
pub struct TestInputInject {
    pub press: Vec<KeyCode>,
    pub release: Vec<KeyCode>,
    pub clear: bool,
}

pub fn apply_test_input(mut keys: ResMut<ButtonInput<KeyCode>>, mut inject: ResMut<TestInputInject>) {
    if inject.clear {
        keys.clear();
        inject.clear = false;
    }
    for key in inject.release.drain(..) {
        keys.release(key);
    }
    for key in inject.press.drain(..) {
        keys.press(key);
    }
}

/// Latched player steering — consumed on sim tick boundaries only.
#[derive(Resource, Default, Clone)]
pub struct SteeringState {
    /// Direction key held after the initial press frame.
    pub hold: Option<Direction>,
    /// One-step move from tap-same-direction (consumed on next tick).
    pub tap_move: Option<Direction>,
    /// Shoot on next tick.
    pub shoot_pending: bool,
}

pub fn tick_input_cooldown(mut cooldown: ResMut<InputCooldown>) {
    if cooldown.frames_remaining > 0 {
        cooldown.frames_remaining -= 1;
    }
}

fn begin_playing(cooldown: &mut InputCooldown) {
    cooldown.frames_remaining = 4;
}

fn resolve_last_level(
    registry: &LevelRegistry,
    save: &GameSave,
    selection: &mut LevelSelection,
) {
    if !save.0.profile.last_pack.is_empty() {
        if let Some((pi, pack)) = registry
            .packs
            .iter()
            .enumerate()
            .find(|(_, p)| p.name == save.0.profile.last_pack)
        {
            selection.pack_index = pi;
            let level_idx = save.0.profile.last_level.saturating_sub(1) as usize;
            selection.level_index = level_idx.min(pack.levels.len().saturating_sub(1));
            return;
        }
    }
    selection.pack_index = 0;
    selection.level_index = 0;
}

struct DirKeys {
    up: KeyCode,
    down: KeyCode,
    left: KeyCode,
    right: KeyCode,
}

const DIR_KEYS: DirKeys = DirKeys {
    up: KeyCode::ArrowUp,
    down: KeyCode::ArrowDown,
    left: KeyCode::ArrowLeft,
    right: KeyCode::ArrowRight,
};

const WASD: DirKeys = DirKeys {
    up: KeyCode::KeyW,
    down: KeyCode::KeyS,
    left: KeyCode::KeyA,
    right: KeyCode::KeyD,
};

fn keys_for_dir(dir: Direction) -> [KeyCode; 2] {
    match dir {
        Direction::Up => [DIR_KEYS.up, WASD.up],
        Direction::Down => [DIR_KEYS.down, WASD.down],
        Direction::Left => [DIR_KEYS.left, WASD.left],
        Direction::Right => [DIR_KEYS.right, WASD.right],
    }
}

fn just_pressed_dir(keys: &ButtonInput<KeyCode>, dir: Direction) -> bool {
    let [a, b] = keys_for_dir(dir);
    keys.just_pressed(a) || keys.just_pressed(b)
}

fn pressed_dir(keys: &ButtonInput<KeyCode>, dir: Direction) -> bool {
    let [a, b] = keys_for_dir(dir);
    keys.pressed(a) || keys.pressed(b)
}

fn just_released_dir(keys: &ButtonInput<KeyCode>, dir: Direction) -> bool {
    let [a, b] = keys_for_dir(dir);
    keys.just_released(a) || keys.just_released(b)
}

fn turn_robbo(bridge: &mut CoreBridge, dir: Direction) {
    bridge.world.turn_robbo(dir);
    bridge.facing_direction = dir;
    bridge.last_direction = dir;
}

fn update_steering(keys: &ButtonInput<KeyCode>, bridge: &mut CoreBridge, steering: &mut SteeringState) {
    for dir in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        if just_released_dir(keys, dir) {
            if steering.hold == Some(dir) {
                steering.hold = None;
            }
            if steering.tap_move == Some(dir) {
                steering.tap_move = None;
            }
        }
    }

    for dir in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        if just_pressed_dir(keys, dir) {
            if dir == bridge.facing_direction {
                steering.tap_move = Some(dir);
            } else {
                turn_robbo(bridge, dir);
            }
        }
    }

    for dir in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        if pressed_dir(keys, dir) && !just_pressed_dir(keys, dir) {
            steering.hold = Some(dir);
        }
    }
}

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut selection: ResMut<LevelSelection>,
    menu_selection: Res<MenuSelection>,
    registry: Res<LevelRegistry>,
    mut load_events: EventWriter<LoadLevelEvent>,
    mut reload: EventWriter<ReloadVisualsEvent>,
    countdown: Res<LevelCountdown>,
    mut save: ResMut<GameSave>,
    mut gate: ResMut<AudioGate>,
    mut cooldown: ResMut<InputCooldown>,
) {
    if keys.get_just_pressed().next().is_some() {
        gate.unlocked = true;
    }

    let menu_blocked = cooldown.frames_remaining > 0;

    match state.get() {
        AppState::Intro | AppState::Boot => {}
        AppState::MainMenu => {
            if menu_blocked {
                return;
            }
            if keys.just_pressed(KeyCode::Enter) {
                if menu_selection.index == 0 {
                    resolve_last_level(&registry, &save, &mut selection);
                    if let Some(pack) = registry.pack_by_index(selection.pack_index) {
                        if let Some(level) = pack.levels.get(selection.level_index) {
                            save.0.profile.last_pack = pack.name.clone();
                            save.0.profile.last_level = level.index;
                            persist_save(&save.0);
                        }
                    }
                    load_events.send(LoadLevelEvent { restart: false });
                    begin_playing(&mut cooldown);
                    next.set(AppState::Playing);
                } else {
                    next.set(AppState::LevelSelect);
                }
            }
        }
        AppState::LevelSelect => {
            if menu_blocked {
                return;
            }
            let pack_count = registry.packs.len().max(1);
            if keys.just_pressed(KeyCode::ArrowUp) {
                selection.pack_index = (selection.pack_index + pack_count - 1) % pack_count;
                selection.level_index = 0;
            }
            if keys.just_pressed(KeyCode::ArrowDown) {
                selection.pack_index = (selection.pack_index + 1) % pack_count;
                selection.level_index = 0;
            }
            if let Some(pack) = registry.pack_by_index(selection.pack_index) {
                let level_count = pack.levels.len().max(1);
                if keys.just_pressed(KeyCode::ArrowLeft) {
                    selection.level_index = (selection.level_index + level_count - 1) % level_count;
                }
                if keys.just_pressed(KeyCode::ArrowRight) {
                    selection.level_index = (selection.level_index + 1) % level_count;
                }
            }
            if keys.just_pressed(KeyCode::Enter) {
                if let Some(pack) = registry.pack_by_index(selection.pack_index) {
                    if let Some(level) = pack.levels.get(selection.level_index) {
                        save.0.profile.last_pack = pack.name.clone();
                        save.0.profile.last_level = level.index;
                        persist_save(&save.0);
                    }
                }
                load_events.send(LoadLevelEvent { restart: false });
                begin_playing(&mut cooldown);
                next.set(AppState::Playing);
            }
            if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::MainMenu);
            }
        }
        AppState::Playing => {
            if countdown.blocks_input() {
                return;
            }

            update_steering(&keys, &mut bridge, &mut steering);

            if keys.just_pressed(KeyCode::Space) {
                steering.shoot_pending = true;
            } else if keys.just_pressed(KeyCode::KeyZ) {
                let current = bridge.world.clone();
                if let Some(prev) = bridge.history.undo(current) {
                    bridge.world = prev;
                    reload.send(ReloadVisualsEvent);
                }
            } else if keys.just_pressed(KeyCode::KeyX) {
                let current = bridge.world.clone();
                if let Some(next_world) = bridge.history.redo(current) {
                    bridge.world = next_world;
                    reload.send(ReloadVisualsEvent);
                }
            } else if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::Paused);
            }
        }
        AppState::Paused => {
            if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::Playing);
            } else if keys.just_pressed(KeyCode::KeyR) {
                load_events.send(LoadLevelEvent { restart: true });
                next.set(AppState::Playing);
            } else if keys.just_pressed(KeyCode::KeyQ) {
                next.set(AppState::LevelSelect);
            }
        }
        AppState::LevelComplete => {
            if keys.just_pressed(KeyCode::KeyN) || keys.just_pressed(KeyCode::Enter) {
                if let Some(pack) = registry.pack_by_index(selection.pack_index) {
                    if selection.level_index + 1 < pack.levels.len() {
                        selection.level_index += 1;
                        load_events.send(LoadLevelEvent { restart: false });
                        begin_playing(&mut cooldown);
                        next.set(AppState::Playing);
                        return;
                    }
                }
                next.set(AppState::LevelSelect);
            } else if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::LevelSelect);
            }
        }
        AppState::GameOver => {
            if keys.just_pressed(KeyCode::KeyR) {
                load_events.send(LoadLevelEvent { restart: true });
                next.set(AppState::Playing);
            } else if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Enter) {
                next.set(AppState::LevelSelect);
            }
        }
    }
}
