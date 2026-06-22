use bevy::prelude::*;
use robbo_core::{Direction, ElementKind, GunType, TileKind};

pub const TILE_PX: f32 = 80.0;

/// Pre-loaded image handles for every game element.
/// Insert as a Resource at startup, then pass into render systems.
#[derive(Resource)]
pub struct SpriteAssets {
    // ── Robbo walk cycle: [dir_idx][frame 0-5]
    // dir_idx: right=0, down=1, left=2, up=3
    pub player: [[Handle<Image>; 6]; 4],
    // ── entities ─────────────────────────────────────────────
    // (Robbo uses self.player[]; the separate `robbo` field was removed)
    pub screw: Handle<Image>,
    pub capsule: Handle<Image>,
    pub capsule_ready: Handle<Image>,
    pub bomb: Handle<Image>,
    pub bx: Handle<Image>,
    pub push_box: Handle<Image>,
    pub key: Handle<Image>,
    pub bullet_pickup: Handle<Image>,
    pub extra_life: Handle<Image>,
    pub question_mark: Handle<Image>,
    /// Robbo / gun / cannon shots (lasers, beams, bolts).
    pub bullet: Handle<Image>,
    pub teleport: Handle<Image>,
    pub magnet: Handle<Image>,
    /// Bear sprites always face up; rotation is applied at render time.
    pub bear_up: Handle<Image>,
    pub blackbear_up: Handle<Image>,
    pub butterfly: Handle<Image>,
    pub bird: Handle<Image>,
    // guns: [right, down, left, up]
    pub gun: [Handle<Image>; 4],
    // ── tiles ────────────────────────────────────────────────
    pub tile_empty: Handle<Image>,
    pub tile_wall_grey: Handle<Image>,
    pub tile_wall_solid: Handle<Image>,
    pub tile_wall_red: Handle<Image>,
    pub tile_wall_green: Handle<Image>,
    pub tile_wall_black: Handle<Image>,
    pub tile_ground: Handle<Image>,
    pub tile_door_closed: Handle<Image>,
    pub tile_barrier: Handle<Image>,
}

impl SpriteAssets {
    /// Player walk frame: dir_idx matches `dir_to_idx`, frame is 0-5.
    pub fn player_frame(&self, dir: Direction, frame: usize) -> Handle<Image> {
        self.player[dir_to_idx(dir)][frame % 6].clone()
    }

    pub fn load(server: &AssetServer) -> Self {
        let sp = |name: &str| server.load(format!("sprites/{name}.png"));
        // dir order: right=0, down=1, left=2, up=3  (matches dir_to_idx)
        let dir_names = ["right", "down", "left", "up"];
        let player = std::array::from_fn(|di| {
            std::array::from_fn(|fi| {
                server.load(format!("sprites/player/{}_{:02}.png", dir_names[di], fi + 1))
            })
        });
        Self {
            player,
            screw:        sp("screw"),
            capsule:      sp("capsule"),
            capsule_ready: sp("capsule_ready"),
            bomb:         sp("bomb"),
            bx:           sp("box"),
            push_box:     sp("push_box"),
            key:          sp("key"),
            bullet_pickup: sp("bullet_pickup"),
            extra_life:   sp("extra_life"),
            question_mark: sp("question_mark"),
            bullet:       sp("bullet"),
            teleport:     sp("teleport"),
            magnet:       sp("magnet"),
            bear_up: sp("bear_up"),
            blackbear_up: sp("baar_2Up"),
            butterfly: sp("butterfly"),
            bird:         sp("bird"),
            gun:   [sp("gun_right"), sp("gun_down"), sp("gun_left"), sp("gun_up")],
            tile_empty:       sp("tile_ground"),
            tile_wall_grey:   sp("tile_wall_grey"),
            tile_wall_solid:  sp("tile_wall_solid"),
            tile_wall_red:    sp("tile_wall_a"),
            tile_wall_green:  sp("tile_wall_b"),
            tile_wall_black:  sp("tile_wall_c"),
            tile_ground:      sp("dirt"),
            tile_door_closed: sp("door"),
            tile_barrier:     sp("tile_barrier"),
        }
    }

    pub fn for_element(&self, kind: &ElementKind, dir: Direction) -> Handle<Image> {
        let dir_idx = dir_to_idx(dir);
        match kind {
            ElementKind::Robbo => self.player[dir_idx][0].clone(),
            ElementKind::Screw => self.screw.clone(),
            ElementKind::Capsule => self.capsule.clone(),
            ElementKind::Bomb => self.bomb.clone(),
            ElementKind::Box => self.bx.clone(),
            ElementKind::PushBox => self.push_box.clone(),
            ElementKind::Key => self.key.clone(),
            ElementKind::BulletPickup => self.bullet_pickup.clone(),
            ElementKind::QuestionMark { .. } => self.question_mark.clone(),
            ElementKind::Projectile { .. } => self.bullet.clone(),
            ElementKind::Teleport { .. } => self.teleport.clone(),
            ElementKind::Magnet { .. } => self.magnet.clone(),
            ElementKind::Bear { .. } => self.bear_up.clone(),
            ElementKind::BlackBear { .. } => self.blackbear_up.clone(),
            ElementKind::Butterfly => self.butterfly.clone(),
            ElementKind::Bird { .. } => self.bird.clone(),
            ElementKind::Gun { direction, .. } => self.gun[dir_to_idx(*direction)].clone(),
            ElementKind::Laser { .. } | ElementKind::BlasterCell { .. } => self.bullet.clone(),
            ElementKind::BigBoom { .. } => self.bomb.clone(),
            ElementKind::BarbedWire => self.tile_wall_black.clone(),
            ElementKind::Stop => self.tile_empty.clone(),
        }
    }

    /// Sprite shown on the collect pop (key, screw, ammo, life).
    pub fn for_collectible(&self, kind: &ElementKind) -> Option<Handle<Image>> {
        match kind {
            ElementKind::Screw => Some(self.screw.clone()),
            ElementKind::Key => Some(self.key.clone()),
            ElementKind::BulletPickup => Some(self.bullet_pickup.clone()),
            _ => None,
        }
    }

    pub fn for_tile(&self, tile: TileKind) -> Option<Handle<Image>> {
        Some(match tile {
            TileKind::Empty       => self.tile_empty.clone(),
            TileKind::WallGrey    => self.tile_wall_grey.clone(),
            TileKind::WallSolid   => self.tile_wall_solid.clone(),
            TileKind::WallRed     => self.tile_wall_red.clone(),
            TileKind::WallGreen   => self.tile_wall_green.clone(),
            TileKind::WallBlack   => self.tile_wall_black.clone(),
            TileKind::Ground      => self.tile_ground.clone(),
            TileKind::DoorClosed | TileKind::DoorOpen => self.tile_door_closed.clone(),
            TileKind::Barrier     => self.tile_barrier.clone(),
            TileKind::Stop        => self.tile_wall_red.clone(),
        })
    }
}

/// Z-rotation (radians) for a magnet sprite whose poles face up in the art.
pub fn magnet_direction_rotation(dir: Direction) -> f32 {
    bear_direction_rotation(dir)
}

/// Z-rotation (radians) for a bear sprite whose base art faces up.
pub fn bear_direction_rotation(dir: Direction) -> f32 {
    match dir {
        Direction::Up => 0.0,
        Direction::Right => -std::f32::consts::FRAC_PI_2,
        Direction::Down => std::f32::consts::PI,
        Direction::Left => std::f32::consts::FRAC_PI_2,
    }
}

/// Direction → index into directional sprite arrays.
/// Order: right=0, down=1, left=2, up=3  (matches extracted gun/bear sheets)
pub fn dir_to_idx(dir: Direction) -> usize {
    match dir {
        Direction::Right => 0,
        Direction::Down  => 1,
        Direction::Left  => 2,
        Direction::Up    => 3,
    }
}
