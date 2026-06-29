use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState, GunType, next_rand, roll_one_in_eight};
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    /// gnurobbo `is_robbo_killed()` — orthogonal adjacency to killing enemies.
    pub(crate) fn check_adjacent_enemy_kill(&mut self, events: &mut Vec<GameEvent>) {
        let Some(robbo) = self.robbo_cell() else {
            return;
        };
        for (dc, dr) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let neighbor = robbo.offset(dc, dr);
            if let Some((_, el)) = self.element_at(neighbor) {
                if Self::is_killing_enemy(&el.kind) {
                    self.kill_robbo(DeathCause::Enemy, events);
                    return;
                }
            }
        }
    }

    pub(crate) fn tick_enemies(&mut self, events: &mut Vec<GameEvent>) {
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
            let delay = match &state.kind {
                ElementKind::Bear { .. }
                | ElementKind::BlackBear { .. }
                | ElementKind::Bird { .. }
                | ElementKind::Butterfly => crate::element::enemy_move_delay(&state.kind),
                ElementKind::PushBox => continue,
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
                ElementKind::Bear { clockwise }
                | ElementKind::BlackBear { clockwise } => {
                    let (next, new_dir) =
                        self.bear_step(*cell, direction, *clockwise, self.sensible_bears);
                    direction = new_dir;
                    next
                }
                ElementKind::Bird { shooting, .. } => {
                    let (dc, dr) = direction.delta();
                    let next = cell.offset(dc, dr);
                    let pos = if !self.is_blocked_for_enemy(next) {
                        next
                    } else {
                        direction = direction.opposite();
                        *cell
                    };
                    if *shooting && roll_one_in_eight(&mut self.rng_state) {
                        bird_shots.push((pos, state.shot_direction));
                    }
                    pos
                }
                ElementKind::Butterfly => {
                    let moved = self.butterfly_move(*cell, direction);
                    direction = self.butterfly_next_direction(moved, direction);
                    moved
                }
                _ => *cell,
            };

            let mv = if new_cell != *cell { Some(new_cell) } else { None };
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
            self.gun_shoot(from, dir, GunType::Regular, None, events, true);
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
        !self.is_blocked_for_enemy(cell)
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

    /// gnurobbo `BUTTERFLY` direction update after move (`board.c`).
    fn butterfly_next_direction(&mut self, cell: Cell, current: Direction) -> Direction {
        if roll_one_in_eight(&mut self.rng_state) {
            return Direction::from_gnurobbo((next_rand(&mut self.rng_state) & 3) as u8);
        }
        let Some(robbo) = self.robbo_cell() else {
            return current;
        };
        if (next_rand(&mut self.rng_state) & 1) == 0 {
            if robbo.col > cell.col {
                Direction::Right
            } else if robbo.col < cell.col {
                Direction::Left
            } else {
                current
            }
        } else if robbo.row > cell.row {
            Direction::Down
        } else if robbo.row < cell.row {
            Direction::Up
        } else {
            current
        }
    }
}
