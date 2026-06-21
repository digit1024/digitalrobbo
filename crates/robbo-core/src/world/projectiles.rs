use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState};
use crate::events::{DeathCause, GameEvent};
use crate::tile::TileKind;
use crate::world::World;

impl World {
    pub(crate) fn tick_projectiles(&mut self, events: &mut Vec<GameEvent>) {
        enum ProjAction {
            Move(u32, Cell),
            Remove(u32),
            KillRobbo,
            Explode(Cell),
            RemovePair(u32, u32),
            DestroyAt(Cell),
        }

        let mut actions = Vec::new();

        for (cell, state) in self.elements.iter() {
            let ElementKind::Projectile { direction, from_player } = state.kind else {
                continue;
            };
            let id = state.id;
            let (dc, dr) = direction.delta();
            let next = cell.offset(dc, dr);
            if !self.in_bounds(next) || self.tile_at(next).is_some_and(|t| t.blocks_shot()) {
                actions.push(ProjAction::Remove(id));
                continue;
            }
            if let Some((_, target)) = self.element_at(next) {
                let target_id = target.id;
                match &target.kind {
                    ElementKind::Robbo if !from_player => {
                        actions.push(ProjAction::KillRobbo);
                        actions.push(ProjAction::Remove(id));
                    }
                    ElementKind::Box | ElementKind::PushBox => {
                        actions.push(ProjAction::RemovePair(id, target_id));
                    }
                    ElementKind::Bomb => {
                        actions.push(ProjAction::RemovePair(id, target_id));
                        actions.push(ProjAction::Explode(next));
                    }
                    ElementKind::QuestionMark { .. } => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::DestroyAt(next));
                    }
                    _ if self.tile_at(next) == Some(TileKind::Ground) => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::DestroyAt(next));
                    }
                    ElementKind::Bear { .. }
                    | ElementKind::BlackBear { .. }
                    | ElementKind::Bird { .. }
                    | ElementKind::Butterfly => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::Remove(target_id));
                    }
                    ElementKind::Screw | ElementKind::Capsule => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::Remove(target_id));
                    }
                    ElementKind::Key | ElementKind::BulletPickup => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::Remove(target_id));
                    }
                    ElementKind::BarbedWire | ElementKind::Stop => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::Remove(target_id));
                    }
                    ElementKind::Gun { .. }
                    | ElementKind::Magnet { .. }
                    | ElementKind::Teleport { .. }
                    | ElementKind::Laser { .. } => {
                        actions.push(ProjAction::Remove(id));
                    }
                    _ => actions.push(ProjAction::Remove(id)),
                }
                continue;
            }
            actions.push(ProjAction::Move(id, next));
        }

        for action in actions {
            match action {
                ProjAction::Move(id, next) => {
                    if let Some((i, _)) = self.element_by_id(id) {
                        let from = self.elements[i].0;
                        self.elements[i].0 = next;
                        events.push(GameEvent::Moved {
                            entity_id: id,
                            from,
                            to: next,
                        });
                    }
                }
                ProjAction::Remove(id) => self.remove_element_by_id(id),
                ProjAction::RemovePair(a, b) => {
                    self.remove_element_by_id(a);
                    self.remove_element_by_id(b);
                }
                ProjAction::KillRobbo => self.kill_robbo(DeathCause::Projectile, events),
                ProjAction::Explode(at) => self.explode_at(at, events),
                ProjAction::DestroyAt(at) => self.destroy_at(at, events),
            }
        }
    }

    pub(crate) fn tick_push_boxes(&mut self, events: &mut Vec<GameEvent>) {
        let sliding: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::PushBox) && s.sliding)
            .map(|(_, s)| s.id)
            .collect();

        for id in sliding {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            self.elements[i].1.tick_counter += 1;
            if self.elements[i].1.tick_counter % 4 != 0 {
                continue;
            }
            let cell = self.elements[i].0;
            let dir = self.elements[i].1.direction;
            let (dc, dr) = dir.delta();
            let next = cell.offset(dc, dr);
            if self.in_bounds(next) && !self.is_blocked(next) && self.element_at(next).is_none() {
                self.elements[i].0 = next;
                events.push(GameEvent::Moved {
                    entity_id: id,
                    from: cell,
                    to: next,
                });
            } else {
                self.elements[i].1.sliding = false;
                self.shoot_from_cell(cell, dir, events);
            }
        }
    }

    pub(crate) fn shoot_from_cell(&mut self, from: Cell, dir: Direction, events: &mut Vec<GameEvent>) {
        let (dc, dr) = dir.delta();
        let spawn = from.offset(dc, dr);
        if !self.in_bounds(spawn) {
            return;
        }
        if self.tile_at(spawn).is_some_and(|t| t.blocks_shot()) {
            return;
        }
        if self.element_at(spawn).is_some() {
            return;
        }
        events.push(GameEvent::Shot { from, direction: dir });
        let pid = self.allocate_id();
        self.elements.push((
            spawn,
            ElementState::new(
                pid,
                ElementKind::Projectile {
                    direction: dir,
                    from_player: false,
                },
                dir,
            ),
        ));
    }
}
