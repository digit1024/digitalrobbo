use crate::world::World;

/// Undo/redo history via world snapshots.
#[derive(Clone, Debug, Default)]
pub struct CommandHistory {
    undo: Vec<World>,
    redo: Vec<World>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn record(&mut self, before: World) {
        self.undo.push(before);
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: World) -> Option<World> {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: World) -> Option<World> {
        if let Some(next) = self.redo.pop() {
            self.undo.push(current);
            Some(next)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    #[test]
    fn undo_redo_roundtrip() {
        let mut history = CommandHistory::new();
        let w1 = World::empty(4, 4);
        let mut w2 = w1.clone();
        w2.ammo = 5;
        history.record(w1.clone());
        let restored = history.undo(w2).expect("undo");
        assert_eq!(restored.ammo, 0);
        let again = history.redo(restored).expect("redo");
        assert_eq!(again.ammo, 5);
    }
}
