use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cell {
    pub col: i16,
    pub row: i16,
}

impl Cell {
    pub const fn new(col: i16, row: i16) -> Self {
        Self { col, row }
    }

    pub fn offset(self, dc: i16, dr: i16) -> Self {
        Self {
            col: self.col + dc,
            row: self.row + dr,
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.col, self.row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_works() {
        let c = Cell::new(3, 4).offset(1, -1);
        assert_eq!(c, Cell::new(4, 3));
    }
}
