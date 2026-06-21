use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::ElementKind;
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    /// Cells illuminated by a magnet beam (gnurobbo: stops at walls and any object).
    pub fn magnet_beam_cells(&self, origin: Cell, direction: Direction) -> Vec<Cell> {
        let (dc, dr) = direction.delta();
        let mut path = Vec::new();
        let mut cursor = origin;
        loop {
            let next = cursor.offset(dc, dr);
            if !self.in_bounds(next) {
                break;
            }
            if self.tile_at(next).is_some_and(|t| t.blocks_movement()) {
                break;
            }
            path.push(next);
            if self.element_at(next).is_some() {
                break;
            }
            cursor = next;
        }
        path
    }

    /// gnurobbo board scan: row-major (y, then x). First magnet to hit Robbo wins.
    fn magnets_in_scan_order(&self) -> Vec<(Cell, Direction)> {
        let mut magnets: Vec<(Cell, Direction)> = self
            .elements
            .iter()
            .filter_map(|(cell, state)| match state.kind {
                ElementKind::Magnet { direction } => Some((*cell, direction)),
                _ => None,
            })
            .collect();
        magnets.sort_by_key(|(cell, _)| (cell.row, cell.col));
        magnets
    }

    pub(crate) fn tick_magnets(&mut self, _events: &mut Vec<GameEvent>) {
        if self.robbo_magnet_locked {
            return;
        }

        let Some(robbo_cell) = self.robbo_cell() else {
            return;
        };

        for (mag_cell, direction) in self.magnets_in_scan_order() {
            let (dc, dr) = direction.delta();
            let mut cursor = mag_cell;
            loop {
                let next = cursor.offset(dc, dr);
                if !self.in_bounds(next) {
                    break;
                }
                if self.tile_at(next).is_some_and(|t| t.blocks_movement()) {
                    break;
                }
                if next == robbo_cell {
                    self.robbo_magnet_locked = true;
                    self.magnet_pull_dir = direction.opposite();
                    return;
                }
                if self.element_at(next).is_some() {
                    break;
                }
                cursor = next;
            }
        }
    }

    pub(crate) fn magnet_pull_robbo(&mut self, events: &mut Vec<GameEvent>) {
        let Some(robbo_idx) = self
            .elements
            .iter()
            .position(|(_, s)| matches!(s.kind, ElementKind::Robbo))
        else {
            return;
        };
        let robbo_id = self.elements[robbo_idx].1.id;
        let from = self.elements[robbo_idx].0;
        let (dc, dr) = self.magnet_pull_dir.delta();
        let pull = from.offset(dc, dr);

        if !self.can_robbo_stand(pull) {
            self.kill_robbo(DeathCause::Hazard, events);
            return;
        }

        self.elements[robbo_idx].0 = pull;
        events.push(GameEvent::Moved {
            entity_id: robbo_id,
            from,
            to: pull,
        });
    }
}
