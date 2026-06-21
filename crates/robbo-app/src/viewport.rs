use bevy::prelude::*;

/// Original digit1024 / space.png design resolution.
pub const DESIGN_WIDTH: f32 = 1280.0;
pub const DESIGN_HEIGHT: f32 = 768.0;

/// Scale factor to cover the window with a background texture (no letterbox gaps).
pub fn cover_scale(window: &Window, tex_w: f32, tex_h: f32) -> f32 {
    let ww = window.width();
    let wh = window.height();
    (ww / tex_w).max(wh / tex_h)
}

/// Scale UI font sizes relative to the design height.
pub fn ui_scale(window: &Window) -> f32 {
    (window.height() / DESIGN_HEIGHT).clamp(0.75, 1.5)
}

pub fn cover_transform(window: &Window, tex_w: f32, tex_h: f32) -> Transform {
    let s = cover_scale(window, tex_w, tex_h);
    Transform::from_scale(Vec3::new(s, s, 1.0))
}
