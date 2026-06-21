use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState};
use crate::events::GameEvent;
use crate::world::World;

impl World {
    pub(crate) fn fire_laser(
        &mut self,
        from: Cell,
        direction: Direction,
        source_id: Option<u32>,
        events: &mut Vec<GameEvent>,
    ) {
        events.push(GameEvent::Shot { from, direction });
        let (dc, dr) = direction.delta();
        let mut cell = from.offset(dc, dr);
        while self.in_bounds(cell) {
            if self.tile_at(cell).is_some_and(|t| t.blocks_shot()) {
                break;
            }
            if let Some((_, existing)) = self.element_at(cell) {
                if matches!(existing.kind, ElementKind::Laser { .. }) {
                    cell = cell.offset(dc, dr);
                    continue;
                }
                if !Self::is_laser_immune(&existing.kind) {
                    self.destroy_at(cell, events);
                }
                break;
            }
            let laser_id = self.allocate_id();
            self.elements.push((
                cell,
                ElementState::new(
                    laser_id,
                    ElementKind::Laser {
                        direction,
                        source_id,
                    },
                    direction,
                ),
            ));
            cell = cell.offset(dc, dr);
        }
    }

    pub(crate) fn fire_blaster(
        &mut self,
        from: Cell,
        direction: Direction,
        events: &mut Vec<GameEvent>,
    ) {
        events.push(GameEvent::Shot { from, direction });
        let (dc, dr) = direction.delta();
        let mut cell = from.offset(dc, dr);
        let mut frame = 0u8;
        while self.in_bounds(cell) && frame < 5 {
            if self.tile_at(cell).is_some_and(|t| t.blocks_shot()) {
                break;
            }
            if let Some((_, el)) = self.element_at(cell) {
                if Self::is_blaster_immune(&el.kind) {
                    break;
                }
                if matches!(el.kind, ElementKind::Bomb) {
                    self.schedule_bomb_detonation(cell);
                } else {
                    self.destroy_at(cell, events);
                }
                break;
            }
            let blaster_id = self.allocate_id();
            self.elements.push((
                cell,
                ElementState::new(
                    blaster_id,
                    ElementKind::BlasterCell {
                        direction,
                        frame,
                    },
                    direction,
                ),
            ));
            cell = cell.offset(dc, dr);
            frame += 1;
        }
    }

    pub(crate) fn tick_lasers_and_blasters(&mut self, events: &mut Vec<GameEvent>) {
        // Remove orphan lasers whose gun source is gone.
        let gun_ids: std::collections::HashSet<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Gun { .. }))
            .map(|(_, s)| s.id)
            .collect();

        let orphan_lasers: Vec<u32> = self
            .elements
            .iter()
            .filter_map(|(_, s)| {
                if let ElementKind::Laser { source_id, .. } = s.kind {
                    if let Some(gid) = source_id {
                        if !gun_ids.contains(&gid) {
                            return Some(s.id);
                        }
                    }
                }
                None
            })
            .collect();
        for id in orphan_lasers {
            self.remove_element_by_id(id);
        }

        // Blaster cells: destroy on contact, tick frames.
        let blaster_ids: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::BlasterCell { .. }))
            .map(|(_, s)| s.id)
            .collect();
        for id in blaster_ids {
            if let Some((i, _)) = self.element_by_id(id) {
                let cell = self.elements[i].0;
                if self.robbo_cell() == Some(cell) {
                    self.kill_robbo(crate::events::DeathCause::Projectile, events);
                }
                if let ElementKind::BlasterCell { frame, .. } = &mut self.elements[i].1.kind {
                    *frame += 1;
                    if *frame >= 5 {
                        self.remove_element_by_id(id);
                    }
                }
            }
        }

        // Laser kills robbo on beam.
        let laser_cells: Vec<Cell> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Laser { .. }))
            .map(|(c, _)| *c)
            .collect();
        let robbo = self.robbo_cell();
        if laser_cells.iter().any(|c| robbo == Some(*c)) {
            self.kill_robbo(crate::events::DeathCause::Projectile, events);
        }
    }
}
