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
    design_to_world_scaled(pos, cover_scale(window, DESIGN_WIDTH, DESIGN_HEIGHT))
}

/// Same as [`design_to_world`] but with an explicit uniform scale (e.g. background cover).
pub fn design_to_world_scaled(pos: Vec2, scale: f32) -> Vec2 {
    Vec2::new(
        (pos.x - DESIGN_WIDTH / 2.0) * scale,
        (DESIGN_HEIGHT / 2.0 - pos.y) * scale,
    )
}

/// Bottom edge of the viewport in world units (Camera2d, center origin).
pub fn world_bottom_y(window: &Window) -> f32 {
    -window.height() * 0.5
}

/// Map design pixel coords to screen-space UI offsets (top-left origin).
pub fn design_to_screen(pos: Vec2, window: &Window) -> Vec2 {
    Vec2::new(
        pos.x * window.width() / DESIGN_WIDTH,
        pos.y * window.height() / DESIGN_HEIGHT,
    )
}

/// Pause / main menu item label size (`game_menus` pause panel).
pub fn menu_item_font_size(window: &Window) -> f32 {
    28.0 * ui_scale(window)
}

/// Pause / main menu title size.
pub fn menu_title_font_size(window: &Window) -> f32 {
    34.0 * ui_scale(window)
}

/// Secondary hint / stat line size (victory rows).
pub fn menu_hint_font_size(window: &Window) -> f32 {
    22.0 * ui_scale(window)
}

pub fn menu_text_color() -> Color {
    Color::srgb(0.95, 0.95, 1.0)
}

pub fn menu_title_text_color() -> Color {
    Color::srgb(0.97, 0.97, 1.0)
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
