//! Deterministic level content hash for seeded music selection.

use sha2::Digest;
use robbo_core::{Cell, Direction, ElementKind, ElementState, GunType, TileKind, BirdVariant};

use crate::pack::Level;

/// SHA-256 digest of canonical level layout → first four bytes as `u32` seed.
pub fn level_content_seed(level: &Level) -> u32 {
    use sha2::{Digest, Sha256};

    let mut h = Sha256::new();
    h.update(level.width.to_le_bytes());
    h.update(level.height.to_le_bytes());
    h.update(level.colour.to_le_bytes());
    h.update(level.required_screws.to_le_bytes());

    for tile in &level.tiles {
        h.update([tile_discriminant(*tile)]);
    }

    let mut elems = level.elements.clone();
    elems.sort_by_key(|(c, s)| (c.col, c.row, s.id));
    for (cell, state) in elems {
        hash_cell(&mut h, cell);
        hash_element_state(&mut h, &state);
    }

    let digest = h.finalize();
    u32::from_le_bytes(digest[..4].try_into().expect("4 bytes"))
}

/// Pick a track index from a seed and track list.
pub fn pick_level_music_index(seed: u32, track_count: usize) -> Option<usize> {
    if track_count == 0 {
        return None;
    }
    Some(seed as usize % track_count)
}

fn hash_cell(h: &mut sha2::Sha256, cell: Cell) {
    h.update(cell.col.to_le_bytes());
    h.update(cell.row.to_le_bytes());
}

fn hash_element_state(h: &mut sha2::Sha256, state: &ElementState) {
    h.update(state.id.to_le_bytes());
    h.update([direction_byte(state.direction)]);
    h.update(state.tick_counter.to_le_bytes());
    h.update([u8::from(state.hidden)]);
    hash_element_kind(h, &state.kind);
}

fn hash_element_kind(h: &mut sha2::Sha256, kind: &ElementKind) {
    match kind {
        ElementKind::Robbo => h.update([0]),
        ElementKind::Screw => h.update([1]),
        ElementKind::BulletPickup => h.update([2]),
        ElementKind::Box => h.update([3]),
        ElementKind::PushBox => h.update([4]),
        ElementKind::Key => h.update([5]),
        ElementKind::Bomb => h.update([6]),
        ElementKind::QuestionMark => h.update([7]),
        ElementKind::Capsule => h.update([8]),
        ElementKind::ExtraLife => h.update([9]),
        ElementKind::Bear { clockwise } => {
            h.update([10, u8::from(*clockwise)]);
        }
        ElementKind::BlackBear { clockwise } => {
            h.update([11, u8::from(*clockwise)]);
        }
        ElementKind::Bird { variant } => {
            h.update([12, bird_variant_byte(*variant)]);
        }
        ElementKind::Butterfly => h.update([13]),
        ElementKind::Gun {
            gun_type,
            direction,
            movable,
            rotatable,
        } => {
            h.update([
                14,
                gun_type_byte(*gun_type),
                direction_byte(*direction),
                u8::from(*movable),
                u8::from(*rotatable),
            ]);
        }
        ElementKind::Magnet { direction } => {
            h.update([15, direction_byte(*direction)]);
        }
        ElementKind::Teleport { id } => {
            h.update([16, *id]);
        }
        ElementKind::Projectile {
            direction,
            from_player,
        } => {
            h.update([17, direction_byte(*direction), u8::from(*from_player)]);
        }
    }
}

fn tile_discriminant(tile: TileKind) -> u8 {
    match tile {
        TileKind::Empty => 0,
        TileKind::WallGrey => 1,
        TileKind::WallGreen => 2,
        TileKind::WallBlack => 3,
        TileKind::WallRed => 4,
        TileKind::WallSolid => 5,
        TileKind::Ground => 6,
        TileKind::DoorClosed => 7,
        TileKind::DoorOpen => 8,
        TileKind::Barrier => 9,
    }
}

fn direction_byte(dir: Direction) -> u8 {
    match dir {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

fn bird_variant_byte(v: BirdVariant) -> u8 {
    match v {
        BirdVariant::Horizontal => 0,
        BirdVariant::Vertical => 1,
        BirdVariant::Firing => 2,
    }
}

fn gun_type_byte(g: GunType) -> u8 {
    match g {
        GunType::Regular => 0,
        GunType::Laser => 1,
        GunType::Blaster => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robbo_core::ElementState;

    fn sample_level() -> Level {
        Level {
            index: 1,
            width: 4,
            height: 4,
            colour: 608050,
            author: String::new(),
            notes: String::new(),
            tiles: vec![TileKind::Empty; 16],
            elements: vec![(
                Cell { col: 1, row: 1 },
                ElementState {
                    id: 1,
                    kind: ElementKind::Robbo,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            )],
            required_screws: 1,
        }
    }

    #[test]
    fn same_level_same_seed() {
        let a = sample_level();
        let b = sample_level();
        assert_eq!(level_content_seed(&a), level_content_seed(&b));
    }

    #[test]
    fn tile_change_changes_seed() {
        let mut a = sample_level();
        let mut b = sample_level();
        b.tiles[0] = TileKind::WallGrey;
        assert_ne!(level_content_seed(&a), level_content_seed(&b));
        let _ = &mut a;
    }

    #[test]
    fn pick_index_wraps() {
        assert_eq!(pick_level_music_index(7, 5), Some(2));
        assert_eq!(pick_level_music_index(0, 0), None);
    }
}
