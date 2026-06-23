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
#[cfg(target_os = "android")]
mod touch_controls;
mod ui;
mod ui_anim;
mod viewport;

pub mod test_harness;

use app_state::AppState;
use assets::SpriteAssets;
use audio::{
    cleanup_enemy_ambient_sounds, ensure_menu_bgm, load_audio_manifest, play_menu_bgm,
    queue_level_bgm_on_load,
    resume_bgm_on_unpause, sfx_on_core_events, start_level_bgm_after_countdown, stop_bgm,
    unlock_audio_on_input, update_enemy_ambient_sounds,
};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::{WakeUp, WinitPlugin};
#[cfg(target_os = "android")]
use bevy::winit::WinitSettings;
use bridge::{CoreBridge, EntityMap, GameSession, GameTickTimer, LoadLevelEvent, ReloadVisualsEvent, TileEntityMap};
use effects::{
    fx_on_core_events, reset_magnet_beams_on_reload, sync_fx_auras, tick_collect_pop_effects,
    tick_fx_particles, tick_teleport_auras, update_capsule_visuals, update_magnet_beams,
    update_magnet_visuals, update_projectile_visuals, update_screw_visuals,
    update_butterfly_visuals, MagnetBeams,
};
use game_menus::{
    cleanup_game_menu, game_menu_button_input, spawn_game_menu, update_level_select_menu,
};
use hud::{cleanup_level_hud, hud_button_input, spawn_level_hud, update_level_hud};
use input::{apply_test_input, InputCooldown, SteeringState, TestInputInject};
use levels::{LevelRegistry, LevelSelection};
use menu::{
    animate_planet, cleanup_main_menu, keyboard_input as main_menu_keyboard,
    pointer_input as main_menu_pointer, refresh_ui_if_dirty, spawn_main_menu,
    track_main_menu_layout, update_highlight,
    update_level_select_labels_system, update_settings_labels, MainMenuState, MainMenuUiDirty,
};
use persistence::{load_game_font, GameSave, SaveBackend, load_save};
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

/// Mobile entry point (`android_main` / iOS); desktop uses `src/main.rs`.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[bevy_main]
fn main() {
    build_app().run();
}

fn asset_root() -> String {
    #[cfg(target_os = "android")]
    {
        return String::new();
    }
    #[cfg(all(target_arch = "wasm32", not(target_os = "android")))]
    {
        return concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_string();
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_string()
    }
}

fn configure_app(app: &mut App, allow_any_thread: bool) {
    let save_backend = SaveBackend::platform_default();
    let game_save = GameSave(load_save(&save_backend));

    let mut plugins = DefaultPlugins
        .set(LogPlugin {
            filter: "robbo=info,bevy_render=warn,wgpu=warn".into(),
            ..default()
        })
        .set(AssetPlugin {
            file_path: asset_root(),
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
    app.add_plugins(plugins);
    #[cfg(target_os = "android")]
    app.insert_resource(WinitSettings::mobile());
    app.init_state::<AppState>()
        .insert_resource(CoreBridge::default())
        .insert_resource(ActiveProjection::default())
        .insert_resource(GameSession::default())
        .insert_resource(EntityMap::default())
        .insert_resource(TileEntityMap::default())
        .insert_resource(GameTickTimer::default())
        .insert_resource(LevelRegistry::load_builtin())
        .insert_resource(LevelSelection::default())
        .insert_resource(save_backend)
        .insert_resource(game_save)
        .insert_resource(SpeedrunTimer::default())
        .insert_resource(LevelCountdown::default())
        .insert_resource(MainMenuState::default())
        .insert_resource(MainMenuUiDirty::default())
        .insert_resource(InputCooldown::default())
        .insert_resource(SteeringState::default())
        .insert_resource(TestInputInject::default())
        .insert_resource(editor::EditorState::default())
        .init_resource::<bridge::RenderAudit>()
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
                ensure_menu_bgm.after(unlock_audio_on_input),
                ui_anim::tick_ui_fade,
                hud_button_input,
                game_menu_button_input,
                editor::editor_toggle,
                iso::toggle_isometric,
                camera::zoom_keyboard_input,
                camera::handle_zoom_buttons,
            ),
        )
        .add_systems(
            Update,
            (
                main_menu_keyboard,
                main_menu_pointer,
                animate_planet,
                update_highlight,
                update_settings_labels,
                update_level_select_labels_system,
                refresh_ui_if_dirty
                    .after(main_menu_keyboard)
                    .after(main_menu_pointer),
                track_main_menu_layout,
            ),
        )
        .add_systems(
            Update,
            update_butterfly_visuals.after(render::update_bear_visuals),
        )
        .add_systems(
            Update,
            update_projectile_visuals.after(render::update_entity_transforms),
        )
        .add_systems(
            OnEnter(AppState::Playing),
            (cleanup_level_hud, spawn_level_hud).chain(),
        )
        .add_systems(
            OnExit(AppState::Playing),
            (cleanup_level_hud, ui::cleanup_countdown_overlay, stop_bgm, cleanup_enemy_ambient_sounds),
        )
        .add_systems(
            OnEnter(AppState::MainMenu),
            (
                cleanup_level_hud,
                render::teardown_level_scene,
                render::reset_sim_on_menu,
                camera::reset_camera_for_menu,
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
                cleanup_level_hud,
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
    #[cfg(target_os = "android")]
    {
        app.add_systems(OnEnter(AppState::Playing), touch_controls::spawn_touch_controls)
            .add_systems(OnExit(AppState::Playing), touch_controls::cleanup_touch_controls)
            .add_systems(
                Update,
                touch_controls::donut_pad_touch.run_if(in_state(AppState::Playing)),
            );
    }
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
