//! Spawn main-menu world sprites and centered UI.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::layout::MainMenuLayout;
use super::state::{
    LevelSelectLevelLabel, LevelSelectPackLabel, LevelSelectStatusLabel, MainMenuAction,
    MainMenuBackground, MainMenuItem, MainMenuPlanet, MainMenuScreen, MainMenuState,
    MainMenuUiDirty, MainMenuUiRoot, MusicVolumeLabel,
};
use crate::audio::{is_muted, music_volume_percent};
use crate::menu::MainMenuOwned;
use crate::persistence::{GameFont, GameSave};
use crate::viewport::{self, DESIGN_HEIGHT, DESIGN_WIDTH};

const COLOR_ITEM: Color = Color::srgb(0.95, 0.95, 1.0);
const COLOR_HINT: Color = Color::srgba(0.85, 0.85, 0.95, 0.85);

pub fn spawn_world(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layout: &MainMenuLayout,
    window: &Window,
) {
    commands.spawn((
        Sprite {
            image: asset_server.load("ui/space.png"),
            ..default()
        },
        viewport::cover_transform(window, DESIGN_WIDTH, DESIGN_HEIGHT),
        MainMenuBackground,
        MainMenuOwned,
    ));

    commands.spawn((
        Sprite {
            image: asset_server.load("ui/planet.png"),
            ..default()
        },
        Transform::from_xyz(0.0, layout.planet_start_y, 1.0).with_scale(Vec3::splat(layout.cover)),
        MainMenuPlanet,
        MainMenuOwned,
    ));
}

pub fn spawn_ui(
    commands: &mut Commands,
    font: &GameFont,
    save: &GameSave,
    window: &Window,
    screen: MainMenuScreen,
) {
    let item_size = viewport::menu_item_font_size(window);
    let title_size = viewport::menu_title_font_size(window);
    let hint_size = viewport::menu_hint_font_size(window);
    let gap = 12.0 * viewport::ui_scale(window);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(10),
            MainMenuUiRoot,
            MainMenuOwned,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(gap),
                        ..default()
                    },
                ))
                .with_children(|column| {
                    column.spawn((
                        Text::new(screen.header()),
                        TextFont {
                            font: font.0.clone(),
                            font_size: title_size,
                            ..default()
                        },
                        TextColor(viewport::menu_title_text_color()),
                        Node {
                            margin: UiRect::bottom(Val::Px(gap * 0.5)),
                            ..default()
                        },
                    ));

                    match screen {
                        MainMenuScreen::Root => spawn_root(column, font, item_size),
                        MainMenuScreen::Settings => {
                            spawn_settings(column, font, item_size, hint_size, save)
                        }
                        MainMenuScreen::LevelSelect => {
                            spawn_level_select(column, font, item_size, hint_size)
                        }
                    }
                });
        });
}

fn spawn_root(parent: &mut ChildSpawnerCommands, font: &GameFont, item_size: f32) {
    spawn_item_button(parent, font, item_size, 0, "START", MainMenuAction::Start);
    spawn_item_button(
        parent,
        font,
        item_size,
        1,
        "SELECT LEVEL",
        MainMenuAction::OpenLevelSelect,
    );
    spawn_item_button(
        parent,
        font,
        item_size,
        2,
        "SETTINGS",
        MainMenuAction::OpenSettings,
    );
}

fn spawn_settings(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    item_size: f32,
    hint_size: f32,
    save: &GameSave,
) {
    let pct = music_volume_percent(save);
    let mute = if is_muted(save) { "UNMUTE" } else { "MUTE" };

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            MainMenuItem { index: 0 },
        ))
        .with_children(|row| {
            spawn_label(row, font, item_size, "MUSIC");
            spawn_inline_button(row, font, item_size, "<", MainMenuAction::MusicLess);
            row.spawn((
                Text::new(format!("{pct:>3}%")),
                TextFont {
                    font: font.0.clone(),
                    font_size: item_size,
                    ..default()
                },
                TextColor(viewport::menu_title_text_color()),
                MusicVolumeLabel,
            ));
            spawn_inline_button(row, font, item_size, ">", MainMenuAction::MusicMore);
        });

    spawn_item_button(parent, font, item_size, 1, mute, MainMenuAction::ToggleMute);
    spawn_item_button(parent, font, item_size, 2, "BACK", MainMenuAction::Back);
    spawn_hint(
        parent,
        font,
        hint_size,
        "[< / >] volume   [M] mute   [Esc] back",
    );
}

fn spawn_level_select(parent: &mut ChildSpawnerCommands, font: &GameFont, item_size: f32, hint_size: f32) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            MainMenuItem { index: 0 },
        ))
        .with_children(|row| {
            spawn_inline_button(row, font, item_size, "<", MainMenuAction::PackPrev);
            row.spawn((
                Text::new("Pack: …"),
                TextFont {
                    font: font.0.clone(),
                    font_size: item_size,
                    ..default()
                },
                TextColor(COLOR_ITEM),
                LevelSelectPackLabel,
            ));
            spawn_inline_button(row, font, item_size, ">", MainMenuAction::PackNext);
        });

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            MainMenuItem { index: 1 },
        ))
        .with_children(|row| {
            spawn_inline_button(row, font, item_size, "<", MainMenuAction::LevelPrev);
            row.spawn((
                Text::new("Level: …"),
                TextFont {
                    font: font.0.clone(),
                    font_size: item_size,
                    ..default()
                },
                TextColor(COLOR_ITEM),
                LevelSelectLevelLabel,
            ));
            spawn_inline_button(row, font, item_size, ">", MainMenuAction::LevelNext);
        });

    parent.spawn((
        Text::new(""),
        TextFont {
            font: font.0.clone(),
            font_size: hint_size,
            ..default()
        },
        TextColor(COLOR_HINT),
        LevelSelectStatusLabel,
    ));

    spawn_item_button(parent, font, item_size, 2, "PLAY", MainMenuAction::PlayLevel);
    spawn_item_button(parent, font, item_size, 3, "BACK", MainMenuAction::Back);
    spawn_hint(
        parent,
        font,
        hint_size,
        "[Up/Down] pack   [< / >] level   [Enter] play",
    );
}

fn spawn_item_button(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    font_size: f32,
    index: usize,
    label: &str,
    action: MainMenuAction,
) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            MainMenuItem { index },
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(format!("  {label}")),
                TextFont {
                    font: font.0.clone(),
                    font_size,
                    ..default()
                },
                TextColor(COLOR_ITEM),
            ));
        });
}

fn spawn_inline_button(
    parent: &mut ChildSpawnerCommands,
    font: &GameFont,
    font_size: f32,
    label: &str,
    action: MainMenuAction,
) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
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
                TextColor(COLOR_ITEM),
            ));
        });
}

fn spawn_label(parent: &mut ChildSpawnerCommands, font: &GameFont, font_size: f32, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.0.clone(),
            font_size,
            ..default()
        },
        TextColor(COLOR_ITEM),
    ));
}

fn spawn_hint(parent: &mut ChildSpawnerCommands, font: &GameFont, font_size: f32, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.0.clone(),
            font_size,
            ..default()
        },
        TextColor(COLOR_HINT),
        Node {
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        },
    ));
}

pub fn rebuild_ui(
    commands: &mut Commands,
    font: &GameFont,
    save: &GameSave,
    window: &Window,
    screen: MainMenuScreen,
    roots: &Query<Entity, With<MainMenuUiRoot>>,
) {
    for entity in roots.iter().collect::<Vec<_>>() {
        commands.entity(entity).despawn();
    }
    spawn_ui(commands, font, save, window, screen);
}

pub fn init_on_enter(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: &GameFont,
    save: &GameSave,
    window: &Window,
    menu_state: &mut MainMenuState,
    dirty: &mut MainMenuUiDirty,
) {
    menu_state.screen = MainMenuScreen::Root;
    menu_state.selection = 0;
    dirty.0 = false;

    let layout = MainMenuLayout::from_window(window);
    commands.insert_resource(layout);
    spawn_world(commands, asset_server, &layout, window);
    spawn_ui(commands, font, save, window, menu_state.screen);
}
