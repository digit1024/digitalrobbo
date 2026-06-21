use bevy::prelude::*;

/// DigitalRobbo menu / UI design resolution (matches `space.png` aspect).
pub const DESIGN_WIDTH: f32 = 1280.0;
pub const DESIGN_HEIGHT: f32 = 768.0;

/// Original cocos2d digit1024 logical resolution (`AppMacros.h`).
pub const ORIGINAL_WIDTH: f32 = 480.0;
pub const ORIGINAL_HEIGHT: f32 = 320.0;

/// Convert cocos bottom-left pixel coords to design space (top-left origin, Y down).
pub fn cocos_to_design(x: f32, y: f32) -> Vec2 {
    Vec2::new(
        (x / ORIGINAL_WIDTH) * DESIGN_WIDTH,
        ((ORIGINAL_HEIGHT - y) / ORIGINAL_HEIGHT) * DESIGN_HEIGHT,
    )
}

/// Uniform scale from design pixels to world units (Camera2d center origin, 1 unit ≈ 1 px).
pub fn world_scale(window: &Window) -> f32 {
    window.height() / DESIGN_HEIGHT
}

/// Map a design-space point (top-left origin) to world translation (center origin, Y up).
pub fn design_to_world(pos: Vec2, window: &Window) -> Vec2 {
    let s = world_scale(window);
    Vec2::new(
        (pos.x - DESIGN_WIDTH / 2.0) * s,
        (DESIGN_HEIGHT / 2.0 - pos.y) * s,
    )
}

/// Design-space center (equivalent to cocos `visibleSize / 2`).
pub fn design_center() -> Vec2 {
    Vec2::new(DESIGN_WIDTH / 2.0, DESIGN_HEIGHT / 2.0)
}

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

/// Horizontal shake offset (matches original `CCShake` feel).
pub fn shake_offset(elapsed: f32, amplitude: f32) -> Vec2 {
    Vec2::new(
        (elapsed * 47.0).sin() * amplitude + (elapsed * 31.0).cos() * amplitude * 0.5,
        (elapsed * 43.0).cos() * amplitude * 0.3,
    )
}
