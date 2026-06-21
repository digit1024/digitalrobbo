#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TileKind {
    #[default]
    Empty,
    WallGrey,
    WallGreen,
    WallBlack,
    WallRed,
    WallSolid,
    Ground,
    DoorClosed,
    DoorOpen,
    Barrier,
}

impl TileKind {
    pub fn blocks_movement(self) -> bool {
        matches!(
            self,
            TileKind::WallGrey
                | TileKind::WallGreen
                | TileKind::WallBlack
                | TileKind::WallRed
                | TileKind::WallSolid
                | TileKind::DoorClosed
                | TileKind::Barrier
        )
    }

    pub fn blocks_shot(self) -> bool {
        self.blocks_movement()
    }
}
