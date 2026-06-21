use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, EntityMap, LoadLevelEvent};
use crate::interpolation::VisualEntityId;
use crate::projection::ActiveProjection;

const CONFIG_STR: &str = include_str!("../../../assets/camera.ron");

#[derive(Debug, Clone, Deserialize)]
pub struct ViewportTiles {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CameraConfigData {
    pub viewport_tiles: ViewportTiles,
    pub zoom_step: f32,
    pub max_zoom_in: u32,
    pub max_zoom_out: u32,
    pub follow_lerp: f32,
    pub zoom_lerp: f32,
    pub margin: f32,
}

impl Default for CameraConfigData {
    fn default() -> Self {
        Self {
            viewport_tiles: ViewportTiles {
                width: 12,
                height: 16,
            },
            zoom_step: 1.15,
            max_zoom_in: 6,
            max_zoom_out: 4,
            follow_lerp: 12.0,
            zoom_lerp: 10.0,
            margin: 1.05,
        }
    }
}

#[derive(Resource, Clone)]
pub struct CameraConfig(pub CameraConfigData);

impl CameraConfig {
    pub fn load() -> Self {
        let data: CameraConfigData =
            ron::from_str(CONFIG_STR).unwrap_or_else(|e| {
                bevy::log::warn!("camera.ron parse failed ({e}); using defaults");
                CameraConfigData::default()
            });
        Self(data)
    }
}

#[derive(Resource, Default)]
pub struct CameraState {
    pub zoom_level: i32,
    pub smoothed_zoom_level: f32,
    pub smoothed_pos: Vec2,
    pub needs_snap: bool,
}

#[derive(Component, Clone, Copy)]
pub enum ZoomDirection {
    In,
    Out,
}

#[derive(Component)]
pub struct ZoomButton(pub ZoomDirection);

pub fn load_camera_config(mut commands: Commands) {
    commands.insert_resource(CameraConfig::load());
    commands.insert_resource(CameraState::default());
}

pub fn reset_camera_on_level_load(
    mut load_events: EventReader<LoadLevelEvent>,
    mut state: ResMut<CameraState>,
) {
    if load_events.read().next().is_some() {
        state.zoom_level = 0;
        state.smoothed_zoom_level = 0.0;
        state.needs_snap = true;
    }
}

pub fn zoom_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    config: Res<CameraConfig>,
    mut cam_state: ResMut<CameraState>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        apply_zoom(&mut cam_state, &config, 1);
    } else if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        apply_zoom(&mut cam_state, &config, -1);
    }
}

pub fn handle_zoom_buttons(
    state: Res<State<AppState>>,
    config: Res<CameraConfig>,
    mut cam_state: ResMut<CameraState>,
    mut interactions: Query<
        (&Interaction, &ZoomButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    for (interaction, button, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                let delta = match button.0 {
                    ZoomDirection::In => 1,
                    ZoomDirection::Out => -1,
                };
                apply_zoom(&mut cam_state, &config, delta);
                bg.0 = Color::srgba(0.35, 0.45, 0.65, 0.95);
            }
            Interaction::Hovered => {
                bg.0 = Color::srgba(0.25, 0.32, 0.48, 0.95);
            }
            Interaction::None => {
                bg.0 = Color::srgba(0.15, 0.18, 0.28, 0.9);
            }
        }
    }
}

fn apply_zoom(state: &mut CameraState, config: &CameraConfig, delta: i32) {
    let cfg = &config.0;
    let min_level = -(cfg.max_zoom_out as i32);
    let max_level = cfg.max_zoom_in as i32;
    state.zoom_level = (state.zoom_level + delta).clamp(min_level, max_level);
}

pub fn update_camera(
    time: Res<Time>,
    state: Res<State<AppState>>,
    config: Res<CameraConfig>,
    mut cam_state: ResMut<CameraState>,
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    entity_map: Res<EntityMap>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut transforms: ParamSet<(
        Query<&Transform, With<VisualEntityId>>,
        Query<&mut Transform, With<Camera2d>>,
    )>,
) {
    if *state.get() != AppState::Playing {
        return;
    }

    let Ok(window) = window.get_single() else {
        return;
    };

    let tile = projection.tile_size();
    let cfg = &config.0;
    let dt = time.delta_secs();

    let target_pos = {
        let robbo_q = transforms.p0();
        robbo_world_pos(&bridge, &entity_map, &robbo_q)
            .unwrap_or_else(|| level_center(&bridge, tile))
    };

    if cam_state.needs_snap {
        cam_state.smoothed_pos = target_pos;
        cam_state.smoothed_zoom_level = cam_state.zoom_level as f32;
        cam_state.needs_snap = false;
    }

    let pos_alpha = smooth_alpha(cfg.follow_lerp, dt);
    cam_state.smoothed_pos = cam_state.smoothed_pos.lerp(target_pos, pos_alpha);

    let zoom_alpha = smooth_alpha(cfg.zoom_lerp, dt);
    let target_zoom = cam_state.zoom_level as f32;
    cam_state.smoothed_zoom_level = cam_state
        .smoothed_zoom_level
        .lerp(target_zoom, zoom_alpha);

    let base_scale = base_scale_for_viewport(
        cfg.viewport_tiles.width,
        cfg.viewport_tiles.height,
        tile,
        window.width(),
        window.height(),
        cfg.margin,
    );
    let zoom_factor = cfg.zoom_step.powf(cam_state.smoothed_zoom_level);
    let scale = (base_scale / zoom_factor).max(0.02);

    let clamped = clamp_camera_pos(
        cam_state.smoothed_pos,
        scale,
        window.width(),
        window.height(),
        bridge.world.width as f32 * tile,
        bridge.world.height as f32 * tile,
    );

    let mut camera_q = transforms.p1();
    let Ok(mut transform) = camera_q.get_single_mut() else {
        return;
    };
    transform.translation.x = clamped.x;
    transform.translation.y = clamped.y;
    transform.scale = Vec3::splat(scale);
}

fn robbo_world_pos(
    bridge: &CoreBridge,
    entity_map: &EntityMap,
    transforms: &Query<&Transform, With<VisualEntityId>>,
) -> Option<Vec2> {
    let robbo_id = bridge.world.robbo_id;
    if robbo_id == 0 {
        return None;
    }
    let entity = entity_map.0.get(&robbo_id)?;
    let transform = transforms.get(*entity).ok()?;
    Some(transform.translation.truncate())
}

fn level_center(bridge: &CoreBridge, tile: f32) -> Vec2 {
    let w = bridge.world.width as f32;
    let h = bridge.world.height as f32;
    Vec2::new(w * tile * 0.5, -h * tile * 0.5)
}

fn base_scale_for_viewport(
    tiles_w: u32,
    tiles_h: u32,
    tile: f32,
    window_w: f32,
    window_h: f32,
    margin: f32,
) -> f32 {
    let view_w = tiles_w as f32 * tile;
    let view_h = tiles_h as f32 * tile;
    let scale_x = view_w / window_w.max(1.0);
    let scale_y = view_h / window_h.max(1.0);
    scale_x.max(scale_y) * margin
}

fn clamp_camera_pos(
    pos: Vec2,
    scale: f32,
    window_w: f32,
    window_h: f32,
    level_w: f32,
    level_h: f32,
) -> Vec2 {
    let half_w = window_w * scale * 0.5;
    let half_h = window_h * scale * 0.5;

    let x = if level_w <= half_w * 2.0 {
        level_w * 0.5
    } else {
        pos.x.clamp(half_w, level_w - half_w)
    };

    let y = if level_h <= half_h * 2.0 {
        -level_h * 0.5
    } else {
        pos.y.clamp(-level_h + half_h, -half_h)
    };

    Vec2::new(x, y)
}

fn smooth_alpha(speed: f32, dt: f32) -> f32 {
    (1.0 - (-speed * dt).exp()).clamp(0.0, 1.0)
}
