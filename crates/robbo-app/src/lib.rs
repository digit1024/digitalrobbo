pub mod app_state;
mod assets;
mod audio;
mod bridge;
mod camera;
mod editor;
mod effects;
mod input;
mod interpolation;
mod intro;
mod iso;
mod levels;
mod menu;
mod persistence;
mod pool;
mod projection;
mod render;
mod ui;
mod ui_anim;
mod viewport;

pub mod test_harness;

use app_state::AppState;
use assets::SpriteAssets;
use audio::{
    load_audio_manifest, play_menu_bgm, queue_level_bgm_on_load, resume_bgm_on_unpause,
    sfx_on_core_events, start_level_bgm_after_countdown, stop_bgm, unlock_audio_on_input,
};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::{WakeUp, WinitPlugin};
use bridge::{CoreBridge, EntityMap, GameSession, GameTickTimer, LoadLevelEvent, ReloadVisualsEvent, TileEntityMap};
use effects::{
    fx_on_core_events, sync_fx_auras, tick_collect_pop_effects, tick_fx_particles,
    tick_teleport_auras, update_screw_visuals,
};
use input::{apply_test_input, InputCooldown, SteeringState, TestInputInject};
use intro::{setup_intro, spawn_intro, start_intro_audio};
use levels::{LevelRegistry, LevelSelection};
use menu::{
    animate_menu_planet, cleanup_main_menu, load_game_font, menu_navigate, spawn_main_menu,
    update_menu_highlight, MenuSelection,
};
use persistence::{GameSave, load_save};
use projection::ActiveProjection;
use ui::{LevelCountdown, SpeedrunTimer};

/// Build the full game app (used by the binary).
pub fn build_app() -> App {
    let mut app = App::new();
    configure_app(&mut app, false);
    app
}

/// Same as [`build_app`] but allows winit on test worker threads (Linux/Windows).
pub fn build_test_app() -> App {
    let mut app = App::new();
    configure_app(&mut app, true);
    app
}

fn configure_app(app: &mut App, allow_any_thread: bool) {
    let mut plugins = DefaultPlugins
        .set(LogPlugin {
            filter: "robbo=info,bevy_render=warn,wgpu=warn".into(),
            ..default()
        })
        .set(AssetPlugin {
            file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_string(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "DigitalRobbo".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        });
    if allow_any_thread {
        let mut winit = WinitPlugin::<WakeUp>::default();
        winit.run_on_any_thread = true;
        plugins = plugins.set(winit);
    }
    app.add_plugins(plugins)
        .init_state::<AppState>()
        .insert_resource(CoreBridge::default())
        .insert_resource(ActiveProjection::default())
        .insert_resource(GameSession::default())
        .insert_resource(EntityMap::default())
        .insert_resource(TileEntityMap::default())
        .insert_resource(GameTickTimer::default())
        .insert_resource(LevelRegistry::load_builtin())
        .insert_resource(LevelSelection::default())
        .insert_resource(GameSave(load_save()))
        .insert_resource(SpeedrunTimer::default())
        .insert_resource(LevelCountdown::default())
        .insert_resource(MenuSelection::default())
        .insert_resource(InputCooldown::default())
        .insert_resource(SteeringState::default())
        .insert_resource(TestInputInject::default())
        .insert_resource(editor::EditorState::default())
        .add_event::<bridge::CoreGameEvent>()
        .add_event::<LoadLevelEvent>()
        .add_event::<ReloadVisualsEvent>()
        .add_systems(
            Startup,
            (
                setup,
                load_audio_manifest,
                camera::load_camera_config,
                load_game_font,
                setup_intro,
            ),
        )
        .add_systems(
            Update,
            (
                apply_test_input,
                input::keyboard_input,
                input::tick_input_cooldown,
                bridge::game_tick_system,
                render::sync_visuals,
                interpolation::advance_interpolation_system,
                ui::load_level_system,
                render::rebuild_level_visuals,
                sync_fx_auras,
                render::update_entity_transforms,
                update_screw_visuals,
                render::update_bear_visuals,
                camera::reset_camera_on_level_load,
                camera::update_camera,
                ui::tick_speedrun_timer,
                ui::tick_level_countdown,
                ui::update_hud,
                ui::update_overlay_text,
                ui::on_core_events,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                sfx_on_core_events,
                fx_on_core_events,
                queue_level_bgm_on_load,
                start_level_bgm_after_countdown,
                unlock_audio_on_input,
                ui_anim::tick_ui_fade,
                intro::spawn_intro,
                intro::intro_title_fly,
                intro::update_intro_sequence,
                intro::intro_title_music,
                intro::intro_skip_input,
                animate_menu_planet,
                update_menu_highlight,
                menu_navigate,
                editor::editor_toggle,
                iso::toggle_isometric,
                camera::zoom_keyboard_input,
                camera::handle_zoom_buttons,
            ),
        )
        .add_systems(OnEnter(AppState::Playing), ui::spawn_playing_hud)
        .add_systems(
            OnExit(AppState::Playing),
            (ui::cleanup_hud, ui::cleanup_countdown_overlay),
        )
        .add_systems(
            OnEnter(AppState::Intro),
            (start_intro_audio, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::Intro), intro::cleanup_intro)
        .add_systems(
            OnEnter(AppState::MainMenu),
            (spawn_main_menu, play_menu_bgm, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu)
        .add_systems(
            OnEnter(AppState::LevelSelect),
            (ui::spawn_menu_overlay, stop_bgm, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::LevelSelect), ui::cleanup_overlay)
        .add_systems(OnEnter(AppState::Paused), (ui::spawn_menu_overlay, stop_bgm))
        .add_systems(
            OnExit(AppState::Paused),
            (ui::cleanup_overlay, resume_bgm_on_unpause),
        )
        .add_systems(
            OnEnter(AppState::LevelComplete),
            (ui::spawn_menu_overlay, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::LevelComplete), ui::cleanup_overlay)
        .add_systems(
            OnEnter(AppState::GameOver),
            (ui::spawn_menu_overlay, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::GameOver), ui::cleanup_overlay)
        .add_systems(
            Update,
            (
                render::update_robbo_sprite,
                render::update_entity_sprites,
                tick_fx_particles,
                tick_teleport_auras,
                tick_collect_pop_effects,
                render::update_explosion_effects,
                render::update_tile_vanish_effects,
                render::update_teleport_reveal,
            )
                .after(render::sync_visuals)
                .before(interpolation::advance_interpolation_system),
        );
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next: ResMut<NextState<AppState>>,
) {
    commands.spawn(Camera2d::default());
    commands.insert_resource(SpriteAssets::load(&asset_server));
    bevy::log::info!("DigitalRobbo starting");
    next.set(AppState::Intro);
}

fn log_state<S: States>(state: Res<State<S>>) {
    bevy::log::info!("Entered state: {:?}", state.get());
}
