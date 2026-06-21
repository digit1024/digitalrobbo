use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::audio::{is_muted, toggle_mute};
use crate::persistence::{GameFont, GameSave};
use crate::viewport::{self, DESIGN_HEIGHT, DESIGN_WIDTH};

#[derive(Component)]
pub struct MenuVisual;

#[derive(Component)]
pub struct MenuPlanet;

#[derive(Component)]
pub struct MenuText;

#[derive(Resource, Default)]
pub struct MenuSelection {
    pub index: usize,
}

#[derive(Resource)]
pub struct MenuPlanetAnim {
    pub target_y: f32,
    pub start_y: f32,
}

pub fn load_game_font(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(GameFont(
        asset_server.load("fonts/MarkerFelt.ttf"),
    ));
}

pub fn spawn_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    font: Res<GameFont>,
    save: Res<GameSave>,
    mut selection: ResMut<MenuSelection>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    selection.index = 0;

    let Ok(window) = window.get_single() else {
        return;
    };
    let scale = viewport::ui_scale(window);
    let cover = viewport::cover_scale(window, DESIGN_WIDTH, DESIGN_HEIGHT);
    let target_y = -DESIGN_HEIGHT * 0.05 * (window.height() / DESIGN_HEIGHT);
    let start_y = -window.height() * 0.65;

    commands.insert_resource(MenuPlanetAnim {
        target_y,
        start_y,
    });

    commands.spawn((
        Sprite {
            image: asset_server.load("ui/space.png"),
            ..default()
        },
        viewport::cover_transform(window, DESIGN_WIDTH, DESIGN_HEIGHT),
        MenuVisual,
    ));

    let planet_scale = cover * 0.55;
    commands.spawn((
        Sprite {
            image: asset_server.load("ui/planet.png"),
            ..default()
        },
        Transform::from_xyz(0.0, start_y, 1.0).with_scale(Vec3::splat(planet_scale)),
        MenuPlanet,
        MenuVisual,
    ));

    let label = menu_label(&selection, &save);
    commands.spawn((
        Text::new(label),
        TextFont {
            font: font.0.clone(),
            font_size: 32.0 * scale,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0 * scale),
            left: Val::Px(80.0 * scale),
            ..default()
        },
        MenuVisual,
        MenuText,
    ));
}

fn menu_label(selection: &MenuSelection, save: &GameSave) -> String {
    let mute_label = if is_muted(save) {
        "[M] Unmute"
    } else {
        "[M] Mute"
    };
    let body = if selection.index == 0 {
        "> START\n  SELECT LEVEL"
    } else {
        "  START\n> SELECT LEVEL"
    };
    format!("{body}\n\n{mute_label}\n\n[Up/Down] navigate  [Enter] confirm")
}

pub fn animate_menu_planet(
    time: Res<Time>,
    state: Res<State<AppState>>,
    anim: Option<Res<MenuPlanetAnim>>,
    mut planets: Query<&mut Transform, With<MenuPlanet>>,
) {
    if *state.get() != AppState::MainMenu {
        return;
    }
    let Some(anim) = anim else {
        return;
    };
    for mut tf in &mut planets {
        tf.translation.y = tf.translation.y.lerp(anim.target_y, time.delta_secs() * 1.2);
    }
}

pub fn update_menu_highlight(
    state: Res<State<AppState>>,
    selection: Res<MenuSelection>,
    save: Res<GameSave>,
    mut text: Query<&mut Text, With<MenuText>>,
) {
    if *state.get() != AppState::MainMenu {
        return;
    }
    let label = menu_label(&selection, &save);
    for mut t in &mut text {
        **t = label.clone();
    }
}

pub fn cleanup_main_menu(
    mut commands: Commands,
    q: Query<Entity, With<MenuVisual>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<MenuPlanetAnim>();
}

pub fn menu_navigate(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MenuSelection>,
    mut save: ResMut<GameSave>,
    state: Res<State<AppState>>,
) {
    if *state.get() != AppState::MainMenu {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        selection.index = selection.index.wrapping_sub(1) % 2;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        selection.index = (selection.index + 1) % 2;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        toggle_mute(&mut save);
    }
}
