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
    /// gnurobbo STOP (`X`) — walk clears to empty.
    Stop,
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
                | TileKind::Ground
                | TileKind::DoorClosed
                | TileKind::Barrier
                | TileKind::Stop
        )
    }

    pub fn blocks_shot(self) -> bool {
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

    pub fn is_barrier(self) -> bool {
        matches!(self, TileKind::Barrier)
    }
}
