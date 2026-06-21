use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState};
use crate::events::{DeathCause, GameEvent};
use crate::tile::TileKind;
use crate::world::World;

pub(crate) enum ProjectileHitAction {
    RemoveProjectile(u32),
    Remove(u32),
    RemovePair(u32, u32),
    KillRobbo,
    Explode(Cell),
    DestroyAt(Cell),
    ClearGround(Cell),
}

impl World {
    /// Resolve what happens when a player/enemy shot reaches `at`.
    /// Returns `None` when the cell is empty and the projectile should keep flying.
    pub(crate) fn resolve_projectile_hit(
        &self,
        at: Cell,
        from_player: bool,
        projectile_id: u32,
    ) -> Option<Vec<ProjectileHitAction>> {
        if let Some((_, target)) = self.element_at(at) {
            let target_id = target.id;
            let actions = match &target.kind {
                ElementKind::Robbo if !from_player => {
                    vec![
                        ProjectileHitAction::KillRobbo,
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                    ]
                }
                ElementKind::Box | ElementKind::PushBox => {
                    vec![ProjectileHitAction::RemovePair(projectile_id, target_id)]
                }
                ElementKind::Bomb => {
                    vec![
                        ProjectileHitAction::RemovePair(projectile_id, target_id),
                        ProjectileHitAction::Explode(at),
                    ]
                }
                ElementKind::QuestionMark { .. } => {
                    vec![
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                        ProjectileHitAction::DestroyAt(at),
                    ]
                }
                _ if self.tile_at(at) == Some(TileKind::Ground) => {
                    vec![
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                        ProjectileHitAction::DestroyAt(at),
                    ]
                }
                ElementKind::Bear { .. }
                | ElementKind::BlackBear { .. }
                | ElementKind::Bird { .. }
                | ElementKind::Butterfly => {
                    vec![
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                        ProjectileHitAction::Remove(target_id),
                    ]
                }
                ElementKind::Screw | ElementKind::Capsule => {
                    vec![
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                        ProjectileHitAction::Remove(target_id),
                    ]
                }
                ElementKind::Key | ElementKind::BulletPickup => {
                    vec![
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                        ProjectileHitAction::Remove(target_id),
                    ]
                }
                ElementKind::BarbedWire | ElementKind::Stop => {
                    vec![
                        ProjectileHitAction::RemoveProjectile(projectile_id),
                        ProjectileHitAction::Remove(target_id),
                    ]
                }
                ElementKind::Gun { .. }
                | ElementKind::Magnet { .. }
                | ElementKind::Teleport { .. }
                | ElementKind::Laser { .. } => {
                    vec![ProjectileHitAction::RemoveProjectile(projectile_id)]
                }
                _ => vec![ProjectileHitAction::RemoveProjectile(projectile_id)],
            };
            return Some(actions);
        }

        if self.tile_at(at) == Some(TileKind::Ground) {
            return Some(vec![
                ProjectileHitAction::RemoveProjectile(projectile_id),
                ProjectileHitAction::ClearGround(at),
            ]);
        }

        None
    }

    pub(crate) fn apply_projectile_hit_actions(
        &mut self,
        actions: &[ProjectileHitAction],
        events: &mut Vec<GameEvent>,
    ) {
        for action in actions {
            match *action {
                ProjectileHitAction::RemoveProjectile(id) | ProjectileHitAction::Remove(id) => {
                    self.remove_element_by_id(id);
                }
                ProjectileHitAction::RemovePair(a, b) => {
                    self.remove_element_by_id(a);
                    self.remove_element_by_id(b);
                }
                ProjectileHitAction::KillRobbo => self.kill_robbo(DeathCause::Projectile, events),
                ProjectileHitAction::Explode(at) => self.explode_at(at, events),
                ProjectileHitAction::DestroyAt(at) => self.destroy_at(at, events),
                ProjectileHitAction::ClearGround(at) => {
                    self.set_tile(at, TileKind::Empty);
                }
            }
        }
    }

    pub(crate) fn tick_projectiles(&mut self, events: &mut Vec<GameEvent>) {
        enum ProjAction {
            Move(u32, Cell),
            Hit(Vec<ProjectileHitAction>),
            Remove(u32),
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
            if let Some(hit) = self.resolve_projectile_hit(next, from_player, id) {
                actions.push(ProjAction::Hit(hit));
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
                ProjAction::Hit(hit) => self.apply_projectile_hit_actions(&hit, events),
                ProjAction::Remove(id) => self.remove_element_by_id(id),
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
