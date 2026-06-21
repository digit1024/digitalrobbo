use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::ElementKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayerInput {
    Move(Direction),
    Shoot(Direction),
    Wait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeathCause {
    Enemy,
    Projectile,
    Explosion,
    Hazard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GameEvent {
    Moved {
        entity_id: u32,
        from: Cell,
        to: Cell,
    },
    Pushed {
        entity_id: u32,
        from: Cell,
        to: Cell,
    },
    Collected {
        kind: ElementKind,
        at: Cell,
    },
    Shot {
        from: Cell,
        direction: Direction,
    },
    Exploded {
        at: Cell,
    },
    Teleported {
        entity_id: u32,
        from: Cell,
        to: Cell,
    },
    Revealed {
        at: Cell,
    },
    DoorOpened,
    Died {
        entity_id: u32,
        cause: DeathCause,
    },
    LevelComplete,
    LevelFailed,
}
