#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::command::CommandHistory;
    use crate::element::{ElementKind, ElementState, QuestionMarkContent};
    use crate::events::{DeathCause, GameEvent, PlayerInput};
    use crate::tile::TileKind;
    use crate::world::{LevelStatus, World};

    fn robbo_at(cell: Cell) -> ElementState {
        ElementState::new(1, ElementKind::Robbo, Direction::Down)
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
    fn ammo_pickup_gives_nine() {
        let w = 5u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::BulletPickup, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.ammo, 9);
    }

    #[test]
    fn door_opens_on_step_consumes_key_robbo_stays() {
        let w = 5u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[7] = TileKind::DoorClosed;
        let elements = vec![(Cell::new(1, 1), robbo_at(Cell::new(1, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.keys = 1;
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(1, 1)));
        assert_eq!(world.keys, 0);
        assert_eq!(world.tile_at(Cell::new(2, 1)), Some(TileKind::Empty));
    }

    #[test]
    fn questionmark_pushable_not_collectable() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::QuestionMark {
                        content: QuestionMarkContent::Screw,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert!(world.elements.iter().any(|(_, s)| matches!(
            s.kind,
            ElementKind::QuestionMark { .. }
        )));
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
    }

    #[test]
    fn questionmark_shot_spawns_hidden_content() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::QuestionMark {
                        content: QuestionMarkContent::Screw,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.step(PlayerInput::Shoot(Direction::Right));
        world.step(PlayerInput::Wait);
        world.step(PlayerInput::Wait);
        assert!(world.elements.iter().any(|(_, s)| matches!(s.kind, ElementKind::Screw)));
    }

    #[test]
    fn questionmark_bomb_no_reveal() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::Bomb,
                    Direction::Down,
                ),
            ),
            (
                Cell::new(3, 1),
                ElementState::new(
                    3,
                    ElementKind::QuestionMark {
                        content: QuestionMarkContent::Screw,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.step(PlayerInput::Shoot(Direction::Right));
        assert!(!world.elements.iter().any(|(_, s)| matches!(
            s.kind,
            ElementKind::QuestionMark { .. } | ElementKind::Screw | ElementKind::BigBoom { .. }
        )));
    }

    #[test]
    fn barbed_wire_kills_on_walk() {
        let w = 5u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(1, 1),
                ElementState::new(2, ElementKind::BarbedWire, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.status, LevelStatus::Failed);
    }

    #[test]
    fn pushbox_slides_when_pushed() {
        let w = 8u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::PushBox, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(3, 1) && matches!(s.kind, ElementKind::PushBox)));
    }

    #[test]
    fn teleport_ring_advances_index() {
        let w = 8u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(1, 1),
                ElementState::new(
                    2,
                    ElementKind::Teleport {
                        group: 1,
                        pair_index: 0,
                    },
                    Direction::Down,
                ),
            ),
            (
                Cell::new(4, 1),
                ElementState::new(
                    3,
                    ElementKind::Teleport {
                        group: 1,
                        pair_index: 1,
                    },
                    Direction::Down,
                ),
            ),
            (
                Cell::new(6, 1),
                ElementState::new(
                    4,
                    ElementKind::Teleport {
                        group: 1,
                        pair_index: 2,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(5, 1)));
    }

    #[test]
    fn teleport_blocked_exit_stays() {
        let w = 6u16;
        let h = 5u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        for idx in [3, 8, 10, 15] {
            tiles[idx] = TileKind::WallSolid;
        }
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(1, 1),
                ElementState::new(
                    2,
                    ElementKind::Teleport {
                        group: 1,
                        pair_index: 0,
                    },
                    Direction::Down,
                ),
            ),
            (
                Cell::new(3, 1),
                ElementState::new(
                    3,
                    ElementKind::Teleport {
                        group: 1,
                        pair_index: 1,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(0, 1)));
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
                ElementState::new(2, ElementKind::Screw, Direction::Down),
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
                ElementState::new(2, ElementKind::Box, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let events = world.step(PlayerInput::Move(Direction::Right));
        assert!(events.iter().any(|e| matches!(e, GameEvent::Pushed { .. })));
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
    }

    #[test]
    fn bomb_explodes_full_3x3_area() {
        let w = 6u16;
        let h = 5u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let box_at = |id: u32, col: i16, row: i16| {
            (
                Cell::new(col, row),
                ElementState::new(id, ElementKind::Box, Direction::Down),
            )
        };
        let elements = vec![
            (Cell::new(0, 2), robbo_at(Cell::new(0, 2))),
            (
                Cell::new(2, 2),
                ElementState::new(2, ElementKind::Bomb, Direction::Down),
            ),
            box_at(3, 1, 1),
            box_at(4, 3, 2),
            box_at(5, 2, 3),
            box_at(6, 4, 2),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.step(PlayerInput::Shoot(Direction::Right));
        assert!(world.elements.iter().all(|(_, s)| !matches!(s.kind, ElementKind::Bomb)));
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(4, 2) && matches!(s.kind, ElementKind::Box)));
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
                ElementState::new(2, ElementKind::Screw, Direction::Down),
            ),
        ];
        let mut w1 = make_world(w, h, tiles.clone(), elements.clone(), 1);
        let mut w2 = make_world(w, h, tiles, elements, 1);
        w1.step(PlayerInput::Move(Direction::Up));
        w2.step(PlayerInput::Move(Direction::Up));
        assert_eq!(w1.state_hash(), w2.state_hash());
    }

    #[test]
    fn turn_robbo_updates_facing_without_move() {
        let w = 5u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(Cell::new(2, 2), robbo_at(Cell::new(2, 2)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.turn_robbo(Direction::Left);
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 2)));
        let robbo = world
            .elements
            .iter()
            .find(|(_, s)| matches!(s.kind, ElementKind::Robbo))
            .unwrap();
        assert_eq!(robbo.1.direction, Direction::Left);
    }

    #[test]
    fn adjacent_shot_explodes_bomb_same_tick() {
        let w = 5u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Bomb, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 0);
        assert!(!world
            .elements
            .iter()
            .any(|(_, s)| matches!(s.kind, ElementKind::Bomb)));
    }

    #[test]
    fn adjacent_shot_destroys_ground_same_tick() {
        let w = 5u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[7] = TileKind::Ground;
        let elements = vec![(Cell::new(1, 1), robbo_at(Cell::new(1, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 0);
        assert_eq!(world.tile_at(Cell::new(2, 1)), Some(TileKind::Empty));
    }

    #[test]
    fn adjacent_shot_into_solid_wall_no_ammo_spent() {
        let w = 5u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[7] = TileKind::WallSolid;
        let elements = vec![(Cell::new(1, 1), robbo_at(Cell::new(1, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 1);
    }
}
