use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, next_rand, roll_one_in_eight};
use crate::events::GameEvent;
use crate::world::World;

impl World {
    pub(crate) fn tick_guns(&mut self, events: &mut Vec<GameEvent>) {
        let gun_ids: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Gun { .. }))
            .map(|(_, s)| s.id)
            .collect();

        for id in gun_ids {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            self.elements[i].1.tick_counter += 1;
            let tick_counter = self.elements[i].1.tick_counter;

            let (
                gun_type,
                direction,
                move_dir,
                movable,
                rotatable,
                random_rotate,
                gun_cell,
            ) = match self.elements[i].1.kind {
                ElementKind::Gun {
                    gun_type,
                    direction,
                    move_dir,
                    movable,
                    rotatable,
                    random_rotate,
                } => (
                    gun_type,
                    direction,
                    move_dir,
                    movable,
                    rotatable,
                    random_rotate,
                    self.elements[i].0,
                ),
                _ => continue,
            };

            let barrel_blocked = self.gun_barrel_blocked(gun_cell, direction);

            if rotatable && !barrel_blocked && tick_counter % 20 == 0 {
                let new_dir = if random_rotate {
                    Self::random_dir(&mut self.rng_state)
                } else {
                    direction.turn_right()
                };
                if let Some((i, _)) = self.element_by_id(id) {
                    if let ElementKind::Gun {
                        ref mut direction, ..
                    } = self.elements[i].1.kind
                    {
                        *direction = new_dir;
                    }
                    self.elements[i].1.direction = new_dir;
                }
            }

            if movable && !barrel_blocked && tick_counter % 8 == 0 {
                self.try_move_gun(id, gun_cell, move_dir, events);
            }

            if tick_counter % 8 != 0 {
                continue;
            }

            if !roll_one_in_eight(&mut self.rng_state) {
                continue;
            }

            let from = self
                .element_by_id(id)
                .map(|(i, _)| self.elements[i].0)
                .unwrap_or(gun_cell);
            let shoot_dir = self
                .element_by_id(id)
                .map(|(i, _)| {
                    if let ElementKind::Gun { direction, .. } = self.elements[i].1.kind {
                        direction
                    } else {
                        direction
                    }
                })
                .unwrap_or(direction);

            self.gun_shoot(from, shoot_dir, gun_type, Some(id), events);
        }
    }

    fn try_move_gun(
        &mut self,
        id: u32,
        cell: Cell,
        move_dir: Direction,
        events: &mut Vec<GameEvent>,
    ) {
        let (dc, dr) = move_dir.delta();
        let next = cell.offset(dc, dr);
        let blocked = !self.in_bounds(next)
            || self.tile_at(next).is_some_and(|t| t.blocks_movement())
            || self.element_at(next).is_some();
        if blocked {
            let bounce = move_dir.opposite();
            if let Some((i, _)) = self.element_by_id(id) {
                if let ElementKind::Gun {
                    ref mut move_dir, ..
                } = self.elements[i].1.kind
                {
                    *move_dir = bounce;
                }
            }
            return;
        }
        if let Some((i, _)) = self.element_by_id(id) {
            self.elements[i].0 = next;
            events.push(GameEvent::Moved {
                entity_id: id,
                from: cell,
                to: next,
            });
        }
    }

    fn random_dir(rng: &mut u64) -> Direction {
        let v = next_rand(rng) % 4;
        match v {
            0 => Direction::Right,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Up,
        }
    }
}
