use std::collections::HashMap;

use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState};
use crate::events::{DeathCause, GameEvent, PlayerInput};
use crate::tile::TileKind;

mod barriers;
mod enemies;
mod explosion;
mod guns;
mod lasers;
mod magnets;
mod movement;
mod projectiles;
mod spawn;

#[cfg(test)]
mod tests;

pub const MAX_TELEPORT_INDEX: i8 = 15;

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
    pub rng_state: u64,
    pub sensible_questionmarks: bool,
    pub sensible_bears: bool,
    pub robbo_magnet_locked: bool,
    pub magnet_pull_dir: Direction,
    pub barrier_directions: HashMap<Cell, Direction>,
    pub delayed_bomb_cells: Vec<Cell>,
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
            rng_state: 1,
            sensible_questionmarks: true,
            sensible_bears: true,
            robbo_magnet_locked: false,
            magnet_pull_dir: Direction::Down,
            barrier_directions: HashMap::new(),
            delayed_bomb_cells: Vec::new(),
        }
    }

    pub fn from_level(
        width: u16,
        height: u16,
        tiles: Vec<TileKind>,
        elements: Vec<(Cell, ElementState)>,
        required_screws: u32,
    ) -> Self {
        let mut world = Self {
            width,
            height,
            tiles,
            elements,
            required_screws,
            ..Self::empty(width, height)
        };
        world.rng_state = (width as u64)
            .wrapping_mul(31)
            .wrapping_add(height as u64)
            .wrapping_add(required_screws as u64)
            .wrapping_add(1);
        for i in 0..world.elements.len() {
            if world.elements[i].1.id == 0 {
                world.elements[i].1.id = world.allocate_id();
            }
        }
        for (_, state) in &world.elements {
            world.next_entity_id = world.next_entity_id.max(state.id.saturating_add(1));
        }
        for (_, state) in &world.elements {
            if matches!(state.kind, ElementKind::Robbo) {
                world.robbo_id = state.id;
            }
        }
        world
    }

    pub fn init_after_load(&mut self) {
        self.init_questionmarks();
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

    pub(crate) fn set_tile(&mut self, cell: Cell, tile: TileKind) {
        if !self.in_bounds(cell) {
            return;
        }
        let idx = cell.row as usize * self.width as usize + cell.col as usize;
        if let Some(t) = self.tiles.get_mut(idx) {
            *t = tile;
        }
    }

    /// Clear a ground tile and notify the view layer for vanish animation.
    pub(crate) fn clear_ground_tile(&mut self, cell: Cell, events: &mut Vec<GameEvent>) {
        if self.tile_at(cell) != Some(TileKind::Ground) {
            return;
        }
        self.set_tile(cell, TileKind::Empty);
        events.push(GameEvent::TileCleared {
            at: cell,
            kind: TileKind::Ground,
        });
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

    pub(crate) fn is_blocked_for_enemy(&self, cell: Cell) -> bool {
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

    pub(crate) fn is_walkable_element(kind: &ElementKind) -> bool {
        matches!(
            kind,
            ElementKind::Screw
                | ElementKind::BulletPickup
                | ElementKind::Key
                | ElementKind::Capsule
        )
    }

    pub(crate) fn is_pushable(kind: &ElementKind) -> bool {
        matches!(
            kind,
            ElementKind::Box
                | ElementKind::PushBox
                | ElementKind::Bomb
                | ElementKind::QuestionMark { .. }
                | ElementKind::Gun { movable: true, .. }
        )
    }

    pub(crate) fn gun_blocks_push(kind: &ElementKind) -> bool {
        matches!(kind, ElementKind::Gun { movable: false, .. })
    }

    pub fn step(&mut self, input: PlayerInput) -> Vec<GameEvent> {
        if self.status != LevelStatus::Playing {
            return Vec::new();
        }

        let mut events = Vec::new();
        self.tick += 1;

        if self.robbo_magnet_locked {
            self.magnet_pull_robbo(&mut events);
        } else {
            match input {
                PlayerInput::Move(dir) => self.try_move_robbo(dir, &mut events),
                PlayerInput::Shoot(dir) => self.try_shoot(dir, &mut events),
                PlayerInput::Wait => {}
            }
        }

        self.tick_pending_spawns(&mut events);
        self.tick_projectiles(&mut events);
        self.tick_lasers_and_blasters(&mut events);
        self.tick_push_boxes(&mut events);
        self.tick_enemies(&mut events);
        self.tick_guns(&mut events);
        self.tick_barriers(&mut events);
        self.tick_magnets(&mut events);
        self.tick_delayed_bombs(&mut events);

        events
    }

    pub(crate) fn remove_element_by_id(&mut self, id: u32) {
        if let Some(idx) = self.elements.iter().position(|(_, s)| s.id == id) {
            self.elements.remove(idx);
        }
    }

    pub(crate) fn kill_robbo(&mut self, cause: DeathCause, events: &mut Vec<GameEvent>) {
        self.status = LevelStatus::Failed;
        self.robbo_magnet_locked = false;
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

    pub(crate) fn direction_to_gnurobbo(dir: Direction) -> u8 {
        match dir {
            Direction::Right => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Up => 3,
        }
    }

    pub(crate) fn gnurobbo_to_direction(v: u8) -> Direction {
        match v & 3 {
            0 => Direction::Right,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Up,
        }
    }

    pub(crate) fn alternate_exit_dir(dir: u8, attempt: u32) -> u8 {
        dir ^ ((((attempt + 1) % 2) + 2) as u8)
    }

    pub(crate) fn can_robbo_stand(&self, cell: Cell) -> bool {
        if !self.in_bounds(cell) {
            return false;
        }
        if self.is_blocked(cell) {
            return false;
        }
        if let Some((_, el)) = self.element_at(cell) {
            if !el.hidden && !Self::is_walkable_element(&el.kind) {
                return false;
            }
        }
        true
    }

    fn tick_pending_spawns(&mut self, events: &mut Vec<GameEvent>) {
        let mut done = Vec::new();
        for (i, (_, state)) in self.elements.iter().enumerate() {
            if let ElementKind::BigBoom {
                content,
                ticks_left,
            } = state.kind
            {
                if ticks_left <= 1 {
                    done.push((i, state.id, content));
                }
            }
        }
        for (idx, id, content) in done.into_iter().rev() {
            let at = self.elements[idx].0;
            self.remove_element_by_id(id);
            self.spawn_question_content(at, content, events);
        }
        for (_, state) in &mut self.elements {
            if let ElementKind::BigBoom { ticks_left, .. } = &mut state.kind {
                *ticks_left = ticks_left.saturating_sub(1);
            }
        }
    }

    fn tick_delayed_bombs(&mut self, events: &mut Vec<GameEvent>) {
        let cells: Vec<Cell> = self.delayed_bomb_cells.drain(..).collect();
        for cell in cells {
            self.explode_at(cell, events);
        }
    }

    pub(crate) fn schedule_bomb_detonation(&mut self, cell: Cell) {
        if !self.delayed_bomb_cells.contains(&cell) {
            self.delayed_bomb_cells.push(cell);
        }
    }

    pub(crate) fn destroy_at(&mut self, cell: Cell, events: &mut Vec<GameEvent>) {
        if let Some((idx, state)) = self.element_at(cell) {
            let kind = state.kind.clone();
            let id = state.id;
            match kind {
                ElementKind::QuestionMark { content } => {
                    self.elements.remove(idx);
                    let boom_id = self.allocate_id();
                    self.elements.push((
                        cell,
                        ElementState::new(
                            boom_id,
                            ElementKind::BigBoom {
                                content,
                                ticks_left: 2,
                            },
                            Direction::Down,
                        ),
                    ));
                }
                ElementKind::Bomb => {
                    self.remove_element_by_id(id);
                    self.explode_at(cell, events);
                }
                _ if self.tile_at(cell) == Some(TileKind::Ground) => {
                    self.clear_ground_tile(cell, events);
                }
                ElementKind::Screw => {
                    self.remove_element_by_id(id);
                }
                ElementKind::Box | ElementKind::PushBox => {
                    self.remove_element_by_id(id);
                }
                ElementKind::Bear { .. }
                | ElementKind::BlackBear { .. }
                | ElementKind::Bird { .. }
                | ElementKind::Butterfly => {
                    self.remove_element_by_id(id);
                }
                ElementKind::BarbedWire | ElementKind::Stop => {
                    self.remove_element_by_id(id);
                }
                ElementKind::Capsule | ElementKind::Key | ElementKind::BulletPickup => {
                    self.remove_element_by_id(id);
                }
                _ => {}
            }
        }
    }

    /// Objects that block `shoot_object` from placing a new laser segment.
    pub(crate) fn is_laser_shot_immune(kind: &ElementKind) -> bool {
        matches!(
            kind,
            ElementKind::Box
                | ElementKind::PushBox
                | ElementKind::Screw
                | ElementKind::Key
                | ElementKind::Capsule
                | ElementKind::Magnet { .. }
                | ElementKind::Teleport { .. }
                | ElementKind::Gun { .. }
                | ElementKind::Laser { .. }
                | ElementKind::BlasterCell { .. }
                | ElementKind::BarbedWire
                | ElementKind::Stop
        )
    }

    /// Objects destroyed by laser bolt / gun shot (gnurobbo `destroyable`).
    pub(crate) fn is_laser_destroyable(kind: &ElementKind) -> bool {
        matches!(
            kind,
            ElementKind::Box
                | ElementKind::PushBox
                | ElementKind::Bomb
                | ElementKind::QuestionMark { .. }
                | ElementKind::Bear { .. }
                | ElementKind::BlackBear { .. }
                | ElementKind::Bird { .. }
                | ElementKind::Butterfly
                | ElementKind::BarbedWire
                | ElementKind::Stop
        )
    }

    pub(crate) fn is_blaster_immune(kind: &ElementKind) -> bool {
        matches!(
            kind,
            ElementKind::Box
                | ElementKind::PushBox
                | ElementKind::Screw
                | ElementKind::Key
                | ElementKind::Capsule
                | ElementKind::Magnet { .. }
                | ElementKind::Teleport { .. }
                | ElementKind::Gun { .. }
                | ElementKind::Laser { .. }
                | ElementKind::BlasterCell { .. }
                | ElementKind::BarbedWire
                | ElementKind::Stop
        )
    }
}
