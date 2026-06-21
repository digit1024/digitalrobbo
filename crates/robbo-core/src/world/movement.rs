use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState};
use crate::events::{DeathCause, GameEvent};
use crate::tile::TileKind;
use crate::world::{World, MAX_TELEPORT_INDEX};

impl World {
    pub(crate) fn try_move_robbo(&mut self, dir: Direction, events: &mut Vec<GameEvent>) {
        let Some(robbo_idx) = self
            .elements
            .iter()
            .position(|(_, s)| matches!(s.kind, ElementKind::Robbo))
        else {
            return;
        };

        let robbo_id = self.elements[robbo_idx].1.id;
        let from = self.elements[robbo_idx].0;
        let (dc, dr) = dir.delta();
        let target = from.offset(dc, dr);

        if !self.in_bounds(target) {
            return;
        }

        // Door: open with key, Robbo stays put.
        if self.tile_at(target) == Some(TileKind::DoorClosed) {
            if self.keys > 0 {
                self.keys -= 1;
                self.set_tile(target, TileKind::Empty);
                events.push(GameEvent::DoorOpened);
            }
            return;
        }

        // STOP tile clears on walk.
        if self.tile_at(target) == Some(TileKind::Stop) {
            self.set_tile(target, TileKind::Empty);
            self.elements[robbo_idx].0 = target;
            self.elements[robbo_idx].1.direction = dir;
            events.push(GameEvent::Moved {
                entity_id: robbo_id,
                from,
                to: target,
            });
            return;
        }

        if let Some((idx, el)) = self.element_at(target) {
            let kind = el.kind.clone();
            match kind {
                ElementKind::Screw => {
                    self.collected_screws += 1;
                    events.push(GameEvent::Collected {
                        kind: ElementKind::Screw,
                        at: target,
                    });
                    self.elements.remove(idx);
                    if self.collected_screws >= self.required_screws {
                        self.open_capsule();
                    }
                }
                ElementKind::BulletPickup => {
                    self.ammo += 9;
                    events.push(GameEvent::Collected {
                        kind: ElementKind::BulletPickup,
                        at: target,
                    });
                    self.elements.remove(idx);
                }
                ElementKind::Key => {
                    self.keys += 1;
                    events.push(GameEvent::Collected {
                        kind: ElementKind::Key,
                        at: target,
                    });
                    self.elements.remove(idx);
                }
                ElementKind::BarbedWire => {
                    self.elements.remove(idx);
                    self.kill_robbo(DeathCause::Hazard, events);
                    return;
                }
                ElementKind::Capsule if self.capsule_open => {
                    self.elements[robbo_idx].0 = target;
                    self.elements[robbo_idx].1.direction = dir;
                    events.push(GameEvent::Moved {
                        entity_id: robbo_id,
                        from,
                        to: target,
                    });
                    self.status = crate::world::LevelStatus::Complete;
                    events.push(GameEvent::LevelComplete);
                    return;
                }
                ElementKind::Box
                | ElementKind::PushBox
                | ElementKind::Bomb
                | ElementKind::QuestionMark { .. } => {
                    if Self::gun_blocks_push(&kind) {
                        return;
                    }
                    if !Self::is_pushable(&kind) {
                        return;
                    }
                    self.try_push_object(robbo_idx, idx, from, target, dir, events);
                    return;
                }
                ElementKind::Gun {
                    movable: true, ..
                } => {
                    self.try_push_object(robbo_idx, idx, from, target, dir, events);
                    return;
                }
                ElementKind::Gun {
                    movable: false, ..
                } => return,
                ElementKind::Teleport { group, pair_index } if group > 0 => {
                    if self.try_teleport(robbo_idx, target, group, pair_index, dir, events) {
                        return;
                    }
                    return;
                }
                ElementKind::Teleport { .. } => {
                    // group == 0: inactive, walk onto tile
                }
                ElementKind::Bear { .. }
                | ElementKind::BlackBear { .. }
                | ElementKind::Bird { .. }
                | ElementKind::Butterfly => {
                    self.kill_robbo(DeathCause::Enemy, events);
                    return;
                }
                _ => return,
            }
        } else if self.is_blocked(target) {
            return;
        }

        self.elements[robbo_idx].0 = target;
        self.elements[robbo_idx].1.direction = dir;
        events.push(GameEvent::Moved {
            entity_id: robbo_id,
            from,
            to: target,
        });
    }

    fn try_push_object(
        &mut self,
        robbo_idx: usize,
        obj_idx: usize,
        from: Cell,
        target: Cell,
        dir: Direction,
        events: &mut Vec<GameEvent>,
    ) {
        let beyond = target.offset(dir.delta().0, dir.delta().1);
        if !self.in_bounds(beyond) || self.is_blocked(beyond) || self.element_at(beyond).is_some() {
            return;
        }
        let robbo_id = self.elements[robbo_idx].1.id;
        let obj_id = self.elements[obj_idx].1.id;
        let is_push_box = matches!(self.elements[obj_idx].1.kind, ElementKind::PushBox);
        self.elements[obj_idx].0 = beyond;
        if is_push_box {
            self.elements[obj_idx].1.sliding = true;
            self.elements[obj_idx].1.direction = dir;
        }
        self.elements[robbo_idx].0 = target;
        self.elements[robbo_idx].1.direction = dir;
        events.push(GameEvent::Pushed {
            entity_id: obj_id,
            from: target,
            to: beyond,
        });
        events.push(GameEvent::Moved {
            entity_id: robbo_id,
            from,
            to: target,
        });
    }

    fn try_teleport(
        &mut self,
        robbo_idx: usize,
        entry: Cell,
        group: u8,
        pair_index: i8,
        robbo_dir: Direction,
        events: &mut Vec<GameEvent>,
    ) -> bool {
        let robbo_id = self.elements[robbo_idx].1.id;
        let from = self.elements[robbo_idx].0;

        let mut search_index = pair_index;
        let start_index = pair_index;
        loop {
            search_index += 1;
            if search_index > MAX_TELEPORT_INDEX {
                search_index = 0;
            }
            let Some(portal_cell) = self.find_teleport_by_group_index(group, search_index) else {
                if search_index == start_index {
                    return false;
                }
                continue;
            };
            if portal_cell == entry {
                if search_index == start_index {
                    return false;
                }
                continue;
            }

            let mut dir_idx = Self::direction_to_gnurobbo(robbo_dir);
            for attempt in 0..4 {
                let exit_dir = Self::gnurobbo_to_direction(dir_idx);
                let (dc, dr) = exit_dir.delta();
                let dest = portal_cell.offset(dc, dr);
                if self.can_robbo_stand(dest) {
                    self.elements[robbo_idx].0 = dest;
                    self.elements[robbo_idx].1.direction = exit_dir;
                    events.push(GameEvent::Teleported {
                        entity_id: robbo_id,
                        from,
                        to: dest,
                    });
                    return true;
                }
                dir_idx = Self::alternate_exit_dir(dir_idx, attempt);
            }

            if search_index == start_index {
                return false;
            }
        }
    }

    fn find_teleport_by_group_index(&self, group: u8, index: i8) -> Option<Cell> {
        self.elements.iter().find_map(|(cell, state)| {
            if let ElementKind::Teleport {
                group: g,
                pair_index,
            } = state.kind
            {
                if g == group && pair_index == index {
                    return Some(*cell);
                }
            }
            None
        })
    }

    pub(crate) fn try_shoot(&mut self, dir: Direction, events: &mut Vec<GameEvent>) {
        if self.ammo == 0 {
            return;
        }
        let Some(from) = self.robbo_cell() else {
            return;
        };
        let (dc, dr) = dir.delta();
        let spawn = from.offset(dc, dr);
        if !self.in_bounds(spawn) || self.is_blocked(spawn) {
            return;
        }
        self.ammo -= 1;
        events.push(GameEvent::Shot { from, direction: dir });
        let proj_id = self.allocate_id();
        self.elements.push((
            spawn,
            ElementState::new(
                proj_id,
                ElementKind::Projectile {
                    direction: dir,
                    from_player: true,
                },
                dir,
            ),
        ));
    }
}
