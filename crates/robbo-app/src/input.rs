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

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut bridge: ResMut<CoreBridge>,
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

            // Instant visual facing on the very first frame of a keypress —
            // happens before the tick fires so Robbo turns immediately.
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                bridge.facing_direction = Direction::Up;
            } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                bridge.facing_direction = Direction::Down;
            } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
                bridge.facing_direction = Direction::Left;
            } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
                bridge.facing_direction = Direction::Right;
            }

            let held_dir = if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
                Some(Direction::Up)
            } else if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
                Some(Direction::Down)
            } else if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
                Some(Direction::Left)
            } else if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
                Some(Direction::Right)
            } else {
                None
            };

            if let Some(dir) = held_dir {
                bridge.pending_input = Some(robbo_core::PlayerInput::Move(dir));
                return;
            }

            if bridge.animating {
                return;
            }

            if keys.just_pressed(KeyCode::Space) {
                bridge.pending_input =
                    Some(robbo_core::PlayerInput::Shoot(bridge.last_direction));
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
