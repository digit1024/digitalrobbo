use crate::cell::Cell;
use crate::element::{bomb_chain_delay_ticks, ElementKind};
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    /// Detonate a bomb now — 3×3 blast; neighbor bombs are queued, not exploded instantly.
    pub(crate) fn detonate_bomb(&mut self, at: Cell, events: &mut Vec<GameEvent>) {
        if !self.in_bounds(at) {
            return;
        }
        let Some((_, state)) = self.element_at(at) else {
            return;
        };
        if !matches!(state.kind, ElementKind::Bomb) {
            return;
        }

        events.push(GameEvent::Exploded { at });
        self.remove_element_by_id(state.id);
        self.apply_bomb_wave(at, events);
    }

    /// Legacy entry — same as [`detonate_bomb`] (no recursive same-tick chain).
    pub(crate) fn explode_at(&mut self, at: Cell, events: &mut Vec<GameEvent>) {
        if self
            .element_at(at)
            .is_some_and(|(_, s)| matches!(s.kind, ElementKind::Bomb))
        {
            self.detonate_bomb(at, events);
        } else {
            // Non-bomb center (shouldn't happen in normal play) — still apply wave.
            events.push(GameEvent::Exploded { at });
            self.apply_bomb_wave(at, events);
        }
    }

    /// gnurobbo `blow_bomb` — damage 3×3; adjacent bombs get `DELAY_BOMB_TARGET`.
    fn apply_bomb_wave(&mut self, at: Cell, events: &mut Vec<GameEvent>) {
        let chain_delay = bomb_chain_delay_ticks();

        for dc in -1..=1 {
            for dr in -1..=1 {
                let cell = at.offset(dc, dr);
                if !self.in_bounds(cell) {
                    continue;
                }

                if self.robbo_cell() == Some(cell) {
                    self.kill_robbo(DeathCause::Explosion, events);
                }

                if let Some((_, state)) = self.element_at(cell) {
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
                        self.queue_bomb_detonation(cell, chain_delay);
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
        }
    }
}
