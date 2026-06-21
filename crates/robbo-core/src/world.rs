use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{BirdVariant, ElementKind, ElementState, GunType};
use crate::events::{DeathCause, GameEvent, PlayerInput};
use crate::tile::TileKind;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct World {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<TileKind>,
    pub elements: Vec<(Cell, ElementState)>,
    pub robbo_id: u32,
    pub next_entity_id: u32,
    pub required_screws: u32,
    pub collected_screws: u32,
    pub ammo: u32,
    pub keys: u32,
    pub capsule_open: bool,
    pub tick: u64,
    pub status: LevelStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LevelStatus {
    #[default]
    Playing,
    Complete,
    Failed,
}

impl World {
    pub fn empty(width: u16, height: u16) -> Self {
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            tiles: vec![TileKind::Empty; len],
            elements: Vec::new(),
            robbo_id: 0,
            next_entity_id: 1,
            required_screws: 0,
            collected_screws: 0,
            ammo: 0,
            keys: 0,
            capsule_open: false,
            tick: 0,
            status: LevelStatus::Playing,
        }
    }

    pub fn from_level(
        width: u16,
        height: u16,
        tiles: Vec<TileKind>,
        mut elements: Vec<(Cell, ElementState)>,
        required_screws: u32,
    ) -> Self {
        let mut world = Self {
            width,
            height,
            tiles,
            elements,
            ..Self::empty(width, height)
        };
        world.required_screws = required_screws;
        for (_, state) in &mut world.elements {
            if state.id == 0 {
                let id = world.next_entity_id;
                world.next_entity_id += 1;
                state.id = id;
            }
            if matches!(state.kind, ElementKind::Robbo) {
                world.robbo_id = state.id;
            }
        }
        world
    }

    pub fn allocate_id(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    pub fn restore(&mut self, snap: Self) {
        *self = snap;
    }

    pub fn in_bounds(&self, cell: Cell) -> bool {
        cell.col >= 0
            && cell.row >= 0
            && cell.col < self.width as i16
            && cell.row < self.height as i16
    }

    pub fn tile_at(&self, cell: Cell) -> Option<TileKind> {
        if !self.in_bounds(cell) {
            return None;
        }
        let idx = cell.row as usize * self.width as usize + cell.col as usize;
        self.tiles.get(idx).copied()
    }

    pub fn element_at(&self, cell: Cell) -> Option<(usize, &ElementState)> {
        self.elements
            .iter()
            .enumerate()
            .find(|(_, (c, _))| *c == cell)
            .map(|(i, (_, s))| (i, s))
    }

    pub fn element_by_id(&self, id: u32) -> Option<(usize, &ElementState)> {
        self.elements
            .iter()
            .enumerate()
            .find(|(_, (_, s))| s.id == id)
            .map(|(i, (_, s))| (i, s))
    }

    pub fn robbo_cell(&self) -> Option<Cell> {
        self.elements
            .iter()
            .find(|(_, s)| matches!(s.kind, ElementKind::Robbo))
            .map(|(c, _)| *c)
    }

    pub fn is_blocked(&self, cell: Cell) -> bool {
        if !self.in_bounds(cell) {
            return true;
        }
        if self.tile_at(cell).is_some_and(|t| t.blocks_movement()) {
            return true;
        }
        if let Some((_, el)) = self.element_at(cell) {
            if el.hidden {
                return false;
            }
            return !Self::is_walkable_element(&el.kind);
        }
        false
    }

    fn is_blocked_for_enemy(&self, cell: Cell) -> bool {
        if !self.in_bounds(cell) {
            return true;
        }
        if self.tile_at(cell).is_some_and(|t| t.blocks_movement()) {
            return true;
        }
        if let Some((_, el)) = self.element_at(cell) {
            if el.hidden {
                return false;
            }
            if matches!(el.kind, ElementKind::Robbo) {
                return false;
            }
            return !Self::is_walkable_element(&el.kind);
        }
        false
    }

    fn is_walkable_element(kind: &ElementKind) -> bool {
        matches!(
            kind,
            ElementKind::Screw
                | ElementKind::BulletPickup
                | ElementKind::Key
                | ElementKind::ExtraLife
                | ElementKind::Capsule
                | ElementKind::QuestionMark
        )
    }

    pub fn step(&mut self, input: PlayerInput) -> Vec<GameEvent> {
        if self.status != LevelStatus::Playing {
            return Vec::new();
        }

        let mut events = Vec::new();
        self.tick += 1;

        match input {
            PlayerInput::Move(dir) => self.try_move_robbo(dir, &mut events),
            PlayerInput::Shoot(dir) => self.try_shoot(dir, &mut events),
            PlayerInput::Wait => {}
        }

        self.tick_projectiles(&mut events);
        self.tick_enemies(&mut events);
        self.tick_guns(&mut events);
        self.tick_magnets(&mut events);

        events
    }

    fn try_move_robbo(&mut self, dir: Direction, events: &mut Vec<GameEvent>) {
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
                        self.capsule_open = true;
                    }
                }
                ElementKind::BulletPickup => {
                    self.ammo += 1;
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
                    self.open_doors(events);
                }
                ElementKind::ExtraLife => {
                    events.push(GameEvent::Collected {
                        kind: ElementKind::ExtraLife,
                        at: target,
                    });
                    self.elements.remove(idx);
                }
                ElementKind::Capsule if self.capsule_open => {
                    self.elements[robbo_idx].0 = target;
                    self.elements[robbo_idx].1.direction = dir;
                    events.push(GameEvent::Moved {
                        entity_id: robbo_id,
                        from,
                        to: target,
                    });
                    self.status = LevelStatus::Complete;
                    events.push(GameEvent::LevelComplete);
                    return;
                }
                ElementKind::Box | ElementKind::PushBox => {
                    let beyond = target.offset(dc, dr);
                    if self.in_bounds(beyond)
                        && !self.is_blocked(beyond)
                        && self.element_at(beyond).is_none()
                    {
                        let box_id = self.elements[idx].1.id;
                        self.elements[idx].0 = beyond;
                        self.elements[robbo_idx].0 = target;
                        self.elements[robbo_idx].1.direction = dir;
                        events.push(GameEvent::Pushed {
                            entity_id: box_id,
                            from: target,
                            to: beyond,
                        });
                        events.push(GameEvent::Moved {
                            entity_id: robbo_id,
                            from,
                            to: target,
                        });
                    }
                    return;
                }
                ElementKind::Bomb => {
                    // Bombs are pushable (like boxes). Only projectiles detonate them.
                    let beyond = target.offset(dc, dr);
                    if self.in_bounds(beyond)
                        && !self.is_blocked(beyond)
                        && self.element_at(beyond).is_none()
                    {
                        let bomb_id = self.elements[idx].1.id;
                        self.elements[idx].0 = beyond;
                        self.elements[robbo_idx].0 = target;
                        self.elements[robbo_idx].1.direction = dir;
                        events.push(GameEvent::Pushed {
                            entity_id: bomb_id,
                            from: target,
                            to: beyond,
                        });
                        events.push(GameEvent::Moved {
                            entity_id: robbo_id,
                            from,
                            to: target,
                        });
                    }
                    return;
                }
                ElementKind::QuestionMark => {
                    self.reveal_questionmark(idx, target, events);
                    return;
                }
                ElementKind::Teleport { id } => {
                    if let Some(dest) = self.find_teleport_pair(id, target) {
                        self.elements[robbo_idx].0 = dest;
                        self.elements[robbo_idx].1.direction = dir;
                        events.push(GameEvent::Teleported {
                            entity_id: robbo_id,
                            from: target,
                            to: dest,
                        });
                    }
                    return;
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

    fn try_shoot(&mut self, dir: Direction, events: &mut Vec<GameEvent>) {
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
            ElementState {
                id: proj_id,
                kind: ElementKind::Projectile {
                    direction: dir,
                    from_player: true,
                },
                direction: dir,
                tick_counter: 0,
                hidden: false,
            },
        ));
    }

    fn tick_projectiles(&mut self, events: &mut Vec<GameEvent>) {
        enum ProjAction {
            Move(u32, Cell),
            Remove(u32),
            KillRobbo,
            Explode(Cell),
            RemovePair(u32, u32),
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
            if let Some((target_idx, target)) = self.element_at(next) {
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
                    ElementKind::Bear { .. }
                    | ElementKind::BlackBear { .. }
                    | ElementKind::Bird { .. }
                    | ElementKind::Butterfly => {
                        actions.push(ProjAction::Remove(id));
                        actions.push(ProjAction::Remove(target_id));
                    }
                    ElementKind::Screw | ElementKind::Capsule => {
                        actions.push(ProjAction::Remove(id));
                    }
                    _ => actions.push(ProjAction::Remove(id)),
                }
                let _ = target_idx;
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
            }
        }
    }

    fn remove_element_by_id(&mut self, id: u32) {
        if let Some(idx) = self.elements.iter().position(|(_, s)| s.id == id) {
            self.elements.remove(idx);
        }
    }

    fn tick_enemies(&mut self, events: &mut Vec<GameEvent>) {
        let robbo = self.robbo_cell();
        let snapshot: Vec<(Cell, ElementState)> = self
            .elements
            .iter()
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        let mut updates: Vec<(u32, u32, Direction, Option<Cell>)> = Vec::new();
        let mut bird_shots: Vec<(Cell, Direction)> = Vec::new();

        for (cell, state) in snapshot.iter() {
            let id = state.id;
            let tick_counter = state.tick_counter + 1;
            let delay = match state.kind {
                ElementKind::Bear { .. } => 4,
                ElementKind::BlackBear { .. } => 2,
                ElementKind::Bird { .. } => 3,
                ElementKind::Butterfly => 2,
                _ => {
                    updates.push((id, tick_counter, state.direction, None));
                    continue;
                }
            };
            if tick_counter % delay != 0 {
                updates.push((id, tick_counter, state.direction, None));
                continue;
            }

            let mut direction = state.direction;
            let new_cell = match &state.kind {
                ElementKind::Bear { clockwise } | ElementKind::BlackBear { clockwise } => {
                    let (next, new_dir) = self.wall_follow_with_dir(*cell, direction, *clockwise);
                    direction = new_dir;
                    next
                }
                ElementKind::Bird { variant } => match variant {
                    BirdVariant::Horizontal => {
                        let (dc, _) = direction.delta();
                        let next = cell.offset(dc, 0);
                        if self.is_blocked_for_enemy(next) {
                            direction = direction.opposite();
                            cell.offset(direction.delta().0, 0)
                        } else {
                            next
                        }
                    }
                    BirdVariant::Vertical => {
                        let (_, dr) = direction.delta();
                        let next = cell.offset(0, dr);
                        if self.is_blocked_for_enemy(next) {
                            direction = direction.opposite();
                            cell.offset(0, direction.delta().1)
                        } else {
                            next
                        }
                    }
                    BirdVariant::Firing => {
                        bird_shots.push((*cell, direction));
                        *cell
                    }
                },
                ElementKind::Butterfly => {
                    let dirs = [
                        Direction::Up,
                        Direction::Down,
                        Direction::Left,
                        Direction::Right,
                    ];
                    let pick = (self.tick as usize + id as usize) % dirs.len();
                    let d = dirs[pick];
                    let (dc, dr) = d.delta();
                    let next = cell.offset(dc, dr);
                    if !self.is_blocked_for_enemy(next) {
                        direction = d;
                        next
                    } else {
                        *cell
                    }
                }
                _ => *cell,
            };

            let mv = if new_cell != *cell { Some(new_cell) } else { None };
            if mv == Some(new_cell) && robbo == Some(new_cell) {
                self.kill_robbo(DeathCause::Enemy, events);
                return;
            }
            updates.push((id, tick_counter, direction, mv));
        }

        for (id, tick_counter, direction, mv) in updates {
            if let Some((i, _)) = self.element_by_id(id) {
                self.elements[i].1.tick_counter = tick_counter;
                self.elements[i].1.direction = direction;
                if let Some(to) = mv {
                    let from = self.elements[i].0;
                    self.elements[i].0 = to;
                    events.push(GameEvent::Moved {
                        entity_id: id,
                        from,
                        to,
                    });
                }
            }
        }

        for (from, dir) in bird_shots {
            let (dc, dr) = dir.delta();
            let spawn = from.offset(dc, dr);
            if self.in_bounds(spawn) && !self.is_blocked(spawn) {
                events.push(GameEvent::Shot { from, direction: dir });
                let pid = self.allocate_id();
                self.elements.push((
                    spawn,
                    ElementState {
                        id: pid,
                        kind: ElementKind::Projectile {
                            direction: dir,
                            from_player: false,
                        },
                        direction: dir,
                        tick_counter: 0,
                        hidden: false,
                    },
                ));
            }
        }
    }

    fn wall_follow_with_dir(
        &self,
        cell: Cell,
        dir: Direction,
        clockwise: bool,
    ) -> (Cell, Direction) {
        let forward = cell.offset(dir.delta().0, dir.delta().1);
        let turn = if clockwise {
            dir.turn_right()
        } else {
            dir.turn_left()
        };
        let turned = cell.offset(turn.delta().0, turn.delta().1);

        if !self.is_blocked_for_enemy(forward) {
            (forward, dir)
        } else if !self.is_blocked_for_enemy(turned) {
            (turned, turn)
        } else {
            (cell, dir)
        }
    }

    fn tick_guns(&mut self, events: &mut Vec<GameEvent>) {
        let gun_ids: Vec<u32> = self
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Gun { .. }))
            .map(|(_, s)| s.id)
            .collect();

        for id in gun_ids {
            let Some((i, _)) = self.element_by_id(id) else {
                continue;
            };
            self.elements[i].1.tick_counter += 1;
            let tick_counter = self.elements[i].1.tick_counter;
            if tick_counter % 8 != 0 {
                continue;
            }
            let (gun_type, direction) = match &self.elements[i].1.kind {
                ElementKind::Gun {
                    gun_type,
                    direction,
                    ..
                } => (*gun_type, *direction),
                _ => continue,
            };
            let from = self.elements[i].0;
            let (dc, dr) = direction.delta();
            let spawn = from.offset(dc, dr);
            if self.in_bounds(spawn) && !self.is_blocked(spawn) {
                events.push(GameEvent::Shot { from, direction });
                if gun_type != GunType::Laser {
                    let pid = self.allocate_id();
                    self.elements.push((
                        spawn,
                        ElementState {
                            id: pid,
                            kind: ElementKind::Projectile {
                                direction,
                                from_player: false,
                            },
                            direction,
                            tick_counter: 0,
                            hidden: false,
                        },
                    ));
                }
            }
        }
    }

    fn tick_magnets(&mut self, events: &mut Vec<GameEvent>) {
        let Some(robbo_idx) = self
            .elements
            .iter()
            .position(|(_, s)| matches!(s.kind, ElementKind::Robbo))
        else {
            return;
        };
        let robbo_cell = self.elements[robbo_idx].0;
        let robbo_id = self.elements[robbo_idx].1.id;

        let magnets: Vec<(Cell, Direction)> = self
            .elements
            .iter()
            .filter_map(|(cell, s)| match s.kind {
                ElementKind::Magnet { direction } => Some((*cell, direction)),
                _ => None,
            })
            .collect();

        for (mag_cell, direction) in magnets {
            let (dc, dr) = direction.delta();
            let same_row = robbo_cell.row == mag_cell.row && dc != 0;
            let same_col = robbo_cell.col == mag_cell.col && dr != 0;
            if !same_row && !same_col {
                continue;
            }
            let pull = robbo_cell.offset(dc, dr);
            if self.is_blocked(pull) || self.element_at(pull).is_some() {
                continue;
            }
            let from = robbo_cell;
            self.elements[robbo_idx].0 = pull;
            events.push(GameEvent::Moved {
                entity_id: robbo_id,
                from,
                to: pull,
            });
            break;
        }
    }

    fn explode_at(&mut self, at: Cell, events: &mut Vec<GameEvent>) {
        self.explode_at_inner(at, events, 0);
    }

    fn explode_at_inner(&mut self, at: Cell, events: &mut Vec<GameEvent>, depth: u32) {
        if depth > 16 || !self.in_bounds(at) {
            return;
        }
        events.push(GameEvent::Exploded { at });

        let mut affected = std::collections::HashSet::new();
        for dc in -1..=1 {
            for dr in -1..=1 {
                let n = at.offset(dc, dr);
                if self.in_bounds(n) {
                    affected.insert(n);
                }
            }
        }

        let mut chain_cells = Vec::new();
        for n in &affected {
            if self.robbo_cell() == Some(*n) {
                self.kill_robbo(DeathCause::Explosion, events);
            }
            for (c, s) in &self.elements {
                if *c == *n && matches!(s.kind, ElementKind::Bomb) {
                    chain_cells.push(*n);
                }
            }
        }

        // Original digit1024: kill every object in the 3×3 area except bullets.
        self.elements.retain(|(c, s)| {
            if !affected.contains(c) {
                return true;
            }
            matches!(
                s.kind,
                ElementKind::Projectile { .. } | ElementKind::Robbo
            )
        });

        for cell in chain_cells {
            self.explode_at_inner(cell, events, depth + 1);
        }
    }

    fn reveal_questionmark(&mut self, idx: usize, at: Cell, events: &mut Vec<GameEvent>) {
        self.elements[idx].1.hidden = false;
        self.elements[idx].1.kind = ElementKind::Screw;
        events.push(GameEvent::Revealed { at });
    }

    fn find_teleport_pair(&self, id: u8, from: Cell) -> Option<Cell> {
        self.elements
            .iter()
            .find(|(c, s)| {
                *c != from && matches!(s.kind, ElementKind::Teleport { id: tid } if tid == id)
            })
            .map(|(c, _)| *c)
    }

    fn open_doors(&mut self, events: &mut Vec<GameEvent>) {
        if self.keys == 0 {
            return;
        }
        self.keys -= 1;
        for tile in self.tiles.iter_mut() {
            if *tile == TileKind::DoorClosed {
                *tile = TileKind::DoorOpen;
            }
        }
        events.push(GameEvent::DoorOpened);
    }

    fn kill_robbo(&mut self, cause: DeathCause, events: &mut Vec<GameEvent>) {
        self.status = LevelStatus::Failed;
        events.push(GameEvent::Died {
            entity_id: self.robbo_id,
            cause,
        });
        events.push(GameEvent::LevelFailed);
    }

    pub fn state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.width.hash(&mut h);
        self.height.hash(&mut h);
        self.tiles.hash(&mut h);
        for (c, s) in &self.elements {
            c.hash(&mut h);
            s.id.hash(&mut h);
            format!("{:?}", s.kind).hash(&mut h);
        }
        self.collected_screws.hash(&mut h);
        self.ammo.hash(&mut h);
        self.status.hash(&mut h);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandHistory;
    use crate::element::ElementState;

    fn robbo_at(cell: Cell) -> ElementState {
        ElementState {
            id: 1,
            kind: ElementKind::Robbo,
            direction: Direction::Down,
            tick_counter: 0,
            hidden: false,
        }
    }

    fn make_world(
        w: u16,
        h: u16,
        tiles: Vec<TileKind>,
        elements: Vec<(Cell, ElementState)>,
        screws: u32,
    ) -> World {
        World::from_level(w, h, tiles, elements, screws)
    }

    #[test]
    fn move_and_collect_screw() {
        let w = 5u16;
        let h = 5u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(2, 2), robbo_at(Cell::new(2, 2))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Screw,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 1);
        let events = world.step(PlayerInput::Move(Direction::Up));
        assert!(events.iter().any(|e| matches!(e, GameEvent::Collected { .. })));
        assert_eq!(world.collected_screws, 1);
        assert!(world.capsule_open);
    }

    #[test]
    fn push_box() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Box,
                    direction: Direction::Right,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let events = world.step(PlayerInput::Move(Direction::Right));
        assert!(events.iter().any(|e| matches!(e, GameEvent::Pushed { .. })));
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
    }

    #[test]
    fn push_against_wall_blocked() {
        let w = 4u16;
        let h = 4u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[5] = TileKind::WallGrey;
        let elements = vec![(Cell::new(0, 1), robbo_at(Cell::new(0, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        let before = world.robbo_cell();
        let events = world.step(PlayerInput::Move(Direction::Right));
        assert!(events.is_empty());
        assert_eq!(world.robbo_cell(), before);
    }

    #[test]
    fn shoot_blocked_no_ammo_loss() {
        let w = 4u16;
        let h = 4u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[5] = TileKind::WallGrey;
        let elements = vec![(Cell::new(0, 1), robbo_at(Cell::new(0, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 3;
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 3);
    }

    #[test]
    fn shoot_enemy_kills_it() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(3, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Bear { clockwise: false },
                    direction: Direction::Left,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.step(PlayerInput::Shoot(Direction::Right));
        world.step(PlayerInput::Wait);
        assert!(world.elements.iter().all(|(_, s)| !matches!(s.kind, ElementKind::Bear { .. })));
    }

    #[test]
    fn bear_kills_robbo_on_tick() {
        let w = 4u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Bear { clockwise: false },
                    direction: Direction::Left,
                    tick_counter: 3,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let events = world.step(PlayerInput::Wait);
        assert!(events.iter().any(|e| matches!(e, GameEvent::LevelFailed)));
        assert_eq!(world.status, LevelStatus::Failed);
    }

    #[test]
    fn bomb_chain_detonation() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Bomb,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
            (
                Cell::new(3, 1),
                ElementState {
                    id: 3,
                    kind: ElementKind::Bomb,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.step(PlayerInput::Shoot(Direction::Right));
        world.step(PlayerInput::Wait);
        world.step(PlayerInput::Wait);
        assert!(world.elements.iter().all(|(_, s)| !matches!(s.kind, ElementKind::Bomb)));
    }

    #[test]
    fn bomb_explodes_full_3x3_area() {
        let w = 6u16;
        let h = 5u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let box_at = |id: u32, col: i16, row: i16| {
            (
                Cell::new(col, row),
                ElementState {
                    id,
                    kind: ElementKind::Box,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            )
        };
        let elements = vec![
            (Cell::new(0, 2), robbo_at(Cell::new(0, 2))),
            (
                Cell::new(2, 2),
                ElementState {
                    id: 2,
                    kind: ElementKind::Bomb,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
            box_at(3, 1, 1), // diagonal
            box_at(4, 3, 2), // cardinal
            box_at(5, 2, 3), // below center
            box_at(6, 4, 2), // outside 3×3 — must survive
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        // Projectile reaches bomb on the same step (spawn at 1,2 then tick to 2,2).
        world.step(PlayerInput::Shoot(Direction::Right));

        assert!(world.elements.iter().all(|(_, s)| !matches!(s.kind, ElementKind::Bomb)));
        assert!(!world.elements.iter().any(|(c, s)| {
            matches!(s.kind, ElementKind::Box) && *c != Cell::new(4, 2)
        }));
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(4, 2) && matches!(s.kind, ElementKind::Box)));
    }

    #[test]
    fn teleport_to_pair() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Teleport { id: 1 },
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
            (
                Cell::new(4, 1),
                ElementState {
                    id: 3,
                    kind: ElementKind::Teleport { id: 1 },
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(4, 1)));
    }

    #[test]
    fn questionmark_reveal_on_push() {
        let w = 5u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::QuestionMark,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: true,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert!(world
            .elements
            .iter()
            .any(|(_, s)| matches!(s.kind, ElementKind::Screw)));
    }

    #[test]
    fn door_blocks_until_key() {
        let w = 4u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[1] = TileKind::DoorClosed;
        let elements = vec![(Cell::new(0, 0), robbo_at(Cell::new(0, 0)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        assert!(world.is_blocked(Cell::new(1, 0)));
        world.keys = 1;
        world.open_doors(&mut Vec::new());
        assert!(!world.is_blocked(Cell::new(1, 0)));
    }

    #[test]
    fn level_complete_on_capsule() {
        let w = 5u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Capsule,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.capsule_open = true;
        let events = world.step(PlayerInput::Move(Direction::Right));
        assert!(events.iter().any(|e| matches!(e, GameEvent::LevelComplete)));
    }

    #[test]
    fn undo_multi_step() {
        let mut history = CommandHistory::new();
        let w = 5u16;
        let h = 5u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(2, 2), robbo_at(Cell::new(2, 2))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Screw,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 1);
        history.record(world.snapshot());
        world.step(PlayerInput::Move(Direction::Up));
        history.record(world.snapshot());
        world.step(PlayerInput::Wait);
        let restored = history.undo(world.snapshot()).unwrap();
        assert_eq!(restored.collected_screws, 1);
        let earlier = history.undo(restored).unwrap();
        assert_eq!(earlier.collected_screws, 0);
    }

    #[test]
    fn determinism_hash() {
        let w = 5u16;
        let h = 5u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(2, 2), robbo_at(Cell::new(2, 2))),
            (
                Cell::new(2, 1),
                ElementState {
                    id: 2,
                    kind: ElementKind::Screw,
                    direction: Direction::Down,
                    tick_counter: 0,
                    hidden: false,
                },
            ),
        ];
        let mut w1 = make_world(w, h, tiles.clone(), elements.clone(), 1);
        let mut w2 = make_world(w, h, tiles, elements, 1);
        w1.step(PlayerInput::Move(Direction::Up));
        w2.step(PlayerInput::Move(Direction::Up));
        assert_eq!(w1.state_hash(), w2.state_hash());
    }
}
