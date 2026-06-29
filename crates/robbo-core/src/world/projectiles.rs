use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{push_box_slide_delay, ElementKind};
use crate::events::GameEvent;
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
                self.shoot_from_cell(cell, dir, events);
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

    pub(crate) fn shoot_from_cell(&mut self, from: Cell, dir: Direction, events: &mut Vec<GameEvent>) {
        self.gun_shoot(from, dir, crate::element::GunType::Regular, None, events, true);
    }
}
