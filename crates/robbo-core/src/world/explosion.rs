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

        let mut chain_cells = Vec::new();
        for n in &affected {
            if self.robbo_cell() == Some(*n) {
                self.kill_robbo(DeathCause::Explosion, events);
            }
            if let Some((_, state)) = self.element_at(*n) {
                if matches!(state.kind, ElementKind::Bomb) {
                    chain_cells.push(*n);
                }
            }
        }

        // Remove destroyable objects; question marks do not reveal on bomb.
        let to_remove: Vec<u32> = self
            .elements
            .iter()
            .filter(|(c, s)| {
                affected.contains(c)
                    && !matches!(
                        s.kind,
                        ElementKind::Projectile { .. } | ElementKind::Robbo
                    )
            })
            .map(|(_, s)| s.id)
            .collect();
        for id in to_remove {
            self.remove_element_by_id(id);
        }

        for cell in chain_cells {
            self.explode_at_inner(cell, events, depth + 1);
        }
    }
}
