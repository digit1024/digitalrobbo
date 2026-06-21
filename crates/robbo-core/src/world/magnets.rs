use crate::element::ElementKind;
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    pub(crate) fn tick_magnets(&mut self, _events: &mut Vec<GameEvent>) {
        if self.robbo_magnet_locked {
            return;
        }

        let Some(robbo_idx) = self
            .elements
            .iter()
            .position(|(_, s)| matches!(s.kind, ElementKind::Robbo))
        else {
            return;
        };
        let _robbo_cell = self.elements[robbo_idx].0;

        for (mag_cell, state) in self.elements.iter() {
            let ElementKind::Magnet { direction } = state.kind else {
                continue;
            };
            let (dc, dr) = direction.delta();
            let mut cursor = *mag_cell;
            loop {
                let next = cursor.offset(dc, dr);
                if !self.in_bounds(next) {
                    break;
                }
                if self.tile_at(next).is_some_and(|t| t.blocks_movement()) {
                    break;
                }
                if let Some((_, el)) = self.element_at(next) {
                    if matches!(el.kind, ElementKind::Robbo) {
                        self.robbo_magnet_locked = true;
                        self.magnet_pull_dir = direction.opposite();
                        return;
                    }
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
