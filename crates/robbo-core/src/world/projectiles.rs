use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{push_box_slide_delay, ElementKind, GunType};
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    pub(crate) fn tick_projectiles(&mut self, _events: &mut Vec<GameEvent>) {
        // Player/enemy bolts are ElementKind::Laser; nothing to tick here.
    }

    pub(crate) fn tick_push_boxes(&mut self, events: &mut Vec<GameEvent>) {
        let sliding: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::PushBox) && s.sliding)
            .map(|(_, s)| s.id)
            .collect();

        let delay = push_box_slide_delay();

        for id in sliding {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            if self.elements[i].1.tick_counter > 0 {
                self.elements[i].1.tick_counter -= 1;
                if self.elements[i].1.tick_counter > 0 {
                    continue;
                }
            }

            let cell = self.elements[i].0;
            let dir = self.elements[i].1.direction;
            let (dc, dr) = dir.delta();
            let next = cell.offset(dc, dr);

            if !self.in_bounds(next) {
                self.elements[i].1.sliding = false;
                continue;
            }

            if self.is_blocked(next) || self.element_at(next).is_some() {
                self.elements[i].1.sliding = false;
                self.push_box_stop_shoot(cell, next, dir, events);
                continue;
            }

            self.elements[i].0 = next;
            self.elements[i].1.tick_counter = delay;
            events.push(GameEvent::Moved {
                entity_id: id,
                from: cell,
                to: next,
            });
        }
    }

    /// gnurobbo `shoot_object` when a sliding push box hits a non-empty cell.
    pub(crate) fn push_box_stop_shoot(
        &mut self,
        from: Cell,
        target: Cell,
        dir: Direction,
        events: &mut Vec<GameEvent>,
    ) {
        if self.robbo_cell() == Some(target) {
            self.kill_robbo(DeathCause::Projectile, events);
            return;
        }

        if let Some((_, el)) = self.element_at(target) {
            if matches!(el.kind, ElementKind::Laser { .. }) {
                return;
            }
            if Self::is_shot_destroyable(&el.kind) || matches!(el.kind, ElementKind::Bomb) {
                self.destroy_at(target, events);
            }
            return;
        }

        if self
            .tile_at(target)
            .is_some_and(Self::is_tile_shot_destroyable)
        {
            self.clear_blowable_tile(target, events);
            return;
        }

        if !self.is_blocked(target) {
            self.gun_shoot(from, dir, GunType::Regular, None, events, true);
        }
    }
}
