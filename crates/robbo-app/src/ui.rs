use bevy::prelude::*;
use robbo_core::GameEvent;

use crate::app_state::AppState;
use crate::audio::{play_countdown_tick, GameAudio};
use crate::bridge::{CoreBridge, CoreGameEvent, GameSession, LoadLevelEvent, ReloadVisualsEvent};
use crate::input::SteeringState;
use crate::levels::{LevelRegistry, LevelSelection};
use crate::persistence::{GameSave, LevelProgress, SaveBackend, persist_save, record_active_level};

#[derive(Resource, Default)]
pub struct SpeedrunTimer {
    pub elapsed_ms: u64,
    pub running: bool,
}

/// 3-2-1 countdown before gameplay input (skipped on restart).
#[derive(Resource)]
pub struct LevelCountdown {
    pub active: bool,
    pub display: u8,
    pub timer: Timer,
    pub skip: bool,
}

impl Default for LevelCountdown {
    fn default() -> Self {
        Self {
            active: false,
            display: 0,
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            skip: false,
        }
    }
}

impl LevelCountdown {
    pub fn blocks_input(&self) -> bool {
        self.active && !self.skip
    }
}

#[derive(Component)]
pub struct CountdownText;

pub fn load_level_system(
    mut commands: Commands,
    mut events: MessageReader<LoadLevelEvent>,
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    mut session: ResMut<GameSession>,
    mut timer: ResMut<SpeedrunTimer>,
    mut countdown: ResMut<LevelCountdown>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    mut reload: MessageWriter<ReloadVisualsEvent>,
    audio: Res<GameAudio>,
    backend: Res<SaveBackend>,
    mut save: ResMut<GameSave>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    countdown_entities: Query<Entity, With<CountdownText>>,
) {
    for ev in events.read() {
        let Some(level) = selection.selected_level(&registry) else {
            bevy::log::warn!("No level selected");
            continue;
        };
        let Some(pack) = registry.pack_by_index(selection.pack_index) else {
            continue;
        };

        bridge.world = level.to_world();
        bridge.history.clear();
        bridge.events_queue.clear();
        *steering = SteeringState::default();

        session.pack_name = pack.name.clone();
        session.level_index = selection.level_index;
        session.level_label = format!("{} #{}", pack.name, level.index);

        record_active_level(&backend, &mut save, &pack.name, level.index);

        if ev.restart {
            session.tries += 1;
        } else {
            session.tries = 1;
        }

        timer.elapsed_ms = 0;
        timer.running = false;

        for entity in &countdown_entities {
            commands.entity(entity).despawn();
        }

        if ev.restart {
            countdown.skip = true;
            countdown.active = false;
            countdown.display = 0;
        } else {
            countdown.skip = false;
            countdown.active = true;
            countdown.display = 3;
            countdown.timer = Timer::from_seconds(1.0, TimerMode::Once);

            let scale = window
                .single()
                .map(|w| crate::viewport::ui_scale(w))
                .unwrap_or(1.0);
            commands.spawn((
                Text::new("3"),
                TextFont {
                    font_size: 96.0 * scale,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.2)),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(38.0),
                    left: Val::Percent(48.0),
                    ..default()
                },
                CountdownText,
            ));
            play_countdown_tick(&mut commands, &audio, &save);
        }

        bevy::log::info!(
            "Loaded level {} ({}x{}) from pack '{}'",
            level.index,
            level.width,
            level.height,
            pack.name
        );

        reload.write(ReloadVisualsEvent);
    }
}

pub fn tick_speedrun_timer(
    state: Res<State<AppState>>,
    mut timer: ResMut<SpeedrunTimer>,
    time: Res<Time>,
) {
    if *state.get() == AppState::Playing && timer.running {
        timer.elapsed_ms += (time.delta_secs() * 1000.0) as u64;
    }
}

pub fn tick_level_countdown(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<AppState>>,
    mut countdown: ResMut<LevelCountdown>,
    mut timer: ResMut<SpeedrunTimer>,
    mut countdown_text: Query<&mut Text, With<CountdownText>>,
    countdown_entities: Query<Entity, With<CountdownText>>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    if countdown.skip || !countdown.active {
        if !countdown.active && !timer.running {
            timer.running = true;
        }
        return;
    }

    countdown.timer.tick(time.delta());
    for mut text in &mut countdown_text {
        **text = countdown.display.to_string();
    }

    if !countdown.timer.is_finished() {
        return;
    }

    if countdown.display > 1 {
        countdown.display -= 1;
        countdown.timer = Timer::from_seconds(1.0, TimerMode::Once);
    } else {
        countdown.active = false;
        countdown.display = 0;
        timer.running = true;
        for entity in &countdown_entities {
            commands.entity(entity).despawn();
        }
    }
}

pub fn cleanup_countdown_overlay(mut commands: Commands, q: Query<Entity, With<CountdownText>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

pub fn on_core_events(
    mut reader: MessageReader<CoreGameEvent>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut timer: ResMut<SpeedrunTimer>,
    mut save: ResMut<GameSave>,
    backend: Res<SaveBackend>,
    session: Res<GameSession>,
) {
    for CoreGameEvent(event) in reader.read() {
        match event {
            GameEvent::LevelComplete => {
                if *state.get() != AppState::Playing {
                    continue;
                }
                timer.running = false;
                let key = (session.level_index + 1).to_string();
                let entry = save
                    .0
                    .packs
                    .entry(session.pack_name.clone())
                    .or_default()
                    .levels
                    .entry(key)
                    .or_insert_with(LevelProgress::default);
                entry.completed = true;
                if entry.best_time_ms == 0 || timer.elapsed_ms < entry.best_time_ms {
                    entry.best_time_ms = timer.elapsed_ms;
                }
                if entry.best_tries == 0 || session.tries < entry.best_tries {
                    entry.best_tries = session.tries;
                }
                persist_save(&backend, &save.0);
                bevy::log::info!("Level complete in {}ms", timer.elapsed_ms);
                next.set(AppState::LevelComplete);
            }
            GameEvent::LevelFailed => {
                if *state.get() != AppState::Playing {
                    continue;
                }
                timer.running = false;
                bevy::log::info!("Level failed");
                next.set(AppState::GameOver);
            }
            _ => {}
        }
    }
}
