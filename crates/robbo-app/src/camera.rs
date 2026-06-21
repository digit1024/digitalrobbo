use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::bridge::CoreBridge;
use crate::projection::ActiveProjection;

pub fn update_camera(
    bridge: Res<CoreBridge>,
    projection: Res<ActiveProjection>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(mut transform) = camera.get_single_mut() else {
        return;
    };
    let tile = projection.tile_size();
    let w = bridge.world.width as f32;
    let h = bridge.world.height as f32;

    transform.translation.x = (w - 1.0).max(0.0) * tile * 0.5;
    transform.translation.y = -(h - 1.0).max(0.0) * tile * 0.5;

    if let Ok(window) = window.get_single() {
        let level_w = w * tile;
        let level_h = h * tile;
        // For a 2D camera, Transform.scale is world-units-per-pixel: a larger
        // scale zooms OUT. To make the level fill the window we want the larger
        // of the two axis ratios, plus a small margin so edges aren't clipped.
        let scale_x = level_w / window.width().max(1.0);
        let scale_y = level_h / window.height().max(1.0);
        let scale = (scale_x.max(scale_y) * 1.08).max(0.05);
        transform.scale = Vec3::splat(scale);
    }
}
