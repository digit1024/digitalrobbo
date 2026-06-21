//! Small tween helpers shared by menu scenes (fade / shake).

use bevy::prelude::*;

use crate::viewport;

#[derive(Component)]
pub struct UiFade {
    pub duration: f32,
    pub from: f32,
    pub to: f32,
    pub elapsed: f32,
    pub active: bool,
}

impl UiFade {
    pub fn new(duration: f32, from: f32, to: f32) -> Self {
        Self {
            duration,
            from,
            to,
            elapsed: 0.0,
            active: true,
        }
    }

    pub fn alpha(&self) -> f32 {
        if !self.active || self.duration <= 0.0 {
            return self.to;
        }
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * t
    }

    pub fn finished(&self) -> bool {
        !self.active || self.elapsed >= self.duration
    }
}

#[derive(Component)]
pub struct UiShake {
    pub duration: f32,
    pub amplitude: f32,
    pub anchor: Vec3,
    pub elapsed: f32,
    pub active: bool,
}

impl UiShake {
    pub fn new(duration: f32, amplitude: f32, anchor: Vec3) -> Self {
        Self {
            duration,
            amplitude,
            anchor,
            elapsed: 0.0,
            active: true,
        }
    }
}

pub fn tick_ui_fade(
    time: Res<Time>,
    mut sprites: Query<(&mut UiFade, &mut Sprite)>,
    mut text_colors: Query<(&mut UiFade, &mut TextColor), Without<Sprite>>,
) {
    let dt = time.delta_secs();
    for (mut fade, mut sprite) in &mut sprites {
        if !fade.active {
            continue;
        }
        fade.elapsed += dt;
        sprite.color = sprite.color.with_alpha(fade.alpha());
        if fade.finished() {
            fade.active = false;
        }
    }
    for (mut fade, mut color) in &mut text_colors {
        if !fade.active {
            continue;
        }
        fade.elapsed += dt;
        color.0 = color.0.with_alpha(fade.alpha());
        if fade.finished() {
            fade.active = false;
        }
    }
}

pub fn tick_ui_shake(time: Res<Time>, mut q: Query<(&mut UiShake, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut shake, mut tf) in &mut q {
        if !shake.active {
            tf.translation = shake.anchor;
            continue;
        }
        shake.elapsed += dt;
        if shake.elapsed >= shake.duration {
            shake.active = false;
            tf.translation = shake.anchor;
            continue;
        }
        let fade = 1.0 - shake.elapsed / shake.duration;
        let amp = shake.amplitude * fade;
        let off = viewport::shake_offset(shake.elapsed, amp);
        tf.translation = shake.anchor + Vec3::new(off.x, off.y, 0.0);
    }
}
