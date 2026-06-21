use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{BirdVariant, ElementKind, ElementState, roll_one_in_eight};
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
                ElementKind::Bear { clockwise } | ElementKind::BlackBear { clockwise } => {
                    let (next, new_dir) = self.wall_follow_with_dir(*cell, direction, *clockwise);
                    direction = new_dir;
                    next
                }
                ElementKind::Bird { variant, shooting } => {
                    if *shooting && roll_one_in_eight(&mut self.rng_state) {
                        bird_shots.push((*cell, state.shot_direction));
                    }
                    match variant {
                        BirdVariant::Horizontal => {
                            let (dc, _) = direction.delta();
                            let next = cell.offset(dc, 0);
                            if self.is_blocked_for_enemy(next) {
                                direction = direction.opposite();
                                cell.offset(direction.delta().0, 0)
                            } else {
                                next
                            }
                        }
                        BirdVariant::Vertical => {
                            let (_, dr) = direction.delta();
                            let next = cell.offset(0, dr);
                            if self.is_blocked_for_enemy(next) {
                                direction = direction.opposite();
                                cell.offset(0, direction.delta().1)
                            } else {
                                next
                            }
                        }
                        BirdVariant::Firing => {
                            bird_shots.push((*cell, state.shot_direction));
                            *cell
                        }
                    }
                }
                ElementKind::Butterfly => self.butterfly_step(*cell, id),
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
            self.shoot_from_cell(from, dir, events);
        }
    }

    fn butterfly_step(&mut self, cell: Cell, id: u32) -> Cell {
        let robbo = self.robbo_cell();
        let dirs = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        if roll_one_in_eight(&mut self.rng_state) {
            let pick = (self.tick as usize + id as usize) % dirs.len();
            let d = dirs[pick];
            let (dc, dr) = d.delta();
            let next = cell.offset(dc, dr);
            if !self.is_blocked_for_enemy(next) {
                return next;
            }
            return cell;
        }
        let Some(robbo) = robbo else {
            return cell;
        };
        let h_first = (self.tick + id as u64) & 1 == 0;
        let try_dirs: [Direction; 2] = if h_first {
            if robbo.col != cell.col {
                if robbo.col > cell.col {
                    [Direction::Right, Direction::Left]
                } else {
                    [Direction::Left, Direction::Right]
                }
            } else if robbo.row > cell.row {
                [Direction::Down, Direction::Up]
            } else {
                [Direction::Up, Direction::Down]
            }
        } else if robbo.row != cell.row {
            if robbo.row > cell.row {
                [Direction::Down, Direction::Up]
            } else {
                [Direction::Up, Direction::Down]
            }
        } else if robbo.col > cell.col {
            [Direction::Right, Direction::Left]
        } else {
            [Direction::Left, Direction::Right]
        };
        for d in try_dirs {
            let (dc, dr) = d.delta();
            let next = cell.offset(dc, dr);
            if !self.is_blocked_for_enemy(next) {
                return next;
            }
        }
        cell
    }

    fn wall_follow_with_dir(
        &self,
        cell: Cell,
        dir: Direction,
        clockwise: bool,
    ) -> (Cell, Direction) {
        let forward = cell.offset(dir.delta().0, dir.delta().1);
        let turn = if clockwise {
            dir.turn_right()
        } else {
            dir.turn_left()
        };
        let turned = cell.offset(turn.delta().0, turn.delta().1);

        if !self.is_blocked_for_enemy(forward) {
            (forward, dir)
        } else if !self.is_blocked_for_enemy(turned) {
            (turned, turn)
        } else {
            (cell, dir)
        }
    }
}
