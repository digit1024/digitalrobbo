use bevy::prelude::*;
use robbo_core::Direction;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, LoadLevelEvent, ReloadVisualsEvent};
use crate::levels::{LevelRegistry, LevelSelection};

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut bridge: ResMut<CoreBridge>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut selection: ResMut<LevelSelection>,
    registry: Res<LevelRegistry>,
    mut load_events: EventWriter<LoadLevelEvent>,
    mut reload: EventWriter<ReloadVisualsEvent>,
) {
    match state.get() {
        AppState::MainMenu => {
            if keys.just_pressed(KeyCode::Enter) {
                next.set(AppState::LevelSelect);
            }
        }
        AppState::LevelSelect => {
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
                load_events.send(LoadLevelEvent { restart: false });
                next.set(AppState::Playing);
            }
            if keys.just_pressed(KeyCode::Escape) {
                next.set(AppState::MainMenu);
            }
        }
        AppState::Playing => {
            // Movement responds to held keys so the hero keeps moving continuously.
            // buffer_input_while_animating will save it as queued input during a tween.
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
                // One-shot actions can't interrupt a move-in-progress.
                return;
            }

            // Nothing held — one-shot actions only valid when the previous
            // tween has fully completed.
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
        AppState::Boot => {}
    }
}
