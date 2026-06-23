//! Android-only on-screen move and shoot pads.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use robbo_core::Direction;

use crate::app_state::AppState;
use crate::bridge::CoreBridge;
use crate::input::{apply_move_pad_hold, apply_move_pad_press, apply_move_pad_release, apply_shoot_pad_press, SteeringState};
use crate::ui::LevelCountdown;
use crate::viewport;

#[derive(Component)]
pub struct TouchControlsRoot;

#[derive(Component, Clone, Copy)]
pub enum MovePadDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component, Clone, Copy)]
pub enum ShootPadDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
pub(crate) struct PadButton {
    pressed: bool,
}

fn dir_from_move(d: MovePadDir) -> Direction {
    match d {
        MovePadDir::Up => Direction::Up,
        MovePadDir::Down => Direction::Down,
        MovePadDir::Left => Direction::Left,
        MovePadDir::Right => Direction::Right,
    }
}

fn dir_from_shoot(d: ShootPadDir) -> Direction {
    match d {
        ShootPadDir::Up => Direction::Up,
        ShootPadDir::Down => Direction::Down,
        ShootPadDir::Left => Direction::Left,
        ShootPadDir::Right => Direction::Right,
    }
}

pub fn spawn_touch_controls(mut commands: Commands, window: Query<&Window, With<PrimaryWindow>>) {
    let Ok(window) = window.get_single() else {
        return;
    };
    let scale = viewport::ui_scale(window);
    let margin = 24.0 * scale;
    let pad = 150.0 * scale;
    let btn = pad * 0.34;

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(90),
            TouchControlsRoot,
        ))
        .with_children(|root| {
            spawn_directional_pad(
                root,
                pad,
                btn,
                margin,
                true,
                Color::srgba(0.08, 0.12, 0.22, 0.62),
                Color::srgba(0.35, 0.65, 1.0, 0.85),
            );
            spawn_directional_pad(
                root,
                pad,
                btn,
                margin,
                false,
                Color::srgba(0.22, 0.10, 0.08, 0.62),
                Color::srgba(1.0, 0.55, 0.25, 0.9),
            );
        });
}

fn spawn_directional_pad(
    parent: &mut ChildBuilder,
    pad: f32,
    btn: f32,
    margin: f32,
    left_side: bool,
    base: Color,
    accent: Color,
) {
    let horizontal = if left_side {
        Val::Px(margin)
    } else {
        Val::Auto
    };
    let horizontal_end = if left_side {
        Val::Auto
    } else {
        Val::Px(margin)
    };

    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: horizontal,
                right: horizontal_end,
                bottom: Val::Px(margin),
                width: Val::Px(pad),
                height: Val::Px(pad),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(base),
            BorderRadius::all(Val::Px(pad * 0.18)),
        ))
        .with_children(|pad_root| {
            let gap = pad * 0.04;
            pad_root
                .spawn(Node {
                    width: Val::Px(btn * 3.0 + gap * 2.0),
                    height: Val::Px(btn * 3.0 + gap * 2.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|grid| {
                    spawn_pad_btn(grid, btn, "▲", accent, left_side, 0);
                    grid.spawn(Node {
                        width: Val::Px(btn * 3.0 + gap * 2.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_pad_btn(row, btn, "◀", accent, left_side, 2);
                        row.spawn((
                            Node {
                                width: Val::Px(btn),
                                height: Val::Px(btn),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.15)),
                            BorderRadius::all(Val::Px(btn * 0.2)),
                        ))
                        .with_children(|center| {
                            center.spawn((
                                Text::new(if left_side { "MOVE" } else { "FIRE" }),
                                TextFont {
                                    font_size: btn * 0.22,
                                    ..default()
                                },
                                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.75)),
                            ));
                        });
                        spawn_pad_btn(row, btn, "▶", accent, left_side, 3);
                    });
                    spawn_pad_btn(grid, btn, "▼", accent, left_side, 1);
                });
        });
}

fn spawn_pad_btn(
    parent: &mut ChildBuilder,
    size: f32,
    label: &str,
    accent: Color,
    move_pad: bool,
    dir_idx: u8,
) {
    let mut entity = parent.spawn((
        Button,
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.55)),
        BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
        BorderRadius::all(Val::Px(size * 0.22)),
        PadButton { pressed: false },
    ));
    if move_pad {
        entity.insert(match dir_idx {
            0 => MovePadDir::Up,
            1 => MovePadDir::Down,
            2 => MovePadDir::Left,
            _ => MovePadDir::Right,
        });
    } else {
        entity.insert(match dir_idx {
            0 => ShootPadDir::Up,
            1 => ShootPadDir::Down,
            2 => ShootPadDir::Left,
            _ => ShootPadDir::Right,
        });
    }
    entity.with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont {
                font_size: size * 0.42,
                ..default()
            },
            TextColor(accent),
        ));
    });
}

pub fn move_pad_input(
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    mut buttons: Query<
        (
            &Interaction,
            &MovePadDir,
            &mut PadButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
) {
    if *state.get() != AppState::Playing || countdown.blocks_input() {
        return;
    }
    for (interaction, dir, mut pad, mut bg, mut border) in &mut buttons {
        let d = dir_from_move(*dir);
        match *interaction {
            Interaction::Pressed => {
                if !pad.pressed {
                    apply_move_pad_press(&mut bridge, &mut steering, d);
                    pad.pressed = true;
                }
                apply_move_pad_hold(&mut steering, d);
                *bg = BackgroundColor(Color::srgba(0.35, 0.65, 1.0, 0.35));
                *border = BorderColor(Color::srgba(0.5, 0.8, 1.0, 0.9));
            }
            Interaction::None => {
                if pad.pressed {
                    apply_move_pad_release(&mut steering, d);
                    pad.pressed = false;
                }
                *bg = BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.55));
                *border = BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.12));
            }
            _ => {}
        }
    }
}

pub fn move_pad_hold(
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    mut steering: ResMut<SteeringState>,
    buttons: Query<(&Interaction, &MovePadDir, &PadButton)>,
) {
    if *state.get() != AppState::Playing || countdown.blocks_input() {
        return;
    }
    for (interaction, dir, pad) in &buttons {
        if !pad.pressed {
            continue;
        }
        if matches!(*interaction, Interaction::Pressed | Interaction::Hovered) {
            apply_move_pad_hold(&mut steering, dir_from_move(*dir));
        }
    }
}

pub fn shoot_pad_input(
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    mut buttons: Query<
        (&Interaction, &ShootPadDir, &mut BackgroundColor, &mut BorderColor),
        Changed<Interaction>,
    >,
) {
    if *state.get() != AppState::Playing || countdown.blocks_input() {
        return;
    }
    for (interaction, dir, mut bg, mut border) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                apply_shoot_pad_press(&mut bridge, &mut steering, dir_from_shoot(*dir));
                *bg = BackgroundColor(Color::srgba(1.0, 0.45, 0.15, 0.45));
                *border = BorderColor(Color::srgba(1.0, 0.7, 0.3, 0.95));
            }
            Interaction::None => {
                *bg = BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.55));
                *border = BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.12));
            }
            _ => {}
        }
    }
}

pub fn cleanup_touch_controls(mut commands: Commands, q: Query<Entity, With<TouchControlsRoot>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
