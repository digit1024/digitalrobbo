//! In-game modal menus: pause, level complete (victory), game over.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::bridge::{GameSession, LoadLevelEvent};
use crate::input::{begin_playing, InputCooldown};
use crate::levels::{LevelRegistry, LevelSelection};
use crate::persistence::{GameFont, GameSave};
use crate::ui::SpeedrunTimer;
use crate::viewport;

const PANEL_W: f32 = 844.0;
const PANEL_H: f32 = 528.0;
const PANEL_PAD: f32 = 36.0;

#[derive(Component)]
pub struct GameMenuRoot;

#[derive(Component, Clone, Copy)]
pub enum GameMenuAction {
    Resume,
    Restart,
    MainMenu,
    NextLevel,
    Retry,
    LevelSelect,
    Share,
}

pub fn spawn_game_menu(
    mut commands: Commands,
    state: Res<State<AppState>>,
    existing: Query<Entity, With<GameMenuRoot>>,
    asset_server: Res<AssetServer>,
    font: Res<GameFont>,
    save: Res<GameSave>,
    session: Res<GameSession>,
    timer: Res<SpeedrunTimer>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    dismiss_game_menu(&mut commands, &existing);

    let Ok(window) = window.single() else {
        return;
    };
    match state.get() {
        AppState::Paused => spawn_pause_menu(&mut commands, &asset_server, &font, window),
        AppState::LevelComplete => spawn_victory_menu(
            &mut commands,
            &asset_server,
            &font,
            window,
            &save,
            &session,
            &timer,
        ),
        AppState::GameOver => {
            spawn_defeat_menu(&mut commands, &asset_server, &font, window, &session, &timer)
        }
        AppState::LevelSelect => spawn_level_select_menu(
            &mut commands,
            &font,
            window,
            &registry,
            &selection,
            &save,
        ),
        _ => {}
    }
}

fn menu_scale(window: &Window) -> f32 {
    let scale = viewport::ui_scale(window);
    let max_w = window.width() * 0.72;
    (max_w / PANEL_W).min(1.0) * scale
}

fn panel_content_width(panel_w: f32, scale: f32) -> f32 {
    panel_w - 2.0 * PANEL_PAD * scale
}

fn spawn_backdrop(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
        ZIndex(0),
    ));
}

fn spawn_pause_menu(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: &GameFont,
    window: &Window,
) {
    let scale = menu_scale(window);
    let title_size = 34.0 * scale;
    let item_size = 28.0 * scale;
    let panel_w = PANEL_W * scale;
    let panel_h = PANEL_H * scale;

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(150),
            GameMenuRoot,
        ))
        .with_children(|root| {
            spawn_backdrop(root);
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ZIndex(1),
            ))
            .with_children(|center| {
            center.spawn((
                Node {
                    width: Val::Px(panel_w),
                    height: Val::Px(panel_h),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(14.0 * scale),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn((
                    ImageNode {
                        image: asset_server.load("ui/menu_panel.png"),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
                spawn_title(
                    panel,
                    font,
                    title_size,
                    scale,
                    panel_w,
                    "PAUSED",
                );
                spawn_text_button(panel, font, item_size, "RESUME", GameMenuAction::Resume);
                spawn_text_button(panel, font, item_size, "Reset", GameMenuAction::Restart);
                spawn_text_button(panel, font, item_size, "Main Menu", GameMenuAction::MainMenu);
            });
            });
        });
}

fn spawn_victory_menu(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: &GameFont,
    window: &Window,
    save: &GameSave,
    session: &GameSession,
    timer: &SpeedrunTimer,
) {
    let scale = menu_scale(window);
    let title_size = 34.0 * scale;
    let row_size = 22.0 * scale;
    let panel_w = PANEL_W * scale;
    let panel_h = PANEL_H * scale;
    let content_w = panel_content_width(panel_w, scale);
    let btn = 58.0 * scale;

    let key = (session.level_index + 1).to_string();
    let progress = save
        .0
        .packs
        .get(&session.pack_name)
        .and_then(|p| p.levels.get(&key));
    let best_time_secs = progress.map(|p| p.best_time_ms / 1000).unwrap_or(0);
    let best_tries = progress.map(|p| p.best_tries).unwrap_or(session.tries);
    let time_secs = timer.elapsed_ms / 1000;
    let best_time = format!("{best_time_secs}");
    let best_tries_str = format!("{best_tries}");
    let time_str = format!("{time_secs}");
    let tries_str = format!("{}", session.tries);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(150),
            GameMenuRoot,
        ))
        .with_children(|root| {
            spawn_backdrop(root);
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ZIndex(1),
            ))
            .with_children(|center| {
            center.spawn((
                Node {
                    width: Val::Px(panel_w),
                    height: Val::Px(panel_h),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    padding: UiRect::all(Val::Px(PANEL_PAD * scale)),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn((
                    ImageNode {
                        image: asset_server.load("ui/menu_panel.png"),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
                spawn_title(panel, font, title_size, scale, content_w, "WELL DONE!!!");
                panel
                    .spawn((
                        Node {
                            width: Val::Px(content_w),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ))
                    .with_children(|stats_zone| {
                        spawn_stats_block(
                            stats_zone,
                            font,
                            row_size,
                            scale,
                            content_w,
                            &[
                                ("BEST TIME:", best_time.as_str()),
                                ("MIN TRIES:", best_tries_str.as_str()),
                                ("TIME:", time_str.as_str()),
                                ("TRIES:", tries_str.as_str()),
                            ],
                        );
                    });

                panel
                    .spawn((
                        Node {
                            width: Val::Px(content_w),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(18.0 * scale),
                            padding: UiRect::top(Val::Px(8.0 * scale)),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        spawn_icon_button(
                            row,
                            asset_server,
                            btn,
                            "ui/facebook.png",
                            GameMenuAction::Share,
                        );
                        spawn_icon_button(
                            row,
                            asset_server,
                            btn,
                            "ui/replay.png",
                            GameMenuAction::Restart,
                        );
                        spawn_icon_button(
                            row,
                            asset_server,
                            btn,
                            "ui/next.png",
                            GameMenuAction::NextLevel,
                        );
                    });
            });
            });
        });
}

fn spawn_defeat_menu(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: &GameFont,
    window: &Window,
    session: &GameSession,
    timer: &SpeedrunTimer,
) {
    let scale = menu_scale(window);
    let title_size = 34.0 * scale;
    let row_size = 22.0 * scale;
    let panel_w = PANEL_W * scale;
    let panel_h = PANEL_H * scale;
    let content_w = panel_content_width(panel_w, scale);
    let btn = 58.0 * scale;
    let time_secs = timer.elapsed_ms / 1000;
    let time_str = format!("{time_secs}");
    let tries_str = format!("{}", session.tries);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(150),
            GameMenuRoot,
        ))
        .with_children(|root| {
            spawn_backdrop(root);
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ZIndex(1),
            ))
            .with_children(|center| {
            center.spawn((
                Node {
                    width: Val::Px(panel_w),
                    height: Val::Px(panel_h),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    padding: UiRect::all(Val::Px(PANEL_PAD * scale)),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn((
                    ImageNode {
                        image: asset_server.load("ui/menu_panel.png"),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
                spawn_title(panel, font, title_size, scale, content_w, "GAME OVER");
                panel
                    .spawn((
                        Node {
                            width: Val::Px(content_w),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ))
                    .with_children(|stats_zone| {
                        spawn_stats_block(
                            stats_zone,
                            font,
                            row_size,
                            scale,
                            content_w,
                            &[
                                ("TIME:", time_str.as_str()),
                                ("TRIES:", tries_str.as_str()),
                            ],
                        );
                    });

                panel
                    .spawn((
                        Node {
                            width: Val::Px(content_w),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(18.0 * scale),
                            padding: UiRect::top(Val::Px(8.0 * scale)),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        spawn_icon_button(
                            row,
                            asset_server,
                            btn,
                            "ui/replay.png",
                            GameMenuAction::Retry,
                        );
                        spawn_text_button(row, font, 24.0 * scale, "Main Menu", GameMenuAction::MainMenu);
                    });
            });
            });
        });
}

fn spawn_level_select_menu(
    commands: &mut Commands,
    font: &GameFont,
    window: &Window,
    registry: &LevelRegistry,
    selection: &LevelSelection,
    save: &GameSave,
) {
    let scale = viewport::ui_scale(window);
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
    let label = format!(
        "LEVEL SELECT\n\nPack:  {}  ({}/{} packs)\nLevel: {} / {}{}\n\n[Up/Down] change pack   [Left/Right] change level\n[Enter] play            [Esc] back",
        pack_name,
        selection.pack_index + 1,
        registry.packs.len(),
        selection.level_index + 1,
        level_count,
        star,
    );

    commands.spawn((
        Text::new(label),
        TextFont {
            font: font.0.clone(),
            font_size: 24.0 * scale,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.92, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(60.0),
            left: Val::Px(40.0),
            ..default()
        },
        ZIndex(50),
        GameMenuRoot,
    ));
}

fn spawn_title(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    font_size: f32,
    scale: f32,
    content_w: f32,
    text: &str,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.0.clone(),
            font_size,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.97, 0.97, 1.0)),
        Node {
            width: Val::Px(content_w),
            margin: UiRect::bottom(Val::Px(12.0 * scale)),
            ..default()
        },
    ));
}

fn spawn_stats_block(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    row_size: f32,
    scale: f32,
    content_w: f32,
    rows: &[(&str, &str)],
) {
    parent
        .spawn((
            Node {
                width: Val::Px(content_w),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0 * scale),
                padding: UiRect::horizontal(Val::Px(24.0 * scale)),
                ..default()
            },
        ))
        .with_children(|block| {
            for (label, value) in rows {
                spawn_stat_row(block, font, row_size, label, value);
            }
        });
}

fn spawn_stat_row(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    font_size: f32,
    label: &str,
    value: &str,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.0.clone(),
                    font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.92, 1.0)),
            ));
            row.spawn((
                Text::new(value),
                TextFont {
                    font: font.0.clone(),
                    font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.97, 0.97, 1.0)),
            ));
        });
}

fn spawn_text_button(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    font_size: f32,
    label: &str,
    action: GameMenuAction,
) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.18, 0.45)),
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.0.clone(),
                    font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 1.0)),
            ));
        });
}

fn spawn_icon_button(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    size: f32,
    icon_path: &str,
    action: GameMenuAction,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                ImageNode {
                    image: asset_server.load(icon_path.to_string()),
                    ..default()
                },
                Node {
                    width: Val::Px(size * 0.92),
                    height: Val::Px(size * 0.92),
                    ..default()
                },
            ));
        });
}

pub fn update_level_select_menu(
    state: Res<State<AppState>>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    save: Res<GameSave>,
    mut menus: Query<&mut Text, With<GameMenuRoot>>,
) {
    if *state.get() != AppState::LevelSelect {
        return;
    }
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
    let label = format!(
        "LEVEL SELECT\n\nPack:  {}  ({}/{} packs)\nLevel: {} / {}{}\n\n[Up/Down] change pack   [Left/Right] change level\n[Enter] play            [Esc] back",
        pack_name,
        selection.pack_index + 1,
        registry.packs.len(),
        selection.level_index + 1,
        level_count,
        star,
    );
    for mut text in &mut menus {
        if **text != label {
            **text = label.clone();
        }
    }
}

pub fn dismiss_game_menu(commands: &mut Commands, roots: &Query<Entity, With<GameMenuRoot>>) {
    for entity in roots.iter().collect::<Vec<_>>() {
        commands.entity(entity).despawn();
    }
}

pub fn game_menu_button_input(
    mut commands: Commands,
    state: Res<State<AppState>>,
    menu_roots: Query<Entity, With<GameMenuRoot>>,
    mut interactions: Query<(&Interaction, &GameMenuAction), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut load_events: MessageWriter<LoadLevelEvent>,
    mut cooldown: ResMut<InputCooldown>,
    mut selection: ResMut<LevelSelection>,
    registry: Res<LevelRegistry>,
) {
    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let dismiss = match (state.get(), *action) {
            (AppState::Paused, GameMenuAction::Resume) => {
                next.set(AppState::Playing);
                true
            }
            (AppState::Paused, GameMenuAction::Restart) => {
                load_events.write(LoadLevelEvent { restart: true });
                next.set(AppState::Playing);
                cooldown.frames_remaining = 0;
                true
            }
            (AppState::Paused, GameMenuAction::MainMenu) => {
                next.set(AppState::MainMenu);
                true
            }

            (AppState::LevelComplete, GameMenuAction::Restart) => {
                load_events.write(LoadLevelEvent { restart: true });
                begin_playing(&mut cooldown);
                next.set(AppState::Playing);
                true
            }
            (AppState::LevelComplete, GameMenuAction::NextLevel) => {
                if let Some(pack) = registry.pack_by_index(selection.pack_index) {
                    if selection.level_index + 1 < pack.levels.len() {
                        selection.level_index += 1;
                        load_events.write(LoadLevelEvent { restart: false });
                        begin_playing(&mut cooldown);
                        next.set(AppState::Playing);
                        true
                    } else {
                        next.set(AppState::LevelSelect);
                        true
                    }
                } else {
                    next.set(AppState::LevelSelect);
                    true
                }
            }
            (AppState::LevelComplete, GameMenuAction::Share) => {
                bevy::log::info!("Share not wired on desktop yet");
                false
            }
            (AppState::LevelComplete, GameMenuAction::MainMenu) => {
                next.set(AppState::MainMenu);
                true
            }

            (AppState::GameOver, GameMenuAction::Retry) => {
                load_events.write(LoadLevelEvent { restart: true });
                begin_playing(&mut cooldown);
                next.set(AppState::Playing);
                true
            }
            (AppState::GameOver, GameMenuAction::MainMenu) => {
                next.set(AppState::MainMenu);
                true
            }
            (AppState::GameOver, GameMenuAction::LevelSelect) => {
                next.set(AppState::LevelSelect);
                true
            }

            _ => false,
        };
        if dismiss {
            dismiss_game_menu(&mut commands, &menu_roots);
        }
    }
}

pub fn cleanup_game_menu(mut commands: Commands, roots: Query<Entity, With<GameMenuRoot>>) {
    dismiss_game_menu(&mut commands, &roots);
}
