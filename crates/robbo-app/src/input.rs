use bevy::prelude::*;
use robbo_core::Direction;

use crate::app_state::AppState;
use crate::audio::AudioGate;
use crate::bridge::{CoreBridge, LoadLevelEvent, ReloadVisualsEvent};
use crate::game_menus::{dismiss_game_menu, GameMenuRoot};
use crate::levels::{LevelRegistry, LevelSelection};
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

/// Latched player steering — movement is decided on sim tick boundaries only.
#[derive(Resource, Default, Clone)]
pub struct SteeringState {
    /// Touch D-pad: direction held while finger is down (sampled on sim tick only).
    pub hold: Option<Direction>,
    /// Locked one-step move from a same-direction tap; survives key/finger release until the next sim tick (turn cancels).
    pub tap_move: Option<Direction>,
    /// Suppresses movement on the sim tick immediately after a turn.
    pub skip_move_on_tick: bool,
    /// Shoot on next tick.
    pub shoot_pending: bool,
}

pub fn tick_input_cooldown(mut cooldown: ResMut<InputCooldown>) {
    if cooldown.frames_remaining > 0 {
        cooldown.frames_remaining -= 1;
    }
}

pub(crate) fn begin_playing(cooldown: &mut InputCooldown) {
    cooldown.frames_remaining = 4;
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

fn turn_robbo(bridge: &mut CoreBridge, steering: &mut SteeringState, dir: Direction) {
    bridge.world.turn_robbo(dir);
    bridge.facing_direction = dir;
    bridge.last_direction = dir;
    steering.skip_move_on_tick = true;
    steering.tap_move = None;
}

/// Decide whether Robbo steps this sim tick. Keyboard uses live key state; touch uses `hold`.
pub(crate) fn player_move_on_tick(
    keys: &ButtonInput<KeyCode>,
    steering: &mut SteeringState,
    facing: Direction,
) -> Option<Direction> {
    if steering.skip_move_on_tick {
        steering.skip_move_on_tick = false;
        return None;
    }

    let wants = pressed_dir(keys, facing)
        || steering.hold == Some(facing)
        || steering.tap_move == Some(facing);
    if wants {
        steering.tap_move = None;
        Some(facing)
    } else {
        None
    }
}

/// Touch move pad: tap a new direction to turn, same direction to step.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_move_pad_press(
    bridge: &mut CoreBridge,
    steering: &mut SteeringState,
    dir: Direction,
) {
    if dir == bridge.facing_direction {
        steering.tap_move = Some(dir);
    } else {
        turn_robbo(bridge, steering, dir);
    }
    steering.hold = Some(dir);
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_move_pad_hold(steering: &mut SteeringState, dir: Direction) {
    steering.hold = Some(dir);
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_move_pad_release(steering: &mut SteeringState, dir: Direction) {
    if steering.hold == Some(dir) {
        steering.hold = None;
    }
}

/// Touch shoot pad: face direction and fire on press.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_shoot_pad_press(
    bridge: &mut CoreBridge,
    steering: &mut SteeringState,
    dir: Direction,
) {
    turn_robbo(bridge, steering, dir);
    steering.shoot_pending = true;
}

fn update_steering(keys: &ButtonInput<KeyCode>, bridge: &mut CoreBridge, steering: &mut SteeringState) {
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
                turn_robbo(bridge, steering, dir);
            }
        }
    }
}

pub fn keyboard_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut selection: ResMut<LevelSelection>,
    registry: Res<LevelRegistry>,
    mut load_events: MessageWriter<LoadLevelEvent>,
    mut reload: MessageWriter<ReloadVisualsEvent>,
    countdown: Res<LevelCountdown>,
    mut gate: ResMut<AudioGate>,
    mut cooldown: ResMut<InputCooldown>,
    menu_roots: Query<Entity, With<GameMenuRoot>>,
) {
    if keys.get_just_pressed().next().is_some() {
        gate.unlocked = true;
    }

    let menu_blocked = cooldown.frames_remaining > 0;

    match state.get() {
        AppState::Intro | AppState::Boot => {}
        AppState::MainMenu => {}
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
                load_events.write(LoadLevelEvent { restart: false });
                begin_playing(&mut cooldown);
                next.set(AppState::Playing);
                dismiss_game_menu(&mut commands, &menu_roots);
            }
            if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::MainMenu);
                dismiss_game_menu(&mut commands, &menu_roots);
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
                    reload.write(ReloadVisualsEvent);
                }
            } else if keys.just_pressed(KeyCode::KeyX) {
                let current = bridge.world.clone();
                if let Some(next_world) = bridge.history.redo(current) {
                    bridge.world = next_world;
                    reload.write(ReloadVisualsEvent);
                }
            } else if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::Paused);
            }
        }
        AppState::Paused => {
            if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::Playing);
                dismiss_game_menu(&mut commands, &menu_roots);
            } else if keys.just_pressed(KeyCode::KeyR) {
                load_events.write(LoadLevelEvent { restart: true });
                next.set(AppState::Playing);
                dismiss_game_menu(&mut commands, &menu_roots);
            } else if keys.just_pressed(KeyCode::KeyQ) {
                next.set(AppState::LevelSelect);
                dismiss_game_menu(&mut commands, &menu_roots);
            }
        }
        AppState::LevelComplete => {
            if keys.just_pressed(KeyCode::KeyN) || keys.just_pressed(KeyCode::Enter) {
                if let Some(pack) = registry.pack_by_index(selection.pack_index) {
                    if selection.level_index + 1 < pack.levels.len() {
                        selection.level_index += 1;
                        load_events.write(LoadLevelEvent { restart: false });
                        begin_playing(&mut cooldown);
                        next.set(AppState::Playing);
                        dismiss_game_menu(&mut commands, &menu_roots);
                        return;
                    }
                }
                next.set(AppState::LevelSelect);
                dismiss_game_menu(&mut commands, &menu_roots);
            } else if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::LevelSelect);
                dismiss_game_menu(&mut commands, &menu_roots);
            }
        }
        AppState::GameOver => {
            if keys.just_pressed(KeyCode::KeyR) {
                load_events.write(LoadLevelEvent { restart: true });
                begin_playing(&mut cooldown);
                next.set(AppState::Playing);
                dismiss_game_menu(&mut commands, &menu_roots);
            } else if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Enter) {
                next.set(AppState::LevelSelect);
                dismiss_game_menu(&mut commands, &menu_roots);
            }
        }
    }
}
