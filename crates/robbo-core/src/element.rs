use crate::direction::Direction;

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
    QuestionMark,
    Capsule,
    ExtraLife,
    Bear { clockwise: bool },
    BlackBear { clockwise: bool },
    Bird { variant: BirdVariant },
    Butterfly,
    Gun {
        gun_type: GunType,
        direction: Direction,
        movable: bool,
        rotatable: bool,
    },
    Magnet { direction: Direction },
    Teleport { id: u8 },
    Projectile {
        direction: Direction,
        from_player: bool,
    },
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
    pub tick_counter: u32,
    pub hidden: bool,
}
