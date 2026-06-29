use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState, GunType};
use crate::events::{DeathCause, GameEvent};
use crate::world::World;

impl World {
    fn place_laser_segment(
        &mut self,
        at: Cell,
        direction: Direction,
        solid: bool,
        source_id: Option<u32>,
    ) {
        if self.element_at(at).is_some() {
            return;
        }
        let lid = self.allocate_id();
        self.elements.push((
            at,
            ElementState::new(
                lid,
                ElementKind::Laser {
                    direction,
                    source_id,
                    solid,
                    returning: false,
                },
                direction,
            ),
        ));
    }

    fn place_blaster_cell(&mut self, at: Cell, direction: Direction) {
        if self.element_at(at).is_some() {
            return;
        }
        let bid = self.allocate_id();
        self.elements.push((
            at,
            ElementState::new(
                bid,
                ElementKind::BlasterCell {
                    direction,
                    frame: 0,
                },
                direction,
            ),
        ));
    }

    /// gnurobbo `shoot_object` — spawn one laser bolt / solid segment, or destroy target.
    pub(crate) fn gun_shoot(
        &mut self,
        from: Cell,
        direction: Direction,
        gun_type: GunType,
        source_id: Option<u32>,
        events: &mut Vec<GameEvent>,
        emit_shot: bool,
    ) {
        let (dc, dr) = direction.delta();
        let target = from.offset(dc, dr);

        if !self.in_bounds(target) {
            return;
        }
        if self.tile_at(target).is_some_and(|t| t.blocks_shot()) {
            return;
        }
        if self.robbo_cell() == Some(target) {
            self.kill_robbo(DeathCause::Projectile, events);
            return;
        }

        if let Some((_, el)) = self.element_at(target) {
            if matches!(el.kind, ElementKind::Laser { .. }) {
                return;
            }

            if gun_type == GunType::Blaster {
                if Self::is_blaster_immune(&el.kind) {
                    return;
                }
                if matches!(el.kind, ElementKind::Bomb) {
                    self.schedule_bomb_detonation(target);
                    return;
                }
                self.destroy_at(target, events);
                self.place_blaster_cell(target, direction);
                return;
            }

            if Self::is_laser_shot_immune(&el.kind) {
                return;
            }
            if Self::is_laser_destroyable(&el.kind) {
                self.destroy_at(target, events);
                return;
            }
            return;
        }

        if self
            .tile_at(target)
            .is_some_and(Self::is_tile_shot_destroyable)
        {
            self.clear_blowable_tile(target, events);
        }

        match gun_type {
            GunType::Blaster => self.place_blaster_cell(target, direction),
            GunType::Laser => self.place_laser_segment(target, direction, true, source_id),
            GunType::Regular => self.place_laser_segment(target, direction, false, source_id),
        }
        if emit_shot {
            events.push(GameEvent::Shot {
                from,
                direction,
                gun_type,
            });
        }
    }

    pub(crate) fn tick_lasers_and_blasters(&mut self, events: &mut Vec<GameEvent>) {
        self.tick_moving_lasers(events);
        self.tick_solid_lasers(events);
        self.tick_blaster_cells(events);
        self.laser_contact_damage(events);
    }

    fn tick_moving_lasers(&mut self, events: &mut Vec<GameEvent>) {
        let bolts: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| {
                matches!(
                    s.kind,
                    ElementKind::Laser {
                        solid: false,
                        ..
                    }
                )
            })
            .map(|(_, s)| s.id)
            .collect();

        for id in bolts {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            let cell = self.elements[i].0;
            let direction = match self.elements[i].1.kind {
                ElementKind::Laser { direction, .. } => direction,
                _ => continue,
            };
            let (dc, dr) = direction.delta();
            let next = cell.offset(dc, dr);

            if !self.in_bounds(next) || self.tile_at(next).is_some_and(|t| t.blocks_shot()) {
                self.remove_element_by_id(id);
                continue;
            }
            if self.robbo_cell() == Some(next) {
                self.kill_robbo(DeathCause::Projectile, events);
                self.remove_element_by_id(id);
                continue;
            }
            if let Some((_, el)) = self.element_at(next) {
                if Self::is_laser_destroyable(&el.kind) {
                    self.destroy_at(next, events);
                }
                self.remove_element_by_id(id);
                continue;
            }
            if self
                .tile_at(next)
                .is_some_and(Self::is_tile_shot_destroyable)
            {
                self.clear_blowable_tile(next, events);
                self.remove_element_by_id(id);
                continue;
            }

            self.elements[i].0 = next;
            events.push(GameEvent::Moved {
                entity_id: id,
                from: cell,
                to: next,
            });
        }
    }

    fn tick_solid_lasers(&mut self, events: &mut Vec<GameEvent>) {
        let gun_ids: std::collections::HashSet<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Gun { .. }))
            .map(|(_, s)| s.id)
            .collect();

        let orphans: Vec<u32> = self
            .elements
            .iter()
            .filter_map(|(_, s)| {
                if let ElementKind::Laser {
                    solid: true,
                    source_id: Some(gid),
                    ..
                } = s.kind
                {
                    if !gun_ids.contains(&gid) {
                        return Some(s.id);
                    }
                }
                None
            })
            .collect();
        for id in orphans {
            self.remove_element_by_id(id);
        }

        let solid: Vec<(u32, Cell, Direction, bool, Option<u32>)> = self
            .elements
            .iter()
            .filter_map(|(c, s)| {
                if let ElementKind::Laser {
                    direction,
                    returning,
                    source_id,
                    solid: true,
                    ..
                } = s.kind
                {
                    Some((s.id, *c, direction, returning, source_id))
                } else {
                    None
                }
            })
            .collect();

        for (id, cell, direction, returning, _) in &solid {
            if *returning {
                self.retract_returning_solid_laser(*id, *cell, *direction);
            }
        }

        for (id, cell, direction, was_returning, source_id) in solid {
            if was_returning || self.solid_laser_returning(id) {
                continue;
            }

            let (dc, dr) = direction.delta();
            let next = cell.offset(dc, dr);

            if self.robbo_cell() == Some(next) {
                self.kill_robbo(DeathCause::Projectile, events);
                continue;
            }

            if !self.in_bounds(next) || self.tile_at(next).is_some_and(|t| t.blocks_shot()) {
                self.set_laser_returning(id);
                continue;
            }

            if let Some((_, el)) = self.element_at(next) {
                if let ElementKind::Laser {
                    direction: other_dir,
                    solid: true,
                    ..
                } = el.kind
                {
                    // gnurobbo board.c: same-direction beams stop; opposing/crossing retract.
                    if other_dir != direction {
                        self.set_laser_returning(id);
                    }
                    continue;
                }
                if Self::is_laser_destroyable(&el.kind) {
                    self.destroy_at(next, events);
                    continue;
                }
                if Self::is_laser_shot_immune(&el.kind) {
                    self.set_laser_returning(id);
                }
                continue;
            }

            if self
                .tile_at(next)
                .is_some_and(Self::is_tile_shot_destroyable)
            {
                self.clear_blowable_tile(next, events);
            }

            self.place_laser_segment(next, direction, true, source_id);
        }
    }

    /// gnurobbo `returnlaser == 1`: clear this segment and ripple retraction toward the gun.
    fn retract_returning_solid_laser(&mut self, id: u32, cell: Cell, direction: Direction) {
        let behind = cell.offset(direction.opposite().delta().0, direction.opposite().delta().1);
        let behind_kind = self
            .element_at(behind)
            .map(|(_, s)| s.kind.clone());

        self.remove_element_by_id(id);

        match behind_kind {
            Some(ElementKind::Laser {
                solid: true,
                direction: behind_dir,
                ..
            }) if behind_dir == direction => {
                if let Some((_, s)) = self.element_at(behind) {
                    self.set_laser_returning(s.id);
                }
            }
            _ => {}
        }
    }

    fn solid_laser_returning(&self, id: u32) -> bool {
        self.element_by_id(id).is_some_and(|(_, s)| {
            matches!(
                s.kind,
                ElementKind::Laser {
                    solid: true,
                    returning: true,
                    ..
                }
            )
        })
    }

    fn set_laser_returning(&mut self, id: u32) {
        if let Some((i, _)) = self.element_by_id(id) {
            if let ElementKind::Laser {
                ref mut returning, ..
            } = self.elements[i].1.kind
            {
                *returning = true;
            }
        }
    }

    fn tick_blaster_cells(&mut self, events: &mut Vec<GameEvent>) {
        let blasters: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::BlasterCell { .. }))
            .map(|(_, s)| s.id)
            .collect();

        for id in blasters {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            let (cell, direction, frame) = match self.elements[i].1.kind {
                ElementKind::BlasterCell { direction, frame } => {
                    (self.elements[i].0, direction, frame)
                }
                _ => continue,
            };

            if frame >= 4 {
                self.remove_element_by_id(id);
                continue;
            }

            let (dc, dr) = direction.delta();
            let next = cell.offset(dc, dr);

            if self.robbo_cell() == Some(next) {
                self.kill_robbo(DeathCause::Projectile, events);
                return;
            }

            // gnurobbo: blaster extends one cell only at state/frame 0.
            if frame == 0
                && self.in_bounds(next)
                && !self.tile_at(next).is_some_and(|t| t.blocks_shot())
            {
                if let Some((_, el)) = self.element_at(next) {
                    if Self::is_blaster_immune(&el.kind) {
                        // stop propagation
                    } else if matches!(el.kind, ElementKind::Bomb) {
                        self.schedule_bomb_detonation(next);
                    } else {
                        self.destroy_at(next, events);
                        self.place_blaster_cell(next, direction);
                    }
                } else {
                    self.destroy_at(next, events);
                    self.place_blaster_cell(next, direction);
                }
            }

            if let Some((i, _)) = self.element_by_id(id) {
                if let ElementKind::BlasterCell { frame, .. } = &mut self.elements[i].1.kind {
                    *frame += 1;
                }
            }
        }
    }

    fn laser_contact_damage(&mut self, events: &mut Vec<GameEvent>) {
        let robbo = self.robbo_cell();
        let on_beam = self.elements.iter().any(|(c, s)| {
            robbo == Some(*c)
                && matches!(
                    s.kind,
                    ElementKind::Laser {
                        solid: true,
                        ..
                    }
                )
        });
        if on_beam {
            self.kill_robbo(DeathCause::Projectile, events);
        }
    }

    pub(crate) fn gun_barrel_blocked(&self, gun_cell: Cell, direction: Direction) -> bool {
        let (dc, dr) = direction.delta();
        let next = gun_cell.offset(dc, dr);
        self.element_at(next).is_some_and(|(_, el)| {
            matches!(
                el.kind,
                ElementKind::Laser { .. } | ElementKind::BlasterCell { .. }
            )
        })
    }
}
