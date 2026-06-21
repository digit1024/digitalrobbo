use bevy::prelude::*;
use robbo_core::GameEvent;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, CoreGameEvent, GameSession, LoadLevelEvent, ReloadVisualsEvent};
use crate::levels::{LevelRegistry, LevelSelection};
use crate::persistence::{GameSave, LevelProgress, persist_save};
#[derive(Resource, Default)]
pub struct SpeedrunTimer {
    pub elapsed_ms: u64,
    pub running: bool,
}

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct OverlayText;

pub fn load_level_system(
    mut events: EventReader<LoadLevelEvent>,
    mut bridge: ResMut<CoreBridge>,
    mut session: ResMut<GameSession>,
    mut timer: ResMut<SpeedrunTimer>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    mut reload: EventWriter<ReloadVisualsEvent>,
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
        bridge.animating = false;
        bridge.pending_input = None;
        bridge.queued_input = None;

        session.pack_name = pack.name.clone();
        session.level_index = selection.level_index;
        session.level_label = format!("{} #{}", pack.name, level.index);

        timer.elapsed_ms = 0;
        timer.running = true;

        bevy::log::info!(
            "Loaded level {} ({}x{}) from pack '{}'",
            level.index,
            level.width,
            level.height,
            pack.name
        );

        reload.send(ReloadVisualsEvent);
    }
}

pub fn update_hud(
    state: Res<State<AppState>>,
    bridge: Res<CoreBridge>,
    session: Res<GameSession>,
    timer: Res<SpeedrunTimer>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    for mut text in &mut hud {
        **text = format!(
            "{} | Screws: {}/{} | Ammo: {} | Keys: {} | Time: {:.1}s\nArrows move | Space shoot | Z undo | X redo | Esc pause",
            session.level_label,
            bridge.world.collected_screws,
            bridge.world.required_screws,
            bridge.world.ammo,
            bridge.world.keys,
            timer.elapsed_ms as f32 / 1000.0,
        );
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

pub fn spawn_playing_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudText,
    ));
}

fn overlay_label(
    state: &AppState,
    registry: &LevelRegistry,
    selection: &LevelSelection,
    save: &GameSave,
    timer: &SpeedrunTimer,
) -> String {
    match state {
        AppState::MainMenu => "DigitalRobbo\n\nPress ENTER to choose a level".to_string(),
        AppState::LevelSelect => {
            let pack = registry.pack_by_index(selection.pack_index);
            let pack_name = pack.map(|p| p.name.as_str()).unwrap_or("?");
            let level_count = pack.map(|p| p.levels.len()).unwrap_or(0);
            let completed = pack
                .and_then(|p| p.levels.get(selection.level_index))
                .map(|lvl| {
                    save.0
                        .packs
                        .get(pack_name)
                        .and_then(|pp| pp.levels.get(&(selection.level_index + 1).to_string()))
                        .map(|lp| lp.completed)
                        .unwrap_or(false)
                        && lvl.index > 0
                })
                .unwrap_or(false);
            let star = if completed { " *DONE*" } else { "" };
            format!(
                "LEVEL SELECT\n\nPack:  {}  ({}/{} packs)\nLevel: {} / {}{}\n\n[Up/Down] change pack   [Left/Right] change level\n[Enter] play            [Esc] back",
                pack_name,
                selection.pack_index + 1,
                registry.packs.len(),
                selection.level_index + 1,
                level_count,
                star,
            )
        }
        AppState::Paused => {
            "PAUSED\n\n[Esc] Resume   [R] Restart   [Q] Level Select".to_string()
        }
        AppState::LevelComplete => format!(
            "LEVEL COMPLETE!\n\nTime: {:.1}s\n\n[N / Enter] Next level   [Esc] Level Select",
            timer.elapsed_ms as f32 / 1000.0
        ),
        AppState::GameOver => "GAME OVER\n\n[R] Retry   [Esc] Level Select".to_string(),
        _ => String::new(),
    }
}

pub fn spawn_menu_overlay(
    mut commands: Commands,
    state: Res<State<AppState>>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    save: Res<GameSave>,
    timer: Res<SpeedrunTimer>,
) {
    let label = overlay_label(state.get(), &registry, &selection, &save, &timer);
    if !label.is_empty() {
        commands.spawn((
            Text::new(label),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.92, 1.0)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(60.0),
                left: Val::Px(40.0),
                ..default()
            },
            OverlayText,
        ));
    }
}

pub fn update_overlay_text(
    state: Res<State<AppState>>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    save: Res<GameSave>,
    timer: Res<SpeedrunTimer>,
    mut overlays: Query<&mut Text, With<OverlayText>>,
) {
    if overlays.is_empty() {
        return;
    }
    let label = overlay_label(state.get(), &registry, &selection, &save, &timer);
    for mut text in &mut overlays {
        if **text != label {
            **text = label.clone();
        }
    }
}

pub fn cleanup_overlay(mut commands: Commands, q: Query<Entity, With<OverlayText>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

pub fn cleanup_hud(mut commands: Commands, hud: Query<Entity, With<HudText>>) {
    for e in &hud {
        commands.entity(e).despawn();
    }
}

pub fn on_core_events(
    mut reader: EventReader<CoreGameEvent>,
    mut next: ResMut<NextState<AppState>>,
    mut timer: ResMut<SpeedrunTimer>,
    mut save: ResMut<GameSave>,
    session: Res<GameSession>,
) {
    for CoreGameEvent(event) in reader.read() {
        match event {
            GameEvent::LevelComplete => {
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
                persist_save(&save.0);
                bevy::log::info!("Level complete in {}ms", timer.elapsed_ms);
                next.set(AppState::LevelComplete);
            }
            GameEvent::LevelFailed => {
                timer.running = false;
                bevy::log::info!("Level failed");
                next.set(AppState::GameOver);
            }
            _ => {}
        }
    }
}
