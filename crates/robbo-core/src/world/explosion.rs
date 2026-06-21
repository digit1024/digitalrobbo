use std::collections::HashSet;

use crate::cell::Cell;
use crate::element::ElementKind;
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    pub(crate) fn explode_at(&mut self, at: Cell, events: &mut Vec<GameEvent>) {
        self.explode_at_inner(at, events, 0);
    }

    fn explode_at_inner(&mut self, at: Cell, events: &mut Vec<GameEvent>, depth: u32) {
        if depth > 16 || !self.in_bounds(at) {
            return;
        }
        events.push(GameEvent::Exploded { at });

        let mut affected = HashSet::new();
        for dc in -1..=1 {
            for dr in -1..=1 {
                let n = at.offset(dc, dr);
                if self.in_bounds(n) {
                    affected.insert(n);
                }
            }
        }

        let mut chain_bombs = Vec::new();
        for cell in affected.iter().copied().collect::<Vec<_>>() {
            if self.robbo_cell() == Some(cell) {
                self.kill_robbo(DeathCause::Explosion, events);
            }

            if let Some((_, state)) = self.element_at(cell) {
                // Solid laser beams are not blowable (gnurobbo `blow_bomb`).
                if matches!(
                    state.kind,
                    ElementKind::Laser {
                        solid: true,
                        ..
                    }
                ) {
                    continue;
                }
                if matches!(state.kind, ElementKind::Bomb) {
                    if !chain_bombs.contains(&cell) {
                        chain_bombs.push(cell);
                    }
                    continue;
                }
                if Self::is_blowable(&state.kind) {
                    self.remove_element_by_id(state.id);
                }
            }

            if let Some(tile) = self.tile_at(cell) {
                if Self::is_tile_blowable(tile) {
                    self.clear_blowable_tile(cell, events);
                }
            }
        }

        for cell in chain_bombs {
            if let Some((_, state)) = self.element_at(cell) {
                if matches!(state.kind, ElementKind::Bomb) {
                    self.remove_element_by_id(state.id);
                }
            }
            self.explode_at_inner(cell, events, depth + 1);
        }
    }
}
