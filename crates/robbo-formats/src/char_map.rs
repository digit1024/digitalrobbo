use robbo_core::{
    Cell, Direction, ElementKind, ElementState, GunType, QuestionMarkContent, TileKind,
};
use crate::error::{FormatError, FormatResult};

pub fn char_to_tile(c: char) -> FormatResult<TileKind> {
    match c {
        '.' | ',' | '+' => Ok(TileKind::Empty),
        'O' | 'q' => Ok(TileKind::WallGrey),
        'o' => Ok(TileKind::WallGreen),
        '-' | 'a' => Ok(TileKind::WallBlack),
        'Q' => Ok(TileKind::WallRed),
        's' | 'S' | 'p' | 'P' => Ok(TileKind::WallSolid),
        'H' => Ok(TileKind::Ground),
        'D' => Ok(TileKind::DoorClosed),
        '=' => Ok(TileKind::Barrier),
        'X' => Ok(TileKind::Stop),
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
        '?' => ElementKind::QuestionMark {
            content: QuestionMarkContent::Screw,
        },
        '!' => ElementKind::Capsule,
        '@' => ElementKind::Bear { clockwise: false },
        '*' => ElementKind::BlackBear { clockwise: true },
        '^' => ElementKind::Bird {
            variant: robbo_core::BirdVariant::Horizontal,
            shooting: false,
        },
        'V' => ElementKind::Butterfly,
        '&' => ElementKind::Teleport {
            group: 0,
            pair_index: 0,
        },
        '}' => ElementKind::Gun {
            gun_type: GunType::Regular,
            direction: Direction::Right,
            move_dir: Direction::Right,
            movable: false,
            rotatable: false,
            random_rotate: false,
        },
        'M' => ElementKind::Magnet {
            direction: Direction::Right,
        },
        'k' => ElementKind::BarbedWire,
        'L' => ElementKind::Laser {
            direction: Direction::Right,
            source_id: None,
            solid: true,
            returning: false,
        },
        'l' => ElementKind::Laser {
            direction: Direction::Down,
            source_id: None,
            solid: true,
            returning: false,
        },
        _ => return Ok(None),
    };
    let direction = match &el {
        ElementKind::Bear { .. } | ElementKind::BlackBear { .. } | ElementKind::Bird { .. } => {
            Direction::Right
        }
        ElementKind::Magnet { direction } => *direction,
        _ => Direction::Down,
    };
    Ok(Some((el, direction)))
}

pub fn tile_or_element(c: char) -> FormatResult<(TileKind, Option<ElementState>)> {
    if c == '+' {
        return Ok((TileKind::Empty, None));
    }
    if let Ok(tile) = char_to_tile(c) {
        if !matches!(tile, TileKind::Empty | TileKind::Ground) {
            return Ok((tile, None));
        }
    }
    if let Some((kind, direction)) = char_to_element(c)? {
        let tile = if c == 'H' {
            TileKind::Ground
        } else {
            TileKind::Empty
        };
        let mut state = ElementState::new(0, kind.clone(), direction);
        state.hidden = matches!(kind, ElementKind::QuestionMark { .. });
        return Ok((tile, Some(state)));
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
            let group = parts[3].parse::<u8>().unwrap_or(0);
            let pair_index = parts[4].parse::<i8>().unwrap_or(0);
            elements[idx].1.kind = ElementKind::Teleport {
                group,
                pair_index,
            };
        }
        'L' | 'l' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            elements[idx].1.kind = ElementKind::Laser {
                direction: dir,
                source_id: None,
                solid: true,
                returning: false,
            };
            elements[idx].1.direction = dir;
        }
        '}' if parts.len() >= 5 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            let move_dir = direction_from_gnurobbo(parts[4].parse().unwrap_or(0));
            let gun_type = if parts.len() >= 6 {
                gun_type_from_gnurobbo(parts[5].parse().unwrap_or(0))
            } else {
                GunType::Regular
            };
            let movable = parts.len() >= 7 && parts[6] == "1";
            let rotatable = parts.len() >= 8 && parts[7] == "1";
            let random_rotate = parts.len() >= 9 && parts[8] == "1";
            elements[idx].1.kind = ElementKind::Gun {
                gun_type,
                direction: dir,
                move_dir,
                movable,
                rotatable,
                random_rotate,
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
            let shot_dir = if parts.len() >= 5 {
                direction_from_gnurobbo(parts[4].parse().unwrap_or(0))
            } else {
                dir
            };
            let shooting = parts.len() >= 6 && parts[5] == "1";
            let variant = if dir == Direction::Up || dir == Direction::Down {
                robbo_core::BirdVariant::Vertical
            } else {
                robbo_core::BirdVariant::Horizontal
            };
            elements[idx].1.kind = ElementKind::Bird {
                variant,
                shooting,
            };
            elements[idx].1.direction = dir;
            elements[idx].1.shot_direction = shot_dir;
        }
        'M' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
            elements[idx].1.kind = ElementKind::Magnet { direction: dir };
            elements[idx].1.direction = dir;
        }
        '=' if parts.len() >= 4 => {
            let dir = direction_from_gnurobbo(parts[3].parse().unwrap_or(0));
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
        TileKind::Stop => 'X',
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
        ElementKind::QuestionMark { .. } => '?',
        ElementKind::Capsule => '!',
        ElementKind::Bear { .. } => '@',
        ElementKind::BlackBear { .. } => '*',
        ElementKind::Bird { .. } => '^',
        ElementKind::Butterfly => 'V',
        ElementKind::Teleport { .. } => '&',
        ElementKind::Gun { .. } => '}',
        ElementKind::Magnet { .. } => 'M',
        ElementKind::Projectile { .. } => '.',
        ElementKind::Laser { direction, .. } => match direction {
            Direction::Down | Direction::Up => 'l',
            _ => 'L',
        },
        ElementKind::BlasterCell { .. } => '.',
        ElementKind::BigBoom { .. } => '.',
        ElementKind::BarbedWire => 'k',
        ElementKind::Stop => 'X',
    }
}

pub fn cell_from_grid(col: usize, row: usize) -> Cell {
    Cell::new(col as i16, row as i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teleport_additional_parses_group_and_index() {
        let mut elements = vec![(
            Cell::new(3, 1),
            ElementState::new(
                1,
                ElementKind::Teleport {
                    group: 0,
                    pair_index: 0,
                },
                Direction::Down,
            ),
        )];
        apply_additional_line(&mut elements, "3.1.&.1.0");
        assert!(matches!(
            elements[0].1.kind,
            ElementKind::Teleport {
                group: 1,
                pair_index: 0
            }
        ));
        apply_additional_line(&mut elements, "12.11.&.3.1");
        // cell not in list — no panic
    }

    #[test]
    fn plus_is_empty_tile() {
        let (tile, el) = tile_or_element('+').unwrap();
        assert_eq!(tile, TileKind::Empty);
        assert!(el.is_none());
    }

    #[test]
    fn barbed_wire_is_element() {
        let (tile, el) = tile_or_element('k').unwrap();
        assert_eq!(tile, TileKind::Empty);
        assert!(matches!(el.unwrap().kind, ElementKind::BarbedWire));
    }
}
