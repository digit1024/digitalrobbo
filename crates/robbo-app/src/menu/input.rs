//! Main-menu keyboard, pointer input, and highlight updates.

use bevy::audio::AudioSink;
use bevy::prelude::*;

use super::layout::MainMenuLayout;
use super::spawn::rebuild_ui;
use super::state::{
    LevelSelectLevelLabel, LevelSelectPackLabel, LevelSelectStatusLabel, MainMenuAction,
    MainMenuBackground, MainMenuItem, MainMenuPlanet, MainMenuScreen, MainMenuState,
    MainMenuUiDirty, MainMenuUiRoot, MusicVolumeLabel,
};
use crate::app_state::AppState;
use crate::audio::{
    adjust_music_volume, apply_live_music_volume, is_muted, music_volume_percent, toggle_mute,
    BgmState, MUSIC_VOLUME_STEP,
};
use crate::bridge::LoadLevelEvent;
use crate::input::{begin_playing, InputCooldown};
use crate::levels::{resolve_last_level, LevelRegistry, LevelSelection};
use crate::persistence::{persist_save, GameSave, SaveBackend};
use crate::viewport;

const COLOR_ITEM: Color = Color::srgb(0.95, 0.95, 1.0);
const COLOR_SELECTED: Color = Color::srgb(1.0, 1.0, 1.0);

pub fn animate_planet(
    time: Res<Time>,
    state: Res<State<AppState>>,
    mut layout_res: Option<ResMut<MainMenuLayout>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut backgrounds: Query<&mut Transform, (With<MainMenuBackground>, Without<MainMenuPlanet>)>,
    mut planets: Query<&mut Transform, With<MainMenuPlanet>>,
) {
    if *state.get() != AppState::MainMenu {
        return;
    }
    let Ok(window) = window.single() else {
        return;
    };

    let layout = MainMenuLayout::from_window(window);
    if let Some(mut res) = layout_res {
        *res = layout;
    }

    for mut tf in &mut backgrounds {
        *tf = viewport::cover_transform(window, viewport::DESIGN_WIDTH, viewport::DESIGN_HEIGHT);
    }
    for mut tf in &mut planets {
        tf.scale = Vec3::splat(layout.cover);
        tf.translation.y = tf
            .translation
            .y
            .lerp(layout.planet_target_y, time.delta_secs() * 1.2);
    }
}

pub fn refresh_ui_if_dirty(
    mut dirty: ResMut<MainMenuUiDirty>,
    state: Res<State<AppState>>,
    mut commands: Commands,
    font: Res<crate::persistence::GameFont>,
    save: Res<GameSave>,
    menu_state: Res<MainMenuState>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    roots: Query<Entity, With<MainMenuUiRoot>>,
) {
    if *state.get() != AppState::MainMenu || !dirty.0 {
        return;
    }
    dirty.0 = false;
    let Ok(window) = window.single() else {
        return;
    };
    rebuild_ui(
        &mut commands,
        &font,
        &save,
        window,
        menu_state.screen,
        &roots,
    );
}

pub fn update_highlight(
    state: Res<State<AppState>>,
    menu_state: Res<MainMenuState>,
    items: Query<(&MainMenuItem, &Children, Option<&Button>)>,
    mut item_texts: Query<
        &mut Text,
        (
            Without<MusicVolumeLabel>,
            Without<LevelSelectPackLabel>,
            Without<LevelSelectLevelLabel>,
            Without<LevelSelectStatusLabel>,
        ),
    >,
    mut item_colors: Query<
        &mut TextColor,
        (
            Without<MusicVolumeLabel>,
            Without<LevelSelectPackLabel>,
            Without<LevelSelectLevelLabel>,
            Without<LevelSelectStatusLabel>,
        ),
    >,
) {
    if *state.get() != AppState::MainMenu {
        return;
    }

    for (item, children, is_button) in &items {
        let selected = item.index == menu_state.selection;
        for (i, child) in children.iter().enumerate() {
            if is_button.is_some() {
                if let Ok(mut text) = item_texts.get_mut(child) {
                    let body = text.to_string().trim_start_matches(['>', ' ']).to_string();
                    **text = if selected {
                        format!("> {body}")
                    } else {
                        format!("  {body}")
                    };
                }
                if let Ok(mut color) = item_colors.get_mut(child) {
                    *color = if selected {
                        TextColor(COLOR_SELECTED)
                    } else {
                        TextColor(COLOR_ITEM)
                    };
                }
            } else if i == 0 {
                if let Ok(mut color) = item_colors.get_mut(child) {
                    *color = if selected {
                        TextColor(COLOR_SELECTED)
                    } else {
                        TextColor(COLOR_ITEM)
                    };
                }
            }
        }
    }
}

pub fn update_settings_labels(
    state: Res<State<AppState>>,
    menu_state: Res<MainMenuState>,
    save: Res<GameSave>,
    mut volume: Query<(&mut Text, &mut TextColor), With<MusicVolumeLabel>>,
) {
    if *state.get() != AppState::MainMenu || menu_state.screen != MainMenuScreen::Settings {
        return;
    }
    let pct = music_volume_percent(&save);
    for (mut text, mut color) in &mut volume {
        **text = format!("{pct:>3}%");
        *color = if menu_state.selection == 0 {
            TextColor(COLOR_SELECTED)
        } else {
            TextColor(viewport::menu_title_text_color())
        };
    }
}

pub fn update_level_select_labels_system(
    state: Res<State<AppState>>,
    menu_state: Res<MainMenuState>,
    registry: Res<LevelRegistry>,
    level_selection: Res<LevelSelection>,
    save: Res<GameSave>,
    mut pack_labels: Query<&mut Text, (With<LevelSelectPackLabel>, Without<LevelSelectLevelLabel>)>,
    mut level_labels: Query<&mut Text, (With<LevelSelectLevelLabel>, Without<LevelSelectPackLabel>)>,
) {
    if *state.get() != AppState::MainMenu || menu_state.screen != MainMenuScreen::LevelSelect {
        return;
    }
    update_level_select_labels(
        &registry,
        &level_selection,
        &save,
        &mut pack_labels,
        &mut level_labels,
    );
}

fn update_level_select_labels(
    registry: &LevelRegistry,
    selection: &LevelSelection,
    save: &GameSave,
    pack_labels: &mut Query<&mut Text, (With<LevelSelectPackLabel>, Without<LevelSelectLevelLabel>)>,
    level_labels: &mut Query<&mut Text, (With<LevelSelectLevelLabel>, Without<LevelSelectPackLabel>)>,
) {
    let pack = registry.pack_by_index(selection.pack_index);
    let pack_name = pack.map(|p| p.name.as_str()).unwrap_or("?");
    let pack_count = registry.packs.len();
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
    let star = if completed { "  *completed*" } else { "" };

    for mut text in pack_labels.iter_mut() {
        **text = format!(
            "Pack: {pack_name}  ({}/{pack_count})",
            selection.pack_index + 1
        );
    }
    for mut text in level_labels.iter_mut() {
        **text = format!(
            "Level: {} / {}{}",
            selection.level_index + 1,
            level_count,
            star
        );
    }
}

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut menu_state: ResMut<MainMenuState>,
    mut dirty: ResMut<MainMenuUiDirty>,
    mut save: ResMut<GameSave>,
    backend: Res<SaveBackend>,
    bgm: Res<BgmState>,
    mut sinks: Query<&mut AudioSink>,
    registry: Res<LevelRegistry>,
    mut level_selection: ResMut<LevelSelection>,
    mut load_events: MessageWriter<LoadLevelEvent>,
    mut cooldown: ResMut<InputCooldown>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::MainMenu || cooldown.frames_remaining > 0 {
        return;
    }

    let count = menu_state.screen.item_count();
    if menu_state.screen != MainMenuScreen::LevelSelect {
        if keys.just_pressed(KeyCode::ArrowUp) {
            menu_state.selection = menu_state.selection.wrapping_sub(1) % count;
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            menu_state.selection = (menu_state.selection + 1) % count;
        }
    }

    match menu_state.screen {
        MainMenuScreen::Settings => settings_keys(
            &keys,
            &backend,
            &mut menu_state,
            &mut dirty,
            &mut save,
            &bgm,
            &mut sinks,
            &registry,
            &mut level_selection,
            &mut load_events,
            &mut cooldown,
            &mut next,
        ),
        MainMenuScreen::LevelSelect => level_select_keys(
            &keys,
            &backend,
            &mut menu_state,
            &mut dirty,
            &mut save,
            &bgm,
            &mut sinks,
            &registry,
            &mut level_selection,
            &mut load_events,
            &mut cooldown,
            &mut next,
        ),
        MainMenuScreen::Root => {
            if keys.just_pressed(KeyCode::Enter) {
                let action = match menu_state.selection {
                    0 => MainMenuAction::Start,
                    1 => MainMenuAction::OpenLevelSelect,
                    _ => MainMenuAction::OpenSettings,
                };
                apply_action(
                    action,
                    &backend,
                    &mut menu_state,
                    &mut dirty,
                    &mut save,
                    &bgm,
                    &mut sinks,
                    &registry,
                    &mut level_selection,
                    &mut load_events,
                    &mut cooldown,
                    &mut next,
                );
            }
        }
    }
}

fn settings_keys(
    keys: &ButtonInput<KeyCode>,
    backend: &SaveBackend,
    menu_state: &mut MainMenuState,
    dirty: &mut MainMenuUiDirty,
    save: &mut GameSave,
    bgm: &BgmState,
    sinks: &mut Query<&mut AudioSink>,
    registry: &LevelRegistry,
    level_selection: &mut LevelSelection,
    load_events: &mut MessageWriter<LoadLevelEvent>,
    cooldown: &mut InputCooldown,
    next: &mut NextState<AppState>,
) {
    if keys.just_pressed(KeyCode::ArrowLeft) {
        apply_action(
            MainMenuAction::MusicLess,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        apply_action(
            MainMenuAction::MusicMore,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
    if keys.just_pressed(KeyCode::KeyM) {
        apply_action(
            MainMenuAction::ToggleMute,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
    if keys.just_pressed(KeyCode::Escape) {
        apply_action(
            MainMenuAction::Back,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
    if keys.just_pressed(KeyCode::Enter) {
        let action = match menu_state.selection {
            1 => MainMenuAction::ToggleMute,
            2 => MainMenuAction::Back,
            _ => return,
        };
        apply_action(
            action,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
}

fn level_select_keys(
    keys: &ButtonInput<KeyCode>,
    backend: &SaveBackend,
    menu_state: &mut MainMenuState,
    dirty: &mut MainMenuUiDirty,
    save: &mut GameSave,
    bgm: &BgmState,
    sinks: &mut Query<&mut AudioSink>,
    registry: &LevelRegistry,
    level_selection: &mut LevelSelection,
    load_events: &mut MessageWriter<LoadLevelEvent>,
    cooldown: &mut InputCooldown,
    next: &mut NextState<AppState>,
) {
    let pack_count = registry.packs.len().max(1);
    if keys.just_pressed(KeyCode::ArrowUp) {
        level_selection.pack_index = (level_selection.pack_index + pack_count - 1) % pack_count;
        level_selection.level_index = 0;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        level_selection.pack_index = (level_selection.pack_index + 1) % pack_count;
        level_selection.level_index = 0;
    }
    if let Some(pack) = registry.pack_by_index(level_selection.pack_index) {
        let level_count = pack.levels.len().max(1);
        if keys.just_pressed(KeyCode::ArrowLeft) {
            level_selection.level_index =
                (level_selection.level_index + level_count - 1) % level_count;
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            level_selection.level_index = (level_selection.level_index + 1) % level_count;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        apply_action(
            MainMenuAction::Back,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
    if keys.just_pressed(KeyCode::Enter) {
        apply_action(
            MainMenuAction::PlayLevel,
            backend,
            menu_state,
            dirty,
            save,
            bgm,
            sinks,
            registry,
            level_selection,
            load_events,
            cooldown,
            next,
        );
    }
}

pub fn pointer_input(
    state: Res<State<AppState>>,
    mut menu_state: ResMut<MainMenuState>,
    mut dirty: ResMut<MainMenuUiDirty>,
    mut save: ResMut<GameSave>,
    backend: Res<SaveBackend>,
    bgm: Res<BgmState>,
    mut sinks: Query<&mut AudioSink>,
    registry: Res<LevelRegistry>,
    mut level_selection: ResMut<LevelSelection>,
    mut load_events: MessageWriter<LoadLevelEvent>,
    mut cooldown: ResMut<InputCooldown>,
    mut next: ResMut<NextState<AppState>>,
    mut interactions: Query<
        (&Interaction, &MainMenuAction, Option<&MainMenuItem>),
        Changed<Interaction>,
    >,
) {
    if *state.get() != AppState::MainMenu || cooldown.frames_remaining > 0 {
        return;
    }

    for (interaction, action, item) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(item) = item {
            menu_state.selection = item.index;
        }
        apply_action(
            *action,
            &backend,
            &mut menu_state,
            &mut dirty,
            &mut save,
            &bgm,
            &mut sinks,
            &registry,
            &mut level_selection,
            &mut load_events,
            &mut cooldown,
            &mut next,
        );
    }
}

fn apply_action(
    action: MainMenuAction,
    backend: &SaveBackend,
    menu_state: &mut MainMenuState,
    dirty: &mut MainMenuUiDirty,
    save: &mut GameSave,
    bgm: &BgmState,
    sinks: &mut Query<&mut AudioSink>,
    registry: &LevelRegistry,
    level_selection: &mut LevelSelection,
    load_events: &mut MessageWriter<LoadLevelEvent>,
    cooldown: &mut InputCooldown,
    next: &mut NextState<AppState>,
) {
    match action {
        MainMenuAction::Start => start_game(backend, registry, save, level_selection, load_events, cooldown, next),
        MainMenuAction::OpenLevelSelect => {
            resolve_last_level(registry, &save.0.profile, level_selection);
            menu_state.screen = MainMenuScreen::LevelSelect;
            menu_state.selection = 0;
            dirty.0 = true;
        }
        MainMenuAction::OpenSettings => {
            menu_state.screen = MainMenuScreen::Settings;
            menu_state.selection = 0;
            dirty.0 = true;
        }
        MainMenuAction::Back => {
            menu_state.screen = MainMenuScreen::Root;
            menu_state.selection = 0;
            dirty.0 = true;
        }
        MainMenuAction::MusicLess => {
            adjust_music_volume(backend, save, -MUSIC_VOLUME_STEP);
            apply_live_music_volume(save, bgm, sinks);
        }
        MainMenuAction::MusicMore => {
            adjust_music_volume(backend, save, MUSIC_VOLUME_STEP);
            apply_live_music_volume(save, bgm, sinks);
        }
        MainMenuAction::ToggleMute => {
            toggle_mute(backend, save);
            apply_live_music_volume(save, bgm, sinks);
            dirty.0 = true;
        }
        MainMenuAction::PackPrev => {
            let n = registry.packs.len().max(1);
            level_selection.pack_index = (level_selection.pack_index + n - 1) % n;
            level_selection.level_index = 0;
        }
        MainMenuAction::PackNext => {
            let n = registry.packs.len().max(1);
            level_selection.pack_index = (level_selection.pack_index + 1) % n;
            level_selection.level_index = 0;
        }
        MainMenuAction::LevelPrev => {
            if let Some(pack) = registry.pack_by_index(level_selection.pack_index) {
                let n = pack.levels.len().max(1);
                level_selection.level_index = (level_selection.level_index + n - 1) % n;
            }
        }
        MainMenuAction::LevelNext => {
            if let Some(pack) = registry.pack_by_index(level_selection.pack_index) {
                let n = pack.levels.len().max(1);
                level_selection.level_index = (level_selection.level_index + 1) % n;
            }
        }
        MainMenuAction::PlayLevel => {
            load_events.write(LoadLevelEvent { restart: false });
            begin_playing(cooldown);
            next.set(AppState::Playing);
        }
    }
}

fn start_game(
    _backend: &SaveBackend,
    registry: &LevelRegistry,
    save: &GameSave,
    selection: &mut LevelSelection,
    load_events: &mut MessageWriter<LoadLevelEvent>,
    cooldown: &mut InputCooldown,
    next: &mut NextState<AppState>,
) {
    resolve_last_level(registry, &save.0.profile, selection);
    load_events.write(LoadLevelEvent { restart: false });
    begin_playing(cooldown);
    next.set(AppState::Playing);
}
