use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{BirdVariant, ElementKind, ElementState, GunType, roll_one_in_eight};
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    pub(crate) fn tick_enemies(&mut self, events: &mut Vec<GameEvent>) {
        let robbo = self.robbo_cell();
        let snapshot: Vec<(Cell, ElementState)> = self
            .elements
            .iter()
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        let mut updates: Vec<(u32, u32, Direction, Option<Cell>)> = Vec::new();
        let mut bird_shots: Vec<(Cell, Direction)> = Vec::new();

        for (cell, state) in snapshot.iter() {
            let id = state.id;
            let tick_counter = state.tick_counter + 1;
            let delay = match state.kind {
                ElementKind::Bear { .. } => 4,
                ElementKind::BlackBear { .. } => 2,
                ElementKind::Bird { .. } => 3,
                ElementKind::Butterfly => 2,
                _ => {
                    updates.push((id, tick_counter, state.direction, None));
                    continue;
                }
            };
            if tick_counter % delay != 0 {
                updates.push((id, tick_counter, state.direction, None));
                continue;
            }

            let mut direction = state.direction;
            let new_cell = match &state.kind {
                ElementKind::Bear { .. } => {
                    let (next, new_dir) =
                        self.bear_step(*cell, direction, false, self.sensible_bears);
                    direction = new_dir;
                    next
                }
                ElementKind::BlackBear { .. } => {
                    let (next, new_dir) =
                        self.bear_step(*cell, direction, true, self.sensible_bears);
                    direction = new_dir;
                    next
                }
                ElementKind::Bird { variant, shooting } => {
                    let mut pos = *cell;
                    match variant {
                        BirdVariant::Horizontal => {
                            let (dc, _) = direction.delta();
                            let next = cell.offset(dc, 0);
                            if self.is_blocked_for_enemy(next) {
                                direction = direction.opposite();
                                let (dc, _) = direction.delta();
                                pos = cell.offset(dc, 0);
                            } else {
                                pos = next;
                            }
                        }
                        BirdVariant::Vertical => {
                            let (_, dr) = direction.delta();
                            let next = cell.offset(0, dr);
                            if self.is_blocked_for_enemy(next) {
                                direction = direction.opposite();
                                let (_, dr) = direction.delta();
                                pos = cell.offset(0, dr);
                            } else {
                                pos = next;
                            }
                        }
                        BirdVariant::Firing => {}
                    }
                    if *shooting && roll_one_in_eight(&mut self.rng_state) {
                        bird_shots.push((pos, state.shot_direction));
                    }
                    pos
                }
                ElementKind::Butterfly => {
                    let moved = self.butterfly_move(*cell, direction);
                    direction = self.butterfly_next_direction(moved);
                    moved
                }
                _ => *cell,
            };

            if matches!(state.kind, ElementKind::Butterfly) && new_cell != *cell {
                let dc = new_cell.col - cell.col;
                let dr = new_cell.row - cell.row;
                direction = if dc > 0 {
                    Direction::Right
                } else if dc < 0 {
                    Direction::Left
                } else if dr > 0 {
                    Direction::Down
                } else if dr < 0 {
                    Direction::Up
                } else {
                    direction
                };
            }

            let mv = if new_cell != *cell { Some(new_cell) } else { None };
            if mv == Some(new_cell) && robbo == Some(new_cell) {
                self.kill_robbo(DeathCause::Enemy, events);
                return;
            }
            updates.push((id, tick_counter, direction, mv));
        }

        for (id, tick_counter, direction, mv) in updates {
            if let Some((i, _)) = self.element_by_id(id) {
                self.elements[i].1.tick_counter = tick_counter;
                self.elements[i].1.direction = direction;
                if let Some(to) = mv {
                    let from = self.elements[i].0;
                    self.elements[i].0 = to;
                    events.push(GameEvent::Moved {
                        entity_id: id,
                        from,
                        to,
                    });
                }
            }
        }

        for (from, dir) in bird_shots {
            self.gun_shoot(from, dir, GunType::Regular, None, events);
        }
    }

    /// gnurobbo left/right-hand maze rule for BEAR / BEAR_B.
    fn bear_step(
        &self,
        cell: Cell,
        dir: Direction,
        right_hand: bool,
        sensible: bool,
    ) -> (Cell, Direction) {
        let side = if right_hand {
            dir.turn_right()
        } else {
            dir.turn_left()
        };
        let (sdc, sdr) = side.delta();
        let side_cell = cell.offset(sdc, sdr);

        let force_forward = if sensible {
            self.bear_force_forward(cell, dir, right_hand)
        } else {
            false
        };

        if !force_forward && self.is_bear_maze_empty(side_cell) {
            let (dc, dr) = side.delta();
            return (cell.offset(dc, dr), side);
        }

        let (fdc, fdr) = dir.delta();
        let forward = cell.offset(fdc, fdr);
        if self.is_bear_maze_empty(forward) {
            return (forward, dir);
        }

        let new_dir = if right_hand {
            dir.turn_left()
        } else {
            dir.turn_right()
        };
        (cell, new_dir)
    }

    fn bear_force_forward(&self, cell: Cell, dir: Direction, right_hand: bool) -> bool {
        let side_dir = if right_hand {
            dir.turn_right()
        } else {
            dir.turn_left()
        };
        let (sdc, sdr) = side_dir.delta();
        let side_cell = cell.offset(sdc, sdr);

        let behind_side = side_cell.offset(-dir.delta().0, -dir.delta().1);
        let behind_dir = if right_hand {
            dir.turn_left()
        } else {
            dir.turn_right()
        };
        let (bdc, bdr) = behind_dir.delta();
        let behind = behind_side.offset(bdc, bdr);

        let self_type = if right_hand { 2u8 } else { 1 };

        self.is_bear_maze_empty(side_cell)
            && self.is_bear_maze_empty(behind)
            && (self.is_bear_maze_empty(behind_side)
                || self.bear_maze_type(behind_side) == self_type)
    }

    fn bear_maze_type(&self, cell: Cell) -> u8 {
        if let Some((_, el)) = self.element_at(cell) {
            match el.kind {
                ElementKind::Bear { .. } => 1,
                ElementKind::BlackBear { .. } => 2,
                _ => 0,
            }
        } else {
            0
        }
    }

    fn is_bear_maze_empty(&self, cell: Cell) -> bool {
        if !self.in_bounds(cell) {
            return false;
        }
        if self.tile_at(cell).is_some_and(|t| t.blocks_movement()) {
            return false;
        }
        self.element_at(cell).is_none()
    }

    fn butterfly_move(&self, cell: Cell, direction: Direction) -> Cell {
        let (dc, dr) = direction.delta();
        let next = cell.offset(dc, dr);
        if !self.is_blocked_for_enemy(next) {
            next
        } else {
            cell
        }
    }

    fn butterfly_next_direction(&mut self, cell: Cell) -> Direction {
        if roll_one_in_eight(&mut self.rng_state) {
            let dirs = [
                Direction::Right,
                Direction::Down,
                Direction::Left,
                Direction::Up,
            ];
            return dirs[(self.tick as usize) % dirs.len()];
        }
        let Some(robbo) = self.robbo_cell() else {
            return Direction::Down;
        };
        if (self.tick & 1) == 0 {
            if robbo.col > cell.col {
                Direction::Right
            } else if robbo.col < cell.col {
                Direction::Left
            } else if robbo.row > cell.row {
                Direction::Down
            } else {
                Direction::Up
            }
        } else if robbo.row > cell.row {
            Direction::Down
        } else if robbo.row < cell.row {
            Direction::Up
        } else if robbo.col > cell.col {
            Direction::Right
        } else {
            Direction::Left
        }
    }
}
