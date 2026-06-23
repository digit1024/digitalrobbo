//! Responsive layout for main-menu world sprites (background cover, planet bottom).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::viewport::{self, DESIGN_HEIGHT, DESIGN_WIDTH};

pub const PLANET_TEX_H: f32 = 768.0;
/// Nudge planet slightly below viewport edge (fraction of scaled height).
const PLANET_BOTTOM_INSET: f32 = 0.1;

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct MainMenuLayout {
    pub window_w: f32,
    pub window_h: f32,
    pub cover: f32,
    pub planet_target_y: f32,
    pub planet_start_y: f32,
}

impl MainMenuLayout {
    pub fn from_window(window: &Window) -> Self {
        let window_w = window.width();
        let window_h = window.height();
        let cover = viewport::cover_scale(window, DESIGN_WIDTH, DESIGN_HEIGHT);
        let bottom = viewport::world_bottom_y(window);
        let half_h = PLANET_TEX_H * cover * 0.5;
        let planet_target_y = bottom + half_h - PLANET_TEX_H * cover * PLANET_BOTTOM_INSET;
        let planet_start_y = planet_target_y - PLANET_TEX_H * cover * 1.1;
        Self {
            window_w,
            window_h,
            cover,
            planet_target_y,
            planet_start_y,
        }
    }
}

pub fn track_main_menu_layout(
    state: Res<State<crate::app_state::AppState>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut layout: Option<ResMut<MainMenuLayout>>,
    mut dirty: Option<ResMut<crate::menu::state::MainMenuUiDirty>>,
) {
    if *state.get() != crate::app_state::AppState::MainMenu {
        return;
    }
    let Ok(window) = window.single() else {
        return;
    };
    let next = MainMenuLayout::from_window(window);
    let Some(mut layout) = layout else {
        return;
    };
    if layout.window_w != next.window_w || layout.window_h != next.window_h {
        *layout = next;
        if let Some(mut dirty) = dirty {
            dirty.0 = true;
        }
    }
}
