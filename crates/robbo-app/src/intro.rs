//! Intro: BMIdeas logo → presents → title fly-off → MainMenu.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::audio::{play_menu_bgm_now, AudioGate};
use crate::persistence::{persist_save, GameFont, GameSave, SaveBackend};
use crate::ui_anim::UiFade;
use crate::viewport::{self, design_center, DESIGN_HEIGHT, DESIGN_WIDTH};

const LOGO_FADE_IN: f32 = 0.25;
const LOGO_FADE_OUT: f32 = 0.5;

const PRESENTS_FADE_IN: f32 = 0.25;
const PRESENTS_FADE_OUT: f32 = 0.5;
const SPACE_FADE_IN: f32 = 1.0;

const TITLE_FADE_IN: f32 = 0.25;
const TITLE_FLY: f32 = 1.0;

const LOGO_TEX_WIDTH: f32 = 623.0;
const TITLE_SCALE: f32 = 2.0;

#[derive(Component)]
pub struct IntroVisual;

#[derive(Component)]
pub struct IntroSpace;

#[derive(Component)]
pub struct IntroLogo;

#[derive(Component)]
pub struct IntroPresents;

#[derive(Component)]
pub struct IntroTitle;

#[derive(Resource, Default)]
pub struct IntroSpawned(bool);

#[derive(Resource)]
pub struct IntroLayout {
    pub ui_scale: f32,
    pub title_font: f32,
    pub logo_scale: f32,
    pub center_world: Vec3,
}

#[derive(Resource)]
pub struct IntroSequence {
    pub phase: IntroPhase,
    pub timer: Timer,
    pub skipped: bool,
    pub music_started: bool,
    pub space_fade_elapsed: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntroPhase {
    LogoFadeIn,
    LogoFadeOut,
    PresentsIn,
    PresentsOut,
    TitleIn,
    TitleFly,
    Done,
}

impl Default for IntroSequence {
    fn default() -> Self {
        Self {
            phase: IntroPhase::LogoFadeIn,
            timer: Timer::from_seconds(LOGO_FADE_IN, TimerMode::Once),
            skipped: false,
            music_started: false,
            space_fade_elapsed: 0.0,
        }
    }
}

pub fn setup_intro(
    mut commands: Commands,
    save: Res<GameSave>,
    mut next: ResMut<NextState<AppState>>,
) {
    if save.0.profile.intro_seen {
        next.set(AppState::MainMenu);
        return;
    }
    commands.insert_resource(IntroSequence::default());
    commands.insert_resource(IntroSpawned::default());
}

pub fn start_intro_audio(mut gate: ResMut<AudioGate>) {
    gate.unlocked = true;
}

fn layout_from_window(window: &Window) -> IntroLayout {
    let ui_scale = viewport::ui_scale(window);
    let center = viewport::design_to_world(design_center(), window);
    let logo_scale = (window.width() * 0.46) / LOGO_TEX_WIDTH;
    IntroLayout {
        ui_scale,
        title_font: 28.0 * ui_scale * TITLE_SCALE,
        logo_scale,
        center_world: center.extend(2.0),
    }
}

/// Spawn visuals once the primary window is available (avoids stuck sequence with no layout).
pub fn spawn_intro(
    state: Res<State<AppState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    font: Res<GameFont>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut spawned: Option<ResMut<IntroSpawned>>,
    sequence: Option<Res<IntroSequence>>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    let Some(mut spawned) = spawned else {
        return;
    };
    if spawned.0 {
        return;
    }
    if sequence.is_none() {
        return;
    }
    let Ok(window) = window.get_single() else {
        return;
    };
    spawned.0 = true;

    let layout = layout_from_window(window);
    let center_world = layout.center_world;
    let logo_scale = layout.logo_scale;
    let ui_scale = layout.ui_scale;
    commands.insert_resource(layout);

    commands.spawn((
        Sprite {
            image: asset_server.load("ui/space.png"),
            color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            ..default()
        },
        viewport::cover_transform(window, DESIGN_WIDTH, DESIGN_HEIGHT),
        IntroVisual,
        IntroSpace,
    ));

    commands.spawn((
        Sprite {
            image: asset_server.load("ui/bmideas.png"),
            color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            ..default()
        },
        Transform::from_translation(center_world).with_scale(Vec3::splat(logo_scale)),
        Visibility::Visible,
        IntroVisual,
        IntroLogo,
        UiFade::new(LOGO_FADE_IN, 0.0, 1.0),
    ));

    commands.spawn((
        Text::new("presents"),
        TextFont {
            font: font.0.clone(),
            font_size: 40.0 * ui_scale,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        IntroVisual,
        IntroPresents,
        Visibility::Hidden,
    ));
}

pub fn update_intro_sequence(
    time: Res<Time>,
    state: Res<State<AppState>>,
    spawned: Option<Res<IntroSpawned>>,
    layout: Option<Res<IntroLayout>>,
    mut sequence: Option<ResMut<IntroSequence>>,
    mut next: ResMut<NextState<AppState>>,
    mut save: ResMut<GameSave>,
    backend: Res<SaveBackend>,
    mut commands: Commands,
    font: Res<GameFont>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut space: Query<&mut Sprite, With<IntroSpace>>,
    mut logos: Query<(Entity, &mut Visibility), With<IntroLogo>>,
    mut presents: Query<(Entity, &mut Visibility), (With<IntroPresents>, Without<IntroLogo>)>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    let Some(mut sequence) = sequence else {
        return;
    };
    if sequence.skipped || sequence.phase == IntroPhase::Done {
        return;
    }
    if !spawned.as_ref().is_some_and(|s| s.0) {
        return;
    }

    if sequence.phase >= IntroPhase::PresentsIn && sequence.space_fade_elapsed < SPACE_FADE_IN {
        sequence.space_fade_elapsed += time.delta_secs();
        let alpha = (sequence.space_fade_elapsed / SPACE_FADE_IN).clamp(0.0, 1.0);
        for mut sprite in &mut space {
            sprite.color = sprite.color.with_alpha(alpha);
        }
    }

    sequence.timer.tick(time.delta());
    if !sequence.timer.finished() {
        return;
    }

    sequence.phase = match sequence.phase {
        IntroPhase::LogoFadeIn => {
            for (entity, mut vis) in &mut logos {
                *vis = Visibility::Visible;
                commands
                    .entity(entity)
                    .insert(UiFade::new(LOGO_FADE_OUT, 1.0, 0.0));
            }
            sequence.timer = Timer::from_seconds(LOGO_FADE_OUT, TimerMode::Once);
            IntroPhase::LogoFadeOut
        }
        IntroPhase::LogoFadeOut => {
            for (_, mut vis) in &mut logos {
                *vis = Visibility::Hidden;
            }
            for (entity, mut vis) in &mut presents {
                *vis = Visibility::Visible;
                commands
                    .entity(entity)
                    .insert(UiFade::new(PRESENTS_FADE_IN, 0.0, 1.0));
            }
            sequence.timer = Timer::from_seconds(PRESENTS_FADE_IN, TimerMode::Once);
            IntroPhase::PresentsIn
        }
        IntroPhase::PresentsIn => {
            for (entity, _) in &mut presents {
                commands
                    .entity(entity)
                    .insert(UiFade::new(PRESENTS_FADE_OUT, 1.0, 0.0));
            }
            sequence.timer = Timer::from_seconds(PRESENTS_FADE_OUT, TimerMode::Once);
            IntroPhase::PresentsOut
        }
        IntroPhase::PresentsOut => {
            for (_, mut vis) in &mut presents {
                *vis = Visibility::Hidden;
            }
            let title_font = layout
                .as_ref()
                .map(|l| l.title_font)
                .or_else(|| window.get_single().ok().map(|w| layout_from_window(w).title_font))
                .unwrap_or(56.0);
            commands.spawn((
                Text::new("DIGIT ADVENTURES"),
                TextFont {
                    font: font.0.clone(),
                    font_size: title_font,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                IntroVisual,
                IntroTitle,
                UiFade::new(TITLE_FADE_IN, 0.0, 1.0),
            ));
            sequence.timer = Timer::from_seconds(TITLE_FADE_IN, TimerMode::Once);
            IntroPhase::TitleIn
        }
        IntroPhase::TitleIn => {
            sequence.timer = Timer::from_seconds(TITLE_FLY, TimerMode::Once);
            IntroPhase::TitleFly
        }
        IntroPhase::TitleFly => IntroPhase::Done,
        IntroPhase::Done => IntroPhase::Done,
    };

    if sequence.phase == IntroPhase::Done {
        finish_intro(&backend, &mut save, &mut next);
    }
}

pub fn intro_title_music(
    state: Res<State<AppState>>,
    mut sequence: Option<ResMut<IntroSequence>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Option<Res<crate::audio::GameAudio>>,
    manifest: Option<Res<crate::audio::AudioManifest>>,
    mut bgm: Option<ResMut<crate::audio::BgmState>>,
    save: Option<Res<GameSave>>,
    gate: Option<Res<AudioGate>>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    let Some(mut sequence) = sequence else {
        return;
    };
    if sequence.music_started || sequence.phase < IntroPhase::TitleIn {
        return;
    }
    let (Some(audio), Some(manifest), Some(mut bgm), Some(save), Some(gate)) =
        (audio, manifest, bgm, save, gate)
    else {
        return;
    };
    sequence.music_started = true;
    play_menu_bgm_now(
        &mut commands,
        &asset_server,
        &audio,
        &manifest,
        &mut bgm,
        &save,
        &gate,
    );
}

pub fn intro_title_fly(
    state: Res<State<AppState>>,
    sequence: Option<Res<IntroSequence>>,
    layout: Option<Res<IntroLayout>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut titles: Query<(&mut Transform, &mut TextColor), With<IntroTitle>>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    let Some(sequence) = sequence else {
        return;
    };
    if sequence.phase != IntroPhase::TitleFly {
        return;
    }
    let ui_scale = layout
        .map(|l| l.ui_scale)
        .or_else(|| window.get_single().ok().map(viewport::ui_scale))
        .unwrap_or(1.0);
    let t = sequence.timer.fraction().clamp(0.0, 1.0);
    let lift = t * ui_scale * 420.0;
    let alpha = 1.0 - t;
    for (mut tf, mut color) in &mut titles {
        tf.translation.y = lift;
        color.0 = color.0.with_alpha(alpha);
    }
}

fn finish_intro(backend: &SaveBackend, save: &mut GameSave, next: &mut NextState<AppState>) {
    save.0.profile.intro_seen = true;
    persist_save(backend, &save.0);
    next.set(AppState::MainMenu);
}

pub fn intro_skip_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<AppState>>,
    mut sequence: Option<ResMut<IntroSequence>>,
    mut gate: ResMut<AudioGate>,
    mut save: ResMut<GameSave>,
    backend: Res<SaveBackend>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Option<Res<crate::audio::GameAudio>>,
    manifest: Option<Res<crate::audio::AudioManifest>>,
    mut bgm: Option<ResMut<crate::audio::BgmState>>,
) {
    if *state.get() != AppState::Intro {
        return;
    }
    let Some(mut sequence) = sequence else {
        return;
    };
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        gate.unlocked = true;
        if !sequence.music_started {
            if let (Some(audio), Some(manifest), Some(mut bgm)) = (audio, manifest, bgm) {
                sequence.music_started = true;
                play_menu_bgm_now(
                    &mut commands,
                    &asset_server,
                    &audio,
                    &manifest,
                    &mut bgm,
                    &save,
                    &gate,
                );
            }
        }
        sequence.skipped = true;
        sequence.phase = IntroPhase::Done;
        finish_intro(&backend, &mut save, &mut next);
    }
}

pub fn cleanup_intro(mut commands: Commands, q: Query<Entity, With<IntroVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<IntroSequence>();
    commands.remove_resource::<IntroLayout>();
    commands.remove_resource::<IntroSpawned>();
}
