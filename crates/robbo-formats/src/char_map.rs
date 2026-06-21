use robbo_core::{Cell, Direction, ElementKind, ElementState, GunType, TileKind};
use crate::error::{FormatError, FormatResult};

pub fn char_to_tile(c: char) -> FormatResult<TileKind> {
    match c {
        '.' | ',' => Ok(TileKind::Empty),
        'O' | 'q' => Ok(TileKind::WallGrey),
        'o' => Ok(TileKind::WallGreen),
        '-' | 'a' | 'L' | 'l' | 'k' => Ok(TileKind::WallBlack),
        'Q' => Ok(TileKind::WallRed),
        's' | 'S' | 'p' | 'P' | 'X' => Ok(TileKind::WallSolid),
        'H' => Ok(TileKind::Ground),
        'D' => Ok(TileKind::DoorClosed),
        '=' => Ok(TileKind::Barrier),
        _ => Err(FormatError::UnknownChar(c)),
    }
}

pub fn char_to_element(c: char) -> FormatResult<Option<(ElementKind, Direction)>> {
    let el = match c {
        'R' => ElementKind::Robbo,
        'T' => ElementKind::Screw,
        '\'' => ElementKind::BulletPickup,
        '#' => ElementKind::Box,
        '~' => ElementKind::PushBox,
        '%' => ElementKind::Key,
        'b' | 'B' => ElementKind::Bomb,
        '?' => ElementKind::QuestionMark,
        '!' => ElementKind::Capsule,
        '+' => ElementKind::ExtraLife,
        '@' => ElementKind::Bear { clockwise: false },
        '*' => ElementKind::BlackBear { clockwise: true },
        '^' => ElementKind::Bird {
            variant: robbo_core::BirdVariant::Horizontal,
        },
        'V' => ElementKind::Butterfly,
        '&' => ElementKind::Teleport { id: 0 },
        '}' => ElementKind::Gun {
            gun_type: GunType::Regular,
            direction: Direction::Right,
            movable: false,
            rotatable: false,
        },
        'M' => ElementKind::Magnet {
            direction: Direction::Left,
        },
        _ => return Ok(None),
    };
    Ok(Some((el, Direction::Down)))
}

pub fn tile_or_element(c: char) -> FormatResult<(TileKind, Option<ElementState>)> {
    if let Ok(tile) = char_to_tile(c) {
        if tile != TileKind::Empty && tile != TileKind::Ground {
            return Ok((tile, None));
        }
    }
    if let Some((kind, direction)) = char_to_element(c)? {
        let tile = if c == 'H' {
            TileKind::Ground
        } else {
            TileKind::Empty
        };
        return Ok((
            tile,
            Some(ElementState {
                id: 0,
                kind: kind.clone(),
                direction,
                tick_counter: 0,
                hidden: matches!(kind, ElementKind::QuestionMark),
            }),
        ));
    }
    char_to_tile(c).map(|t| (t, None))
}

pub fn direction_from_gnurobbo(v: u8) -> Direction {
    match v {
        0 => Direction::Right,
        1 => Direction::Down,
        2 => Direction::Left,
        3 => Direction::Up,
        _ => Direction::Down,
    }
}

pub fn gun_type_from_gnurobbo(v: u8) -> GunType {
    match v {
        1 => GunType::Laser,
        2 => GunType::Blaster,
        _ => GunType::Regular,
    }
}

pub fn apply_additional_line(elements: &mut [(Cell, ElementState)], line: &str) {
    let parts: Vec<&str> = line.split('.').collect();
    if parts.len() < 3 {
        return;
    }
    let Ok(col) = parts[0].parse::<i16>() else {
        return;
    };
    let Ok(row) = parts[1].parse::<i16>() else {
        return;
    };
    let cell = Cell::new(col, row);
    let Some(ch) = parts[2].chars().next() else {
        return;
    };

    let Some((idx, _)) = elements
        .iter()
        .enumerate()
        .find(|(_, (c, _))| *c == cell)
    else {
        return;
    };

    match ch {
        '&' if parts.len() >= 5 => {
            if let Ok(id) = parts[4].parse::<u8>() {
                elements[idx].1.kind = ElementKind::Teleport { id };
            }
        }
        '}' if parts.len() >= 5 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            let gun_type = if parts.len() >= 6 {
                gun_type_from_gnurobbo(parts[4].parse().unwrap_or(0))
            } else {
                GunType::Regular
            };
            let movable = parts.len() >= 7 && parts[5] == "1";
            let rotatable = parts.len() >= 8 && parts[6] == "1";
            elements[idx].1.kind = ElementKind::Gun {
                gun_type,
                direction: dir,
                movable,
                rotatable,
            };
            elements[idx].1.direction = dir;
        }
        '@' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            elements[idx].1.kind = ElementKind::Bear { clockwise: false };
            elements[idx].1.direction = dir;
        }
        '*' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            elements[idx].1.kind = ElementKind::BlackBear { clockwise: true };
            elements[idx].1.direction = dir;
        }
        '^' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            let variant = if parts.len() >= 6 && parts[5] == "1" {
                robbo_core::BirdVariant::Firing
            } else if dir == Direction::Up || dir == Direction::Down {
                robbo_core::BirdVariant::Vertical
            } else {
                robbo_core::BirdVariant::Horizontal
            };
            elements[idx].1.kind = ElementKind::Bird { variant };
            elements[idx].1.direction = dir;
        }
        'M' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            elements[idx].1.kind = ElementKind::Magnet { direction: dir };
            elements[idx].1.direction = dir;
        }
        _ => {}
    }
}

pub fn tile_to_char(tile: TileKind) -> char {
    match tile {
        TileKind::Empty => '.',
        TileKind::WallGrey => 'O',
        TileKind::WallGreen => 'o',
        TileKind::WallBlack => '-',
        TileKind::WallRed => 'Q',
        TileKind::WallSolid => 's',
        TileKind::Ground => 'H',
        TileKind::DoorClosed | TileKind::DoorOpen => 'D',
        TileKind::Barrier => '=',
    }
}

pub fn element_to_char(kind: &ElementKind) -> char {
    match kind {
        ElementKind::Robbo => 'R',
        ElementKind::Screw => 'T',
        ElementKind::BulletPickup => '\'',
        ElementKind::Box => '#',
        ElementKind::PushBox => '~',
        ElementKind::Key => '%',
        ElementKind::Bomb => 'b',
        ElementKind::QuestionMark => '?',
        ElementKind::Capsule => '!',
        ElementKind::ExtraLife => '+',
        ElementKind::Bear { .. } => '@',
        ElementKind::BlackBear { .. } => '*',
        ElementKind::Bird { .. } => '^',
        ElementKind::Butterfly => 'V',
        ElementKind::Teleport { .. } => '&',
        ElementKind::Gun { .. } => '}',
        ElementKind::Magnet { .. } => 'M',
        ElementKind::Projectile { .. } => '.',
    }
}

pub fn cell_from_grid(col: usize, row: usize) -> Cell {
    Cell::new(col as i16, row as i16)
}
