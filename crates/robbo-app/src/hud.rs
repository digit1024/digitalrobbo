//! In-level HUD: semi-transparent top bar, collectible counters, menu buttons.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, LoadLevelEvent};
use crate::input::InputCooldown;
use crate::persistence::GameFont;
use crate::viewport;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct HudKeysText;

#[derive(Component)]
pub struct HudScrewsText;

#[derive(Component)]
pub struct HudAmmoText;

#[derive(Component, Clone, Copy)]
pub enum HudButton {
    PauseMenu,
    Restart,
}

pub fn spawn_level_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    font: Res<GameFont>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };
    let scale = viewport::ui_scale(window);
    let bar_h = 56.0 * scale;
    let icon = 36.0 * scale;
    let font_size = 26.0 * scale;
    let btn = 52.0 * scale;
    let pad = 10.0 * scale;
    let gap = pad * 2.0;

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(bar_h),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(pad)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.58)),
            ZIndex(100),
            HudRoot,
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(gap),
                    ..default()
                },
            ))
            .with_children(|stats| {
                spawn_stat(
                    stats,
                    &asset_server,
                    &font,
                    icon,
                    font_size,
                    "sprites/key.png",
                    "0",
                    HudKeysText,
                );
                spawn_stat(
                    stats,
                    &asset_server,
                    &font,
                    icon,
                    font_size,
                    "sprites/screw.png",
                    "0/0",
                    HudScrewsText,
                );
                spawn_stat(
                    stats,
                    &asset_server,
                    &font,
                    icon,
                    font_size,
                    "sprites/bullet.png",
                    "0",
                    HudAmmoText,
                );
            });

            bar.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(pad * 0.4),
                    ..default()
                },
            ))
            .with_children(|buttons| {
                spawn_hud_button(
                    buttons,
                    &asset_server,
                    btn,
                    "ui/settingss.png",
                    HudButton::PauseMenu,
                );
                spawn_hud_button(
                    buttons,
                    &asset_server,
                    btn,
                    "ui/replays.png",
                    HudButton::Restart,
                );
            });
        });
}

fn spawn_stat<M: Component>(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    font: &GameFont,
    icon_px: f32,
    font_size: f32,
    icon_path: &str,
    initial: &str,
    marker: M,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                ImageNode {
                    image: asset_server.load(icon_path),
                    ..default()
                },
                Node {
                    width: Val::Px(icon_px),
                    height: Val::Px(icon_px),
                    ..default()
                },
            ));
            row.spawn((
                Text::new(initial),
                TextFont {
                    font: font.0.clone(),
                    font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.97, 0.97, 1.0)),
                marker,
            ));
        });
}

fn spawn_hud_button(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    size: f32,
    icon_path: &str,
    action: HudButton,
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
                    image: asset_server.load(icon_path),
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

pub fn update_level_hud(
    state: Res<State<AppState>>,
    bridge: Res<CoreBridge>,
    mut keys: Query<&mut Text, (With<HudKeysText>, Without<HudScrewsText>, Without<HudAmmoText>)>,
    mut screws: Query<
        &mut Text,
        (With<HudScrewsText>, Without<HudKeysText>, Without<HudAmmoText>),
    >,
    mut ammo: Query<&mut Text, (With<HudAmmoText>, Without<HudKeysText>, Without<HudScrewsText>)>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    let world = &bridge.world;
    let keys_text = format!("{}", world.keys);
    let screws_text = format!("{}/{}", world.collected_screws, world.total_screws);
    let ammo_text = format!("{}", world.ammo);
    for mut text in &mut keys {
        if **text != keys_text {
            **text = keys_text.clone();
        }
    }
    for mut text in &mut screws {
        if **text != screws_text {
            **text = screws_text.clone();
        }
    }
    for mut text in &mut ammo {
        if **text != ammo_text {
            **text = ammo_text.clone();
        }
    }
}

pub fn hud_button_input(
    state: Res<State<AppState>>,
    mut interactions: Query<(&Interaction, &HudButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut load_events: EventWriter<LoadLevelEvent>,
    mut cooldown: ResMut<InputCooldown>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            HudButton::PauseMenu => next.set(AppState::Paused),
            HudButton::Restart => {
                load_events.send(LoadLevelEvent { restart: true });
                cooldown.frames_remaining = 0;
            }
        }
    }
}

pub fn cleanup_level_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}
