use bevy::prelude::*;
use robbo_core::Cell;

pub trait GridProjection: Send + Sync {
    fn cell_to_world(&self, cell: Cell, layer: f32) -> Vec3;
    fn sort_key(&self, cell: Cell) -> f32;
    fn tile_size(&self) -> f32;
}

pub struct TopDownProjection {
    pub tile_size: f32,
}

impl Default for TopDownProjection {
    fn default() -> Self {
        Self { tile_size: crate::assets::TILE_PX }
    }
}

impl GridProjection for TopDownProjection {
    fn cell_to_world(&self, cell: Cell, layer: f32) -> Vec3 {
        Vec3::new(
            cell.col as f32 * self.tile_size,
            -cell.row as f32 * self.tile_size,
            layer,
        )
    }

    fn sort_key(&self, cell: Cell) -> f32 {
        cell.row as f32
    }

    fn tile_size(&self) -> f32 {
        self.tile_size
    }
}

/// Isometric projection for M7 — swap via resource without touching core logic.
pub struct IsometricProjection {
    pub tile_width: f32,
    pub tile_height: f32,
}

impl Default for IsometricProjection {
    fn default() -> Self {
        Self {
            tile_width: 64.0,
            tile_height: 32.0,
        }
    }
}

impl GridProjection for IsometricProjection {
    fn cell_to_world(&self, cell: Cell, layer: f32) -> Vec3 {
        let x = (cell.col - cell.row) as f32 * self.tile_width * 0.5;
        let y = -(cell.col + cell.row) as f32 * self.tile_height * 0.5;
        Vec3::new(x, y, layer)
    }

    fn sort_key(&self, cell: Cell) -> f32 {
        (cell.col + cell.row) as f32
    }

    fn tile_size(&self) -> f32 {
        self.tile_width
    }
}

#[derive(Resource)]
pub struct ActiveProjection {
    pub top_down: TopDownProjection,
    pub isometric: IsometricProjection,
    pub use_isometric: bool,
}

impl Default for ActiveProjection {
    fn default() -> Self {
        Self {
            top_down: TopDownProjection::default(),
            isometric: IsometricProjection::default(),
            use_isometric: false,
        }
    }
}

impl ActiveProjection {
    pub fn project(&self, cell: Cell, layer: f32) -> Vec3 {
        if self.use_isometric {
            self.isometric.cell_to_world(cell, layer)
        } else {
            self.top_down.cell_to_world(cell, layer)
        }
    }

    pub fn tile_size(&self) -> f32 {
        if self.use_isometric {
            self.isometric.tile_size()
        } else {
            self.top_down.tile_size()
        }
    }
}
