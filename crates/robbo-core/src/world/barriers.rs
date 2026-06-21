use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::ElementKind;
use crate::events::DeathCause;
use crate::tile::TileKind;
use crate::events::GameEvent;
use crate::world::World;

impl World {
    pub(crate) fn tick_barriers(&mut self, events: &mut Vec<GameEvent>) {
        if self.tick % 4 != 0 {
            return;
        }

        let barrier_cells: Vec<Cell> = self
            .barrier_directions
            .keys()
            .copied()
            .collect();

        for start in barrier_cells {
            let Some(&dir) = self.barrier_directions.get(&start) else {
                continue;
            };
            if dir == Direction::Right {
                self.shift_barriers_east(start.row, events);
            } else if dir == Direction::Left {
                self.shift_barriers_west(start.row, events);
            }
        }
    }

    fn shift_barriers_east(&mut self, row: i16, events: &mut Vec<GameEvent>) {
        let mut x = self.width as i16 - 1;
        while x >= 0 {
            let cell = Cell::new(x, row);
            if self.tile_at(cell) == Some(TileKind::WallGrey)
                || self.tile_at(cell) == Some(TileKind::WallBlack)
                || self.tile_at(cell) == Some(TileKind::WallGreen)
                || self.tile_at(cell) == Some(TileKind::WallRed)
                || self.tile_at(cell) == Some(TileKind::WallSolid)
            {
                break;
            }
            if self.tile_at(cell) == Some(TileKind::Barrier) {
                let dest = Cell::new(x + 1, row);
                if self.robbo_cell() == Some(dest) {
                    self.kill_robbo(DeathCause::Hazard, events);
                    return;
                }
                self.consume_cell_contents(cell, events);
                self.set_tile(cell, TileKind::Empty);
                self.barrier_directions.remove(&cell);
                if self.in_bounds(dest) && self.tile_at(dest) == Some(TileKind::Empty) {
                    self.set_tile(dest, TileKind::Barrier);
                    self.barrier_directions.insert(dest, Direction::Right);
                }
            }
            x -= 1;
        }
    }

    fn shift_barriers_west(&mut self, row: i16, events: &mut Vec<GameEvent>) {
        let mut x = 0i16;
        while x < self.width as i16 {
            let cell = Cell::new(x, row);
            if self.tile_at(cell).is_some_and(|t| {
                matches!(
                    t,
                    TileKind::WallGrey
                        | TileKind::WallBlack
                        | TileKind::WallGreen
                        | TileKind::WallRed
                        | TileKind::WallSolid
                )
            }) {
                break;
            }
            if self.tile_at(cell) == Some(TileKind::Barrier) {
                let dest = Cell::new(x - 1, row);
                if self.robbo_cell() == Some(dest) {
                    self.kill_robbo(DeathCause::Hazard, events);
                    return;
                }
                self.consume_cell_contents(cell, events);
                self.set_tile(cell, TileKind::Empty);
                self.barrier_directions.remove(&cell);
                if self.in_bounds(dest) && self.tile_at(dest) == Some(TileKind::Empty) {
                    self.set_tile(dest, TileKind::Barrier);
                    self.barrier_directions.insert(dest, Direction::Left);
                }
            }
            x += 1;
        }
    }

    fn consume_cell_contents(&mut self, cell: Cell, events: &mut Vec<GameEvent>) {
        if let Some((_, state)) = self.element_at(cell) {
            let id = state.id;
            if !matches!(state.kind, ElementKind::Robbo) {
                if matches!(state.kind, ElementKind::Bomb) {
                    self.explode_at(cell, events);
                } else {
                    self.remove_element_by_id(id);
                }
            }
        }
    }
}
