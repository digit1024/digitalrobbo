use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState, GunType};
use crate::events::{DeathCause, GameEvent};
use crate::tile::TileKind;
use crate::world::World;

impl World {
    /// gnurobbo `shoot_object` — spawn one laser bolt / solid segment, or destroy target.
    pub(crate) fn gun_shoot(
        &mut self,
        from: Cell,
        direction: Direction,
        gun_type: GunType,
        source_id: Option<u32>,
        events: &mut Vec<GameEvent>,
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
            if gun_type == GunType::Blaster {
                if Self::is_blaster_immune(&el.kind) {
                    return;
                }
                if matches!(el.kind, ElementKind::Bomb) {
                    self.schedule_bomb_detonation(target);
                    return;
                }
                self.destroy_at(target, events);
                let bid = self.allocate_id();
                self.elements.push((
                    target,
                    ElementState::new(
                        bid,
                        ElementKind::BlasterCell {
                            direction,
                            frame: 0,
                        },
                        direction,
                    ),
                ));
                return;
            }

            if Self::is_laser_shot_immune(&el.kind) {
                return;
            }
            if Self::is_laser_destroyable(&el.kind)
                || self.tile_at(target) == Some(TileKind::Ground)
            {
                self.destroy_at(target, events);
                return;
            }
            return;
        }

        if self.tile_at(target) == Some(TileKind::Ground) {
            self.set_tile(target, TileKind::Empty);
            return;
        }

        match gun_type {
            GunType::Blaster => {
                let bid = self.allocate_id();
                self.elements.push((
                    target,
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
            GunType::Laser => {
                let lid = self.allocate_id();
                self.elements.push((
                    target,
                    ElementState::new(
                        lid,
                        ElementKind::Laser {
                            direction,
                            source_id,
                            solid: true,
                            returning: false,
                        },
                        direction,
                    ),
                ));
            }
            GunType::Regular => {
                let lid = self.allocate_id();
                self.elements.push((
                    target,
                    ElementState::new(
                        lid,
                        ElementKind::Laser {
                            direction,
                            source_id,
                            solid: false,
                            returning: false,
                        },
                        direction,
                    ),
                ));
            }
        }
        events.push(GameEvent::Shot { from, direction });
    }

    pub(crate) fn spawn_moving_laser(
        &mut self,
        at: Cell,
        direction: Direction,
        from_player: bool,
        events: &mut Vec<GameEvent>,
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
                    source_id: None,
                    solid: false,
                    returning: false,
                },
                direction,
            ),
        ));
        if from_player {
            events.push(GameEvent::Shot {
                from: at.offset(-direction.delta().0, -direction.delta().1),
                direction,
            });
        }
    }

    pub(crate) fn tick_lasers_and_blasters(&mut self, events: &mut Vec<GameEvent>) {
        if self.tick % 4 != 0 {
            // Still check robbo contact every tick.
            self.laser_contact_damage(events);
            return;
        }

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
                if Self::is_laser_destroyable(&el.kind)
                    || self.tile_at(next) == Some(TileKind::Ground)
                {
                    self.destroy_at(next, events);
                }
                self.remove_element_by_id(id);
                continue;
            }
            if self.tile_at(next) == Some(TileKind::Ground) {
                self.set_tile(next, TileKind::Empty);
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

        // Solid beam extension at returning segments (simplified gnurobbo returnlaser).
        let solid_ids: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| {
                matches!(
                    s.kind,
                    ElementKind::Laser {
                        solid: true,
                        ..
                    }
                )
            })
            .map(|(_, s)| s.id)
            .collect();

        for id in solid_ids {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            let (cell, direction, returning) = match self.elements[i].1.kind {
                ElementKind::Laser {
                    direction,
                    returning,
                    ..
                } => (self.elements[i].0, direction, returning),
                _ => continue,
            };
            if !returning {
                continue;
            }
            let (dc, dr) = direction.delta();
            let next = cell.offset(dc, dr);
            if !self.in_bounds(next) || self.is_laser_extend_blocked(next) {
                if let Some((i, _)) = self.element_by_id(id) {
                    if let ElementKind::Laser {
                        ref mut returning, ..
                    } = self.elements[i].1.kind
                    {
                        *returning = false;
                    }
                }
                continue;
            }
            if self.element_at(next).is_none() {
                let lid = self.allocate_id();
                let source_id = match self.elements[i].1.kind {
                    ElementKind::Laser { source_id, .. } => source_id,
                    _ => None,
                };
                self.elements.push((
                    next,
                    ElementState::new(
                        lid,
                        ElementKind::Laser {
                            direction,
                            source_id,
                            solid: true,
                            returning: false,
                        },
                        direction,
                    ),
                ));
            }
            if let Some((i, _)) = self.element_by_id(id) {
                if let ElementKind::Laser {
                    ref mut returning, ..
                } = self.elements[i].1.kind
                {
                    *returning = false;
                }
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

            if self.in_bounds(next)
                && !self.tile_at(next).is_some_and(|t| t.blocks_shot())
                && self.robbo_cell() != Some(next)
            {
                if let Some((_, el)) = self.element_at(next) {
                    if Self::is_blaster_immune(&el.kind) {
                        // stop propagation
                    } else if matches!(el.kind, ElementKind::Bomb) {
                        self.schedule_bomb_detonation(next);
                    } else {
                        self.destroy_at(next, events);
                        let bid = self.allocate_id();
                        self.elements.push((
                            next,
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
                } else if self.element_at(next).is_none() {
                    let bid = self.allocate_id();
                    self.elements.push((
                        next,
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

    fn is_laser_extend_blocked(&self, cell: Cell) -> bool {
        if self.tile_at(cell).is_some_and(|t| t.blocks_shot()) {
            return true;
        }
        if let Some((_, el)) = self.element_at(cell) {
            return !matches!(el.kind, ElementKind::Laser { solid: true, .. });
        }
        false
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

    pub(crate) fn mark_solid_laser_return(&mut self, gun_cell: Cell, direction: Direction) {
        let (dc, dr) = direction.delta();
        let mut cell = gun_cell.offset(dc, dr);
        while self.in_bounds(cell) {
            if let Some((i, _)) = self
                .elements
                .iter()
                .enumerate()
                .find(|(_, (c, _))| *c == cell)
            {
                if let ElementKind::Laser {
                    solid: true,
                    ref mut returning,
                    ..
                } = self.elements[i].1.kind
                {
                    *returning = true;
                    return;
                }
            }
            if self.is_laser_extend_blocked(cell) {
                return;
            }
            cell = cell.offset(dc, dr);
        }
    }
}
