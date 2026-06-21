//! Deterministic level content hash for seeded music selection.

use sha2::Digest;
use robbo_core::{
    BirdVariant, Cell, Direction, ElementKind, ElementState, GunType, QuestionMarkContent,
    TileKind,
};

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
    h.update([u8::from(state.sliding)]);
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
        ElementKind::QuestionMark { content } => {
            h.update([7, qm_content_byte(*content)]);
        }
        ElementKind::Capsule => h.update([8]),
        ElementKind::Bear { clockwise } => h.update([10, u8::from(*clockwise)]),
        ElementKind::BlackBear { clockwise } => h.update([11, u8::from(*clockwise)]),
        ElementKind::Bird { variant, shooting } => {
            h.update([12, bird_variant_byte(*variant), u8::from(*shooting)]);
        }
        ElementKind::Butterfly => h.update([13]),
        ElementKind::Gun {
            gun_type,
            direction,
            move_dir,
            movable,
            rotatable,
            random_rotate,
        } => h.update([
            14,
            gun_type_byte(*gun_type),
            direction_byte(*direction),
            direction_byte(*move_dir),
            u8::from(*movable),
            u8::from(*rotatable),
            u8::from(*random_rotate),
        ]),
        ElementKind::Magnet { direction } => h.update([15, direction_byte(*direction)]),
        ElementKind::Teleport {
            group,
            pair_index,
        } => h.update([16, *group, *pair_index as u8]),
        ElementKind::Projectile {
            direction,
            from_player,
        } => h.update([17, direction_byte(*direction), u8::from(*from_player)]),
        ElementKind::Laser {
            direction,
            source_id,
        } => h.update([
            18,
            direction_byte(*direction),
            source_id.map(|id| id as u8).unwrap_or(255),
        ]),
        ElementKind::BlasterCell { direction, frame } => {
            h.update([19, direction_byte(*direction), *frame]);
        }
        ElementKind::BigBoom { content, ticks_left } => {
            h.update([20, qm_content_byte(*content), *ticks_left]);
        }
        ElementKind::BarbedWire => h.update([21]),
        ElementKind::Stop => h.update([22]),
    }
}

fn qm_content_byte(c: QuestionMarkContent) -> u8 {
    match c {
        QuestionMarkContent::Empty => 0,
        QuestionMarkContent::PushBox => 1,
        QuestionMarkContent::Screw => 2,
        QuestionMarkContent::BulletPickup => 3,
        QuestionMarkContent::Key => 4,
        QuestionMarkContent::Bomb => 5,
        QuestionMarkContent::Ground => 6,
        QuestionMarkContent::Butterfly => 7,
        QuestionMarkContent::Gun => 8,
        QuestionMarkContent::QuestionMark => 9,
        QuestionMarkContent::Capsule => 10,
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
        TileKind::Stop => 10,
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
                ElementState::new(1, ElementKind::Robbo, Direction::Down),
            )],
            required_screws: 1,
            barrier_directions: Default::default(),
        }
    }

    #[test]
    fn same_level_same_seed() {
        let a = sample_level();
        let b = sample_level();
        assert_eq!(level_content_seed(&a), level_content_seed(&b));
    }

    #[test]
    fn pick_index_wraps() {
        assert_eq!(pick_level_music_index(7, 5), Some(2));
        assert_eq!(pick_level_music_index(0, 0), None);
    }
}
