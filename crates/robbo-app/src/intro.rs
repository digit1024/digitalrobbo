use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::audio::AudioGate;
use crate::persistence::GameFont;
use crate::viewport::{self, DESIGN_HEIGHT, DESIGN_WIDTH};

#[derive(Component)]
pub struct IntroVisual;

#[derive(Component)]
pub struct IntroText;

#[derive(Resource)]
pub struct IntroSequence {
    pub phase: IntroPhase,
    pub timer: Timer,
    pub skipped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IntroPhase {
    Logo,
    Presents,
    Title,
    Done,
}

impl Default for IntroSequence {
    fn default() -> Self {
        Self {
            phase: IntroPhase::Logo,
            timer: Timer::from_seconds(2.0, TimerMode::Once),
            skipped: false,
        }
    }
}

pub fn setup_intro(mut commands: Commands) {
    commands.insert_resource(IntroSequence::default());
}

/// Start intro music immediately (original MenuScene played on init).
pub fn start_intro_audio(mut gate: ResMut<AudioGate>) {
    gate.unlocked = true;
}

pub fn spawn_intro(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    font: Res<GameFont>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };
    let scale = viewport::ui_scale(window);

    commands.spawn((
        Sprite {
            image: asset_server.load("ui/space.png"),
            ..default()
        },
        viewport::cover_transform(window, DESIGN_WIDTH, DESIGN_HEIGHT),
        IntroVisual,
    ));

    commands.spawn((
        Text::new("BMIdeas"),
        TextFont {
            font: font.0.clone(),
            font_size: 52.0 * scale,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        IntroVisual,
        IntroText,
    ));
}

pub fn update_intro_sequence(
    time: Res<Time>,
    state: Res<State<AppState>>,
    mut sequence: ResMut<IntroSequence>,
    mut next: ResMut<NextState<AppState>>,
    mut text: Query<&mut Text, With<IntroText>>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    if sequence.skipped || sequence.phase == IntroPhase::Done {
        return;
    }

    sequence.timer.tick(time.delta());
    if !sequence.timer.finished() {
        return;
    }

    sequence.phase = match sequence.phase {
        IntroPhase::Logo => {
            for mut t in &mut text {
                **t = "presents".into();
            }
            sequence.timer = Timer::from_seconds(2.0, TimerMode::Once);
            IntroPhase::Presents
        }
        IntroPhase::Presents => {
            for mut t in &mut text {
                **t = "DIGIT ADVENTURES".into();
            }
            sequence.timer = Timer::from_seconds(2.5, TimerMode::Once);
            IntroPhase::Title
        }
        IntroPhase::Title => {
            sequence.timer = Timer::from_seconds(0.5, TimerMode::Once);
            IntroPhase::Done
        }
        IntroPhase::Done => IntroPhase::Done,
    };

    if sequence.phase == IntroPhase::Done {
        next.set(AppState::MainMenu);
    }
}

pub fn intro_skip_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<AppState>>,
    mut sequence: ResMut<IntroSequence>,
    mut gate: ResMut<AudioGate>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        gate.unlocked = true;
        sequence.skipped = true;
        sequence.phase = IntroPhase::Done;
        next.set(AppState::MainMenu);
    }
}

pub fn cleanup_intro(mut commands: Commands, q: Query<Entity, With<IntroVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
