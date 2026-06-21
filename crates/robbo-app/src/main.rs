mod app_state;
mod assets;
mod bridge;
mod camera;
mod editor;
mod input;
mod interpolation;
mod iso;
mod levels;
mod persistence;
mod pool;
mod projection;
mod render;
mod ui;

use app_state::AppState;
use assets::SpriteAssets;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bridge::{CoreBridge, EntityMap, GameSession, GameTickTimer, LoadLevelEvent, ReloadVisualsEvent};
use levels::{LevelRegistry, LevelSelection};
use persistence::{GameSave, load_save};
use projection::ActiveProjection;
use ui::SpeedrunTimer;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(LogPlugin {
                filter: "robbo=debug,bevy_render=warn,wgpu=warn".into(),
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
            }),
        )
        .init_state::<AppState>()
        .insert_resource(CoreBridge::default())
        .insert_resource(ActiveProjection::default())
        .insert_resource(GameSession::default())
        .insert_resource(EntityMap::default())
        .insert_resource(GameTickTimer::default())
        .insert_resource(LevelRegistry::load_builtin())
        .insert_resource(LevelSelection::default())
        .insert_resource(GameSave(load_save()))
        .insert_resource(SpeedrunTimer::default())
        .insert_resource(editor::EditorState::default())
        .add_event::<bridge::CoreGameEvent>()
        .add_event::<LoadLevelEvent>()
        .add_event::<ReloadVisualsEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                input::keyboard_input,
                bridge::buffer_input_while_animating,
                bridge::game_tick_system,
                bridge::release_queued_input,
                render::sync_visuals,            // must run before advance so VisualMotion.from/to are set before progress advances
                interpolation::advance_interpolation_system,
                render::update_entity_transforms,
                render::rebuild_level_visuals,
                camera::update_camera,
                ui::tick_speedrun_timer,
                ui::update_hud,
                ui::update_overlay_text,
                ui::on_core_events,
                editor::editor_toggle,
                iso::toggle_isometric,
            )
                .chain(),
        )
        .add_systems(OnEnter(AppState::Playing), ui::spawn_playing_hud)
        .add_systems(OnExit(AppState::Playing), ui::cleanup_hud)
        .add_systems(
            OnEnter(AppState::MainMenu),
            (ui::spawn_menu_overlay, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::MainMenu), ui::cleanup_overlay)
        .add_systems(
            OnEnter(AppState::LevelSelect),
            (ui::spawn_menu_overlay, log_state::<AppState>),
        )
        .add_systems(OnExit(AppState::LevelSelect), ui::cleanup_overlay)
        .add_systems(OnEnter(AppState::Paused), ui::spawn_menu_overlay)
        .add_systems(OnExit(AppState::Paused), ui::cleanup_overlay)
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
        .add_systems(Update, ui::load_level_system)
        .run();
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
