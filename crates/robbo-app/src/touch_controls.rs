//! Android-only donut D-pads (move left, fire right).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::widget::ImageNode;
use bevy::ui::FocusPolicy;
use bevy::window::PrimaryWindow;
use robbo_core::Direction;

use crate::app_state::AppState;
use crate::bridge::CoreBridge;
use crate::input::{
    apply_move_pad_hold, apply_move_pad_press, apply_move_pad_release, apply_shoot_pad_press,
    SteeringState,
};
use crate::ui::LevelCountdown;
use crate::viewport::DESIGN_HEIGHT;

/// Pad diameter at design resolution (1280×768).
const PAD_DIAMETER_DESIGN: f32 = 300.0;
/// Margin from screen edges (design px).
const PAD_MARGIN_DESIGN: f32 = 24.0;
/// Min gap between the two pads (design px).
const PAD_GAP_DESIGN: f32 = 40.0;
/// Each pad may use at most this fraction of screen height.
const PAD_MAX_HEIGHT_FRAC: f32 = 0.36;
/// Inner hole diameter / outer diameter (hand drawing ≈ ⅓).
const INNER_HOLE_DIAMETER_FRAC: f32 = 1.0 / 3.0;
/// Half-width of diagonal gap between wedges (radians).
const GAP_HALF_ANGLE: f32 = 0.105;
/// Raster resolution for pad textures.
const TEX_SIZE: u32 = 512;

/// Shared geometry: texture outer radius / texture half-size (must match rasterizer).
const OUTER_RADIUS_FRAC: f32 = (TEX_SIZE as f32 * 0.5 - 2.0) / (TEX_SIZE as f32 * 0.5);
/// Inner radius / outer radius (inner hole diameter is ⅓ of outer diameter).
const INNER_RADIUS_RATIO: f32 = INNER_HOLE_DIAMETER_FRAC;

#[derive(Clone, Copy)]
struct PadLayout {
    size: f32,
    margin: f32,
}

#[derive(Component)]
pub struct TouchControlsRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadQuarter {
    Up,
    Down,
    Left,
    Right,
}

impl PadQuarter {
    const ALL: [PadQuarter; 4] = [
        PadQuarter::Up,
        PadQuarter::Down,
        PadQuarter::Left,
        PadQuarter::Right,
    ];
}

#[derive(Clone, Copy)]
struct PadPalette {
    segment: [u8; 4],
    highlight: [u8; 4],
}

impl PadPalette {
    fn move_pad() -> Self {
        Self {
            segment: [24, 42, 92, 235],
            highlight: [70, 118, 210, 250],
        }
    }

    fn fire_pad() -> Self {
        Self {
            segment: [118, 42, 18, 235],
            highlight: [230, 108, 38, 250],
        }
    }
}

#[derive(Component)]
pub(crate) struct DonutPad {
    is_move: bool,
}

#[derive(Component, Default)]
pub(crate) struct DonutPadActive {
    quarter: Option<PadQuarter>,
}

#[derive(Component, Default)]
pub(crate) struct DonutPadTracking {
    touch_id: Option<u64>,
}

#[derive(Component)]
pub(crate) struct DonutQuarterOverlay {
    quarter: PadQuarter,
}

fn design_to_screen(design: f32, window: &Window) -> f32 {
    design * window.height() / DESIGN_HEIGHT
}

fn pad_radii(node: &ComputedNode) -> (f32, f32) {
    let half = 0.5 * node.size().x.min(node.size().y);
    let outer = half * OUTER_RADIUS_FRAC;
    let inner = outer * INNER_RADIUS_RATIO;
    (outer, inner)
}

fn quarter_to_direction(q: PadQuarter) -> Direction {
    match q {
        PadQuarter::Up => Direction::Up,
        PadQuarter::Down => Direction::Down,
        PadQuarter::Left => Direction::Left,
        PadQuarter::Right => Direction::Right,
    }
}

fn quarter_from_angle(angle: f32) -> PadQuarter {
    const FRAC: f32 = std::f32::consts::FRAC_PI_4;
    if angle >= -FRAC && angle < FRAC {
        PadQuarter::Right
    } else if angle >= FRAC && angle < 3.0 * FRAC {
        PadQuarter::Down
    } else if angle <= -FRAC && angle > -3.0 * FRAC {
        PadQuarter::Up
    } else {
        PadQuarter::Left
    }
}

fn angle_near(a: f32, b: f32, eps: f32) -> bool {
    let mut d = (a - b).abs();
    if d > std::f32::consts::PI {
        d = std::f32::consts::TAU - d;
    }
    d < eps
}

fn in_diagonal_gap(angle: f32) -> bool {
    const FRAC: f32 = std::f32::consts::FRAC_PI_4;
    angle_near(angle, FRAC, GAP_HALF_ANGLE)
        || angle_near(angle, -FRAC, GAP_HALF_ANGLE)
        || angle_near(angle, 3.0 * FRAC, GAP_HALF_ANGLE)
        || angle_near(angle, -3.0 * FRAC, GAP_HALF_ANGLE)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 == edge1 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn donut_pixel(
    x: f32,
    y: f32,
    center: f32,
    outer: f32,
    inner: f32,
    quarter_only: Option<PadQuarter>,
    palette: PadPalette,
    highlight: bool,
) -> [u8; 4] {
    let dx = x - center;
    let dy = y - center;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < inner || dist > outer + 1.0 {
        return [0, 0, 0, 0];
    }

    let angle = dy.atan2(dx);
    if in_diagonal_gap(angle) {
        return [0, 0, 0, 0];
    }

    let quarter = quarter_from_angle(angle);
    if quarter_only.is_some_and(|q| q != quarter) {
        return [0, 0, 0, 0];
    }

    let c = if highlight {
        palette.highlight
    } else {
        palette.segment
    };
    let alpha = smoothstep(outer + 1.0, outer - 1.0, dist);
    [c[0], c[1], c[2], (c[3] as f32 * alpha) as u8]
}

fn rasterize_donut(
    palette: PadPalette,
    quarter_only: Option<PadQuarter>,
    highlight: bool,
) -> Image {
    let size = TEX_SIZE;
    let center = size as f32 * 0.5;
    let outer = center * OUTER_RADIUS_FRAC;
    let inner = outer * INNER_RADIUS_RATIO;

    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let rgba = donut_pixel(
                px,
                py,
                center,
                outer,
                inner,
                quarter_only,
                palette,
                highlight,
            );
            data.extend_from_slice(&rgba);
        }
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Same coordinate path as Bevy's `ui_focus_system` (physical viewport space).
fn pointer_to_normalized(
    pointer_logical: Vec2,
    window: &Window,
    camera: &Camera,
    node: &ComputedNode,
    global: &GlobalTransform,
) -> Option<Vec2> {
    let size = node.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let viewport_min = camera
        .physical_viewport_rect()
        .map(|rect| rect.min.as_vec2())
        .unwrap_or_default();
    let cursor = pointer_logical * window.scale_factor() - viewport_min;
    let node_rect = Rect::from_center_size(global.translation().truncate(), size);
    Some((cursor - node_rect.min) / size)
}

fn point_in_pad(norm: Vec2) -> bool {
    (0.0..=1.0).contains(&norm.x) && (0.0..=1.0).contains(&norm.y)
}

fn donut_quarter_at_norm(norm: Vec2, node: &ComputedNode) -> Option<PadQuarter> {
    if !point_in_pad(norm) {
        return None;
    }
    let size = node.size();
    let px = (norm.x - 0.5) * size.x;
    let py = (norm.y - 0.5) * size.y;
    let (outer, inner) = pad_radii(node);
    let dist = Vec2::new(px, py).length();

    // Dead center + diagonal gaps; outer bound matches drawn ring (no outward slop).
    if dist < inner || dist > outer {
        return None;
    }

    let angle = py.atan2(px);
    if in_diagonal_gap(angle) {
        return None;
    }
    Some(quarter_from_angle(angle))
}

fn set_quarter_overlays(
    active: Option<PadQuarter>,
    children: &Children,
    overlays: &mut Query<(&DonutQuarterOverlay, &mut Visibility)>,
) {
    for child in children.iter() {
        let Ok((overlay, mut vis)) = overlays.get_mut(child) else {
            continue;
        };
        *vis = if active == Some(overlay.quarter) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn compute_pad_layout(window: &Window) -> PadLayout {
    let w = window.width();
    let h = window.height();
    let margin = design_to_screen(PAD_MARGIN_DESIGN, window);
    let gap = design_to_screen(PAD_GAP_DESIGN, window);
    let desired = design_to_screen(PAD_DIAMETER_DESIGN, window);
    let max_by_width = ((w - gap) * 0.5 - margin).max(160.0);
    let max_by_height = (h * PAD_MAX_HEIGHT_FRAC - margin).max(160.0);
    let size = desired.min(max_by_width).min(max_by_height);
    PadLayout { size, margin }
}

/// Bottom inset reserved by touch pads (pad height + margin).
pub(crate) fn bottom_chrome_height(window: &Window) -> f32 {
    let PadLayout { size, margin } = compute_pad_layout(window);
    size + margin
}

pub fn spawn_touch_controls(
    mut commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let PadLayout { size, margin } = compute_pad_layout(window);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(90),
            TouchControlsRoot,
        ))
        .with_children(|root| {
            spawn_donut_pad(
                root,
                &mut images,
                size,
                margin,
                true,
                PadPalette::move_pad(),
            );
            spawn_donut_pad(
                root,
                &mut images,
                size,
                margin,
                false,
                PadPalette::fire_pad(),
            );
        });
}

fn spawn_donut_pad(
    parent: &mut ChildSpawnerCommands,
    images: &mut Assets<Image>,
    size: f32,
    margin: f32,
    is_move: bool,
    palette: PadPalette,
) {
    let (left, right) = if is_move {
        (Val::Px(margin), Val::Auto)
    } else {
        (Val::Auto, Val::Px(margin))
    };

    let base = images.add(rasterize_donut(palette, None, false));

    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left,
                right,
                bottom: Val::Px(margin),
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
            BackgroundColor(Color::NONE),
            FocusPolicy::Pass,
            DonutPad { is_move },
            DonutPadActive::default(),
            DonutPadTracking::default(),
        ))
        .with_children(|pad| {
            pad.spawn((
                ImageNode {
                    image: base,
                    ..default()
                },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                FocusPolicy::Pass,
            ));

            for quarter in PadQuarter::ALL {
                let overlay = images.add(rasterize_donut(palette, Some(quarter), true));
                pad.spawn((
                    ImageNode {
                        image: overlay,
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    Visibility::Hidden,
                    FocusPolicy::Pass,
                    DonutQuarterOverlay { quarter },
                ));
            }
        });
}

fn apply_quarter_change(
    pad: &DonutPad,
    prev: &mut DonutPadActive,
    quarter: Option<PadQuarter>,
    bridge: &mut CoreBridge,
    steering: &mut SteeringState,
) {
    if quarter == prev.quarter {
        return;
    }
    if let Some(old) = prev.quarter.take() {
        if pad.is_move {
            apply_move_pad_release(steering, quarter_to_direction(old));
        }
    }
    prev.quarter = quarter;
    if let Some(q) = quarter {
        let dir = quarter_to_direction(q);
        if pad.is_move {
            apply_move_pad_press(bridge, steering, dir);
        } else {
            apply_shoot_pad_press(bridge, steering, dir);
        }
    }
}

pub fn donut_pad_touch(
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<&Camera, With<Camera2d>>,
    touches: Res<Touches>,
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    mut pads: Query<(
        &DonutPad,
        &ComputedNode,
        &GlobalTransform,
        &mut DonutPadActive,
        &mut DonutPadTracking,
        &Children,
    )>,
    mut overlays: Query<(&DonutQuarterOverlay, &mut Visibility)>,
) {
    if *state.get() != AppState::Playing || countdown.blocks_input() {
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };
    let Ok(camera) = camera.single() else {
        return;
    };

    for (pad, node, global, mut active, mut tracking, children) in &mut pads {
        if let Some(id) = tracking.touch_id {
            if touches.just_released(id) || touches.just_canceled(id) {
                apply_quarter_change(pad, &mut active, None, &mut bridge, &mut steering);
                set_quarter_overlays(None, children, &mut overlays);
                tracking.touch_id = None;
                continue;
            }

            let Some(touch) = touches.get_pressed(id) else {
                apply_quarter_change(pad, &mut active, None, &mut bridge, &mut steering);
                set_quarter_overlays(None, children, &mut overlays);
                tracking.touch_id = None;
                continue;
            };

            let norm = pointer_to_normalized(touch.position(), window, camera, node, global);
            let quarter = norm.and_then(|n| donut_quarter_at_norm(n, node));
            apply_quarter_change(pad, &mut active, quarter, &mut bridge, &mut steering);
            set_quarter_overlays(active.quarter, children, &mut overlays);
            if pad.is_move {
                if let Some(q) = active.quarter {
                    apply_move_pad_hold(&mut steering, quarter_to_direction(q));
                }
            }
            continue;
        }

        for touch in touches.iter_just_pressed() {
            let Some(norm) = pointer_to_normalized(touch.position(), window, camera, node, global)
            else {
                continue;
            };
            if !point_in_pad(norm) {
                continue;
            }
            tracking.touch_id = Some(touch.id());
            let quarter = donut_quarter_at_norm(norm, node);
            apply_quarter_change(pad, &mut active, quarter, &mut bridge, &mut steering);
            set_quarter_overlays(active.quarter, children, &mut overlays);
            break;
        }
    }
}

pub fn cleanup_touch_controls(mut commands: Commands, q: Query<Entity, With<TouchControlsRoot>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}
