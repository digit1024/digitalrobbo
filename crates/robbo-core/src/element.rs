use crate::direction::Direction;

/// Hidden content assigned to each `?` at level load (gnurobbo `random_id` pool).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QuestionMarkContent {
    Empty,
    PushBox,
    Screw,
    BulletPickup,
    Key,
    Bomb,
    Ground,
    Butterfly,
    Gun,
    QuestionMark,
    Capsule,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ElementKind {
    Robbo,
    Screw,
    BulletPickup,
    Box,
    PushBox,
    Key,
    Bomb,
    QuestionMark {
        content: QuestionMarkContent,
    },
    Capsule,
    Bear {
        clockwise: bool,
    },
    BlackBear {
        clockwise: bool,
    },
    Bird {
        variant: BirdVariant,
        shooting: bool,
    },
    Butterfly,
    Gun {
        gun_type: GunType,
        direction: Direction,
        move_dir: Direction,
        movable: bool,
        rotatable: bool,
        random_rotate: bool,
    },
    Magnet {
        direction: Direction,
    },
    Teleport {
        group: u8,
        pair_index: i8,
    },
    Projectile {
        direction: Direction,
        from_player: bool,
    },
    /// Laser beam segment (gnurobbo LASER_L / LASER_D).
    Laser {
        direction: Direction,
        source_id: Option<u32>,
        /// `false` = moving bolt (regular gun / player); `true` = solid beam segment.
        solid: bool,
        /// Solid laser reflection state at obstacles.
        returning: bool,
    },
    /// Blaster spread cell.
    BlasterCell {
        direction: Direction,
        frame: u8,
    },
    /// Explosion animation before question-mark reveal.
    BigBoom {
        content: QuestionMarkContent,
        ticks_left: u8,
    },
    BarbedWire,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BirdVariant {
    Horizontal,
    Vertical,
    Firing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GunType {
    Regular,
    Laser,
    Blaster,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ElementState {
    pub id: u32,
    pub kind: ElementKind,
    pub direction: Direction,
    /// Bird shot direction (gnurobbo `direction2`).
    pub shot_direction: Direction,
    pub tick_counter: u32,
    pub hidden: bool,
    /// Push-box sliding in progress.
    pub sliding: bool,
}

impl ElementState {
    pub fn new(id: u32, kind: ElementKind, direction: Direction) -> Self {
        let hidden = matches!(kind, ElementKind::QuestionMark { .. });
        Self {
            id,
            kind,
            direction,
            shot_direction: direction,
            tick_counter: 0,
            hidden,
            sliding: false,
        }
    }
}

impl QuestionMarkContent {
    pub const POOL_SENSIBLE: [QuestionMarkContent; 10] = [
        QuestionMarkContent::Empty,
        QuestionMarkContent::PushBox,
        QuestionMarkContent::Screw,
        QuestionMarkContent::BulletPickup,
        QuestionMarkContent::Key,
        QuestionMarkContent::Bomb,
        QuestionMarkContent::Ground,
        QuestionMarkContent::Butterfly,
        QuestionMarkContent::Gun,
        QuestionMarkContent::Capsule,
    ];

    pub const POOL_FULL: [QuestionMarkContent; 11] = [
        QuestionMarkContent::Empty,
        QuestionMarkContent::PushBox,
        QuestionMarkContent::Screw,
        QuestionMarkContent::BulletPickup,
        QuestionMarkContent::Key,
        QuestionMarkContent::Bomb,
        QuestionMarkContent::Ground,
        QuestionMarkContent::Butterfly,
        QuestionMarkContent::Gun,
        QuestionMarkContent::QuestionMark,
        QuestionMarkContent::Capsule,
    ];

    pub fn roll(rng: &mut u64, sensible: bool) -> Self {
        let pool = if sensible {
            &Self::POOL_SENSIBLE[..]
        } else {
            &Self::POOL_FULL[..]
        };
        let idx = (next_rand(rng) as usize) % pool.len();
        pool[idx]
    }
}

/// Deterministic LCG (same seed → same sequence).
pub fn next_rand(state: &mut u64) -> u32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*state >> 32) as u32
}

pub fn roll_one_in_eight(rng: &mut u64) -> bool {
    next_rand(rng) & 7 == 0
}
