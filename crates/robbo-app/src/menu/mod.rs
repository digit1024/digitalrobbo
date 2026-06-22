//! Main menu scene.
//!
//! # Lifecycle (see `lib.rs` state transitions)
//!
//! ```text
//! OnEnter(MainMenu):
//!   1. cleanup_level_hud
//!   2. render::teardown_level_scene   — drop level sprites / FX
//!   3. render::reset_sim_on_menu      — reset bridge, session, timers
//!   4. spawn_main_menu                — build menu-owned entities
//!   5. play_menu_bgm
//!
//! OnExit(MainMenu):
//!   1. cleanup_main_menu              — despawn every [`MainMenuOwned`] entity
//!   2. stop_bgm
//! ```
//!
//! Every entity created for the main menu (world sprites, UI nodes, helpers) must
//! carry [`MainMenuOwned`] so teardown is a single query — no leaked sprites when
//! switching scenes.

mod input;
mod layout;
mod spawn;
mod state;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub use layout::{track_main_menu_layout, MainMenuLayout};
pub use state::{MainMenuScreen, MainMenuState, MainMenuUiDirty};

use spawn::init_on_enter;

/// Tag for all entities owned by the main menu scene.
#[derive(Component)]
pub struct MainMenuOwned;

pub fn spawn_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    font: Res<crate::persistence::GameFont>,
    save: Res<crate::persistence::GameSave>,
    mut menu_state: ResMut<MainMenuState>,
    mut dirty: ResMut<MainMenuUiDirty>,
    existing: Query<Entity, With<MainMenuOwned>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    despawn_all(&mut commands, &existing);
    let Ok(window) = window.get_single() else {
        return;
    };
    init_on_enter(
        &mut commands,
        &asset_server,
        &font,
        &save,
        window,
        &mut menu_state,
        &mut dirty,
    );
}

pub fn cleanup_main_menu(
    mut commands: Commands,
    owned: Query<Entity, With<MainMenuOwned>>,
) {
    despawn_all(&mut commands, &owned);
    commands.remove_resource::<MainMenuLayout>();
}

fn despawn_all(commands: &mut Commands, owned: &Query<Entity, With<MainMenuOwned>>) {
    for entity in owned.iter().collect::<Vec<_>>() {
        commands.entity(entity).despawn_recursive();
    }
}

pub use input::{
    animate_planet, keyboard_input, pointer_input, refresh_ui_if_dirty, update_highlight,
    update_level_select_labels_system, update_settings_labels,
};
