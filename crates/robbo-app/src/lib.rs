pub mod app_state;
mod assets;
mod audio;
mod bridge;
mod camera;
mod editor;
mod effects;
mod game_menus;
mod hud;
mod input;
mod interpolation;
#[allow(dead_code)]
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
    cleanup_enemy_ambient_sounds, load_audio_manifest, play_menu_bgm, queue_level_bgm_on_load,
    resume_bgm_on_unpause, sfx_on_core_events, start_level_bgm_after_countdown, stop_bgm,
    unlock_audio_on_input, update_enemy_ambient_sounds,
};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::{WakeUp, WinitPlugin};
use bridge::{CoreBridge, EntityMap, GameSession, GameTickTimer, LoadLevelEvent, ReloadVisualsEvent, TileEntityMap};
use effects::{
    fx_on_core_events, reset_magnet_beams_on_reload, sync_fx_auras, tick_collect_pop_effects,
    tick_fx_particles, tick_teleport_auras, update_capsule_visuals, update_magnet_beams,
    update_magnet_visuals, update_projectile_visuals, update_screw_visuals, MagnetBeams,
};
use game_menus::{
    cleanup_game_menu, game_menu_button_input, spawn_game_menu, update_level_select_menu,
};
use hud::{cleanup_level_hud, hud_button_input, spawn_level_hud, update_level_hud};
use input::{apply_test_input, InputCooldown, SteeringState, TestInputInject};
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
        .init_resource::<MagnetBeams>()
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
                update_capsule_visuals,
                camera::reset_camera_on_level_load,
                camera::update_camera,
                ui::tick_speedrun_timer,
                ui::tick_level_countdown,
                update_level_hud,
                update_level_select_menu,
                ui::on_core_events,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                sfx_on_core_events,
                update_enemy_ambient_sounds,
                fx_on_core_events,
                queue_level_bgm_on_load,
                start_level_bgm_after_countdown,
                unlock_audio_on_input,
                ui_anim::tick_ui_fade,
                hud_button_input,
                game_menu_button_input,
                animate_menu_planet,
                update_menu_highlight,
                menu_navigate,
                editor::editor_toggle,
                iso::toggle_isometric,
                camera::zoom_keyboard_input,
                camera::handle_zoom_buttons,
            ),
        )
        .add_systems(
            Update,
            update_projectile_visuals.after(render::update_entity_transforms),
        )
        .add_systems(OnEnter(AppState::Playing), spawn_level_hud)
        .add_systems(
            OnExit(AppState::Playing),
            (cleanup_level_hud, ui::cleanup_countdown_overlay, stop_bgm, cleanup_enemy_ambient_sounds),
        )
        .add_systems(
            OnEnter(AppState::MainMenu),
            (
                render::teardown_level_scene,
                render::reset_sim_on_menu,
                spawn_main_menu,
                play_menu_bgm,
                log_state::<AppState>,
            )
                .chain(),
        )
        .add_systems(OnExit(AppState::MainMenu), (cleanup_main_menu, stop_bgm))
        .add_systems(
            OnEnter(AppState::LevelSelect),
            (
                render::teardown_level_scene,
                render::reset_sim_on_menu,
                spawn_game_menu,
                stop_bgm,
                log_state::<AppState>,
            )
                .chain(),
        )
        .add_systems(OnExit(AppState::LevelSelect), cleanup_game_menu)
        .add_systems(
            OnEnter(AppState::Paused),
            (spawn_game_menu, stop_bgm, cleanup_enemy_ambient_sounds),
        )
        .add_systems(
            OnExit(AppState::Paused),
            (cleanup_game_menu, resume_bgm_on_unpause),
        )
        .add_systems(
            OnEnter(AppState::LevelComplete),
            (
                spawn_game_menu,
                stop_bgm,
                cleanup_enemy_ambient_sounds,
                log_state::<AppState>,
            ),
        )
        .add_systems(OnExit(AppState::LevelComplete), cleanup_game_menu)
        .add_systems(
            OnEnter(AppState::GameOver),
            (
                spawn_game_menu,
                stop_bgm,
                cleanup_enemy_ambient_sounds,
                log_state::<AppState>,
            ),
        )
        .add_systems(OnExit(AppState::GameOver), cleanup_game_menu)
        .add_systems(
            Update,
            (
                reset_magnet_beams_on_reload,
                render::update_robbo_sprite,
                render::update_entity_sprites,
                update_magnet_visuals,
                tick_fx_particles,
                tick_teleport_auras,
                update_magnet_beams,
                tick_collect_pop_effects,
                render::update_explosion_effects,
                render::update_tile_vanish_effects,
                render::update_teleport_reveal,
            )
                .after(bridge::game_tick_system)
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
    next.set(AppState::MainMenu);
}

fn log_state<S: States>(state: Res<State<S>>) {
    bevy::log::info!("Entered state: {:?}", state.get());
}
