#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::command::CommandHistory;
    use crate::element::{ElementKind, ElementState, GunType, QuestionMarkContent};
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
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
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
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
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
        world.turn_robbo(Direction::Right);
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
    fn teleport_partner_blocked_falls_back_to_entry_pass_through() {
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
        // Partner exits blocked; GNU tries entry mirror pass-through (up).
        assert_eq!(world.robbo_cell(), Some(Cell::new(1, 0)));
    }

    #[test]
    fn lone_teleport_passes_through_to_far_side() {
        let w = 6u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::Teleport {
                        group: 3,
                        pair_index: 0,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(3, 1)));
    }

    #[test]
    fn lone_teleport_blocked_when_all_exits_occupied() {
        let w = 6u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[2] = TileKind::WallSolid; // (2, 0)
        tiles[14] = TileKind::WallSolid; // (2, 2)
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::Teleport {
                        group: 3,
                        pair_index: 0,
                    },
                    Direction::Down,
                ),
            ),
            (
                Cell::new(3, 1),
                ElementState::new(3, ElementKind::Box, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(1, 1)));
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
    fn move_and_collect_screw_when_screw_before_robbo_in_vec() {
        let w = 5u16;
        let h = 5u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        // Level load order: screw at index 0, Robbo at index 1.
        let elements = vec![
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Screw, Direction::Down),
            ),
            (Cell::new(2, 2), robbo_at(Cell::new(2, 2))),
        ];
        let mut world = make_world(w, h, tiles, elements, 1);
        let events = world.step(PlayerInput::Move(Direction::Up));
        assert!(events.iter().any(|e| matches!(e, GameEvent::Collected { .. })));
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::Moved {
                entity_id: 1,
                from,
                to
            } if *from == Cell::new(2, 2) && *to == Cell::new(2, 1)
        )));
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
            (Cell::new(1, 2), robbo_at(Cell::new(1, 2))),
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
        world.turn_robbo(Direction::Right);
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
    fn shot_into_solid_wall_wastes_ammo() {
        let w = 5u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[7] = TileKind::WallSolid;
        let elements = vec![(Cell::new(1, 1), robbo_at(Cell::new(1, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 0);
    }

    #[test]
    fn shot_into_box_wastes_ammo_without_destroying() {
        let w = 5u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Box, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 0);
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(2, 1) && matches!(s.kind, ElementKind::Box)));
        assert!(!world
            .elements
            .iter()
            .any(|(_, s)| matches!(s.kind, ElementKind::Laser { .. })));
    }

    #[test]
    fn shot_into_screw_wastes_ammo_without_destroying() {
        let w = 5u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Screw, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 0);
        assert!(world
            .elements
            .iter()
            .any(|(_, s)| matches!(s.kind, ElementKind::Screw)));
    }

    #[test]
    fn bomb_blast_removes_box_not_solid_wall() {
        let w = 6u16;
        let h = 4u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[8] = TileKind::WallSolid; // (2, 1)
        let elements = vec![
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Bomb, Direction::Down),
            ),
            (
                Cell::new(3, 1),
                ElementState::new(3, ElementKind::Box, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.explode_at(Cell::new(2, 1), &mut events);
        assert!(!world
            .elements
            .iter()
            .any(|(_, s)| matches!(s.kind, ElementKind::Box)));
        assert_eq!(world.tile_at(Cell::new(2, 1)), Some(TileKind::WallSolid));
    }

    #[test]
    fn bomb_blast_clears_barrier_tile() {
        let w = 5u16;
        let h = 3u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[7] = TileKind::Barrier; // (2, 1)
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(2, ElementKind::Bomb, Direction::Down),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.explode_at(Cell::new(2, 1), &mut events);
        assert_eq!(world.tile_at(Cell::new(2, 1)), Some(TileKind::Empty));
    }

    #[test]
    fn regular_gun_spawns_single_moving_laser() {
        let w = 8u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(
                2,
                ElementKind::Gun {
                    gun_type: GunType::Regular,
                    direction: Direction::Right,
                    move_dir: Direction::Right,
                    movable: false,
                    rotatable: false,
                    random_rotate: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Regular, Some(2), &mut events, true);
        let lasers: Vec<_> = world
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Laser { solid: false, .. }))
            .collect();
        assert_eq!(lasers.len(), 1);
        assert_eq!(lasers[0].0, Cell::new(3, 1));
    }

    #[test]
    fn blaster_gun_spawns_one_cell_not_beam() {
        let w = 8u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(
                2,
                ElementKind::Gun {
                    gun_type: GunType::Blaster,
                    direction: Direction::Right,
                    move_dir: Direction::Right,
                    movable: false,
                    rotatable: false,
                    random_rotate: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Blaster, Some(2), &mut events, true);
        let blasters: Vec<_> = world
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::BlasterCell { .. }))
            .collect();
        assert_eq!(blasters.len(), 1);
        assert_eq!(blasters[0].0, Cell::new(3, 1));
    }

    #[test]
    fn laser_gun_extends_one_solid_segment_per_shot() {
        let w = 10u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(
                2,
                ElementKind::Gun {
                    gun_type: GunType::Laser,
                    direction: Direction::Right,
                    move_dir: Direction::Right,
                    movable: false,
                    rotatable: false,
                    random_rotate: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Laser, Some(2), &mut events, true);
        let solid: Vec<_> = world
            .elements
            .iter()
            .filter(|(_, s)| matches!(s.kind, ElementKind::Laser { solid: true, .. }))
            .collect();
        assert_eq!(solid.len(), 1);
        assert_eq!(solid[0].0, Cell::new(3, 1));
    }

    #[test]
    fn bear_prefers_left_opening_in_maze() {
        let w = 6u16;
        let h = 5u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[8] = TileKind::WallSolid; // block forward from (1,1) going right
        let elements = vec![(
            Cell::new(1, 1),
            ElementState::new(2, ElementKind::Bear { clockwise: false }, Direction::Right),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.sensible_bears = false;
        for _ in 0..2 {
            world.step(PlayerInput::Wait);
        }
        let bear_cell = world
            .elements
            .iter()
            .find(|(_, s)| matches!(s.kind, ElementKind::Bear { .. }))
            .map(|(c, _)| *c);
        assert_eq!(bear_cell, Some(Cell::new(1, 0)));
    }

    #[test]
    fn laser_gun_beam_extends_over_ticks() {
        let w = 10u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(
                2,
                ElementKind::Gun {
                    gun_type: GunType::Laser,
                    direction: Direction::Right,
                    move_dir: Direction::Right,
                    movable: false,
                    rotatable: false,
                    random_rotate: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Laser, Some(2), &mut events, true);
        for _ in 0..8 {
            world.step(PlayerInput::Wait);
        }
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(4, 1) && matches!(s.kind, ElementKind::Laser { solid: true, .. })));
    }

    #[test]
    fn gun_shoot_on_ground_places_laser() {
        let w = 6u16;
        let h = 4u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[9] = TileKind::Ground; // (3, 1)
        let mut world = make_world(w, h, tiles, vec![], 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Laser, None, &mut events, true);
        assert_eq!(world.tile_at(Cell::new(3, 1)), Some(TileKind::Empty));
        assert!(world.elements.iter().any(|(c, s)| {
            *c == Cell::new(3, 1) && matches!(s.kind, ElementKind::Laser { solid: true, .. })
        }));
    }

    #[test]
    fn laser_gun_stops_at_wall() {
        let w = 8u16;
        let h = 4u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[13] = TileKind::WallSolid; // (5, 1) — index x + y*w = 5 + 8
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(
                2,
                ElementKind::Gun {
                    gun_type: GunType::Laser,
                    direction: Direction::Right,
                    move_dir: Direction::Right,
                    movable: false,
                    rotatable: false,
                    random_rotate: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Laser, Some(2), &mut events, true);
        for _ in 0..2 {
            world.step(PlayerInput::Wait);
        }
        assert!(!world.elements.iter().any(|(c, _)| *c == Cell::new(5, 1)));
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(4, 1) && matches!(s.kind, ElementKind::Laser { solid: true, .. })));
    }

    #[test]
    fn solid_laser_retracts_after_wall_hit() {
        let w = 8u16;
        let h = 4u16;
        let mut tiles = vec![TileKind::Empty; (w * h) as usize];
        tiles[13] = TileKind::WallSolid; // (5, 1) — index x + y*w = 5 + 8
        let elements = vec![(
            Cell::new(2, 1),
            ElementState::new(
                2,
                ElementKind::Gun {
                    gun_type: GunType::Laser,
                    direction: Direction::Right,
                    move_dir: Direction::Right,
                    movable: false,
                    rotatable: false,
                    random_rotate: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.gun_shoot(Cell::new(2, 1), Direction::Right, GunType::Laser, Some(2), &mut events, true);
        for _ in 0..2 {
            world.step(PlayerInput::Wait);
        }
        assert!(world
            .elements
            .iter()
            .any(|(c, s)| *c == Cell::new(4, 1) && matches!(s.kind, ElementKind::Laser { solid: true, .. })));
        for _ in 0..4 {
            world.step(PlayerInput::Wait);
        }
        assert!(!world.elements.iter().any(|(c, s)| {
            *c == Cell::new(4, 1) && matches!(s.kind, ElementKind::Laser { solid: true, .. })
        }));
        assert!(!world
            .elements
            .iter()
            .any(|(c, s)| matches!(s.kind, ElementKind::Laser { solid: true, .. })));
    }

    #[test]
    fn robbo_shot_spawns_laser_not_projectile() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(Cell::new(1, 1), robbo_at(Cell::new(1, 1)))];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert!(world.elements.iter().any(|(_, s)| matches!(
            s.kind,
            ElementKind::Laser { solid: false, .. }
        )));
        assert!(!world
            .elements
            .iter()
            .any(|(_, s)| matches!(s.kind, ElementKind::Projectile { .. })));
    }

    #[test]
    fn robbo_shot_into_laser_immune_gun() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(0, 1), robbo_at(Cell::new(0, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::Gun {
                        gun_type: GunType::Regular,
                        direction: Direction::Left,
                        move_dir: Direction::Left,
                        movable: false,
                        rotatable: false,
                        random_rotate: false,
                    },
                    Direction::Left,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.ammo = 1;
        world.turn_robbo(Direction::Right);
        world.step(PlayerInput::Shoot(Direction::Right));
        assert_eq!(world.ammo, 0);
        assert!(world.elements.iter().any(|(c, s)| {
            *c == Cell::new(2, 1) && matches!(s.kind, ElementKind::Gun { .. })
        }));
    }

    #[test]
    fn adjacent_enemy_kills_robbo() {
        let w = 5u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(
                    2,
                    ElementKind::Bear {
                        clockwise: false,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Wait);
        assert_eq!(world.status, LevelStatus::Failed);
    }

    #[test]
    fn walk_into_enemy_cell_blocked_not_same_cell_death() {
        let w = 6u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(3, 1), robbo_at(Cell::new(3, 1))),
            (
                Cell::new(4, 1),
                ElementState::new(
                    2,
                    ElementKind::Bear {
                        clockwise: false,
                    },
                    Direction::Down,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let mut events = Vec::new();
        world.try_move_robbo(Direction::Right, &mut events);
        assert_eq!(world.robbo_cell(), Some(Cell::new(3, 1)));
        assert_eq!(world.status, LevelStatus::Playing);
        assert!(!events.iter().any(|e| matches!(e, GameEvent::Died { .. })));
    }

    #[test]
    fn blaster_wave_kills_robbo_on_path() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(4, 1), robbo_at(Cell::new(4, 1))),
            (
                Cell::new(3, 1),
                ElementState::new(
                    2,
                    ElementKind::BlasterCell {
                        direction: Direction::Right,
                        frame: 0,
                    },
                    Direction::Right,
                ),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        for _ in 0..4 {
            world.step(PlayerInput::Wait);
        }
        assert_eq!(world.status, LevelStatus::Failed);
    }

    #[test]
    fn bird_patrols_along_direction_reverses_on_wall() {
        let w = 8u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![(
            Cell::new(3, 1),
            ElementState::new(
                2,
                ElementKind::Bird {
                    variant: crate::element::BirdVariant::Horizontal,
                    shooting: false,
                },
                Direction::Right,
            ),
        )];
        let mut world = make_world(w, h, tiles, elements, 0);
        for _ in 0..2 {
            world.step(PlayerInput::Wait);
        }
        assert_eq!(
            world
                .elements
                .iter()
                .find(|(_, s)| matches!(s.kind, ElementKind::Bird { .. }))
                .map(|(c, _)| *c),
            Some(Cell::new(4, 1))
        );
    }

    #[test]
    fn bird_with_shooting_still_patrols() {
        let w = 8u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let mut bird = ElementState::new(
            2,
            ElementKind::Bird {
                variant: crate::element::BirdVariant::Horizontal,
                shooting: true,
            },
            Direction::Right,
        );
        bird.shot_direction = Direction::Down;
        let elements = vec![(Cell::new(2, 1), bird)];
        let mut world = make_world(w, h, tiles, elements, 0);
        for _ in 0..2 {
            world.step(PlayerInput::Wait);
        }
        assert_eq!(
            world
                .elements
                .iter()
                .find(|(_, s)| matches!(s.kind, ElementKind::Bird { .. }))
                .map(|(c, _)| *c),
            Some(Cell::new(3, 1))
        );
    }

    #[test]
    fn capsule_pushable_when_closed() {
        let w = 7u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Capsule, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        assert!(!world.capsule_open);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
        assert!(
            world
                .elements
                .iter()
                .any(|(c, s)| *c == Cell::new(3, 1) && matches!(s.kind, ElementKind::Capsule))
        );
    }

    #[test]
    fn capsule_enters_when_open_not_pushed_aside() {
        let w = 6u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Capsule, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.capsule_open = true;
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.status, LevelStatus::Complete);
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
    }

    #[test]
    fn capsule_push_blocked_when_destination_occupied() {
        let w = 7u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (Cell::new(1, 1), robbo_at(Cell::new(1, 1))),
            (
                Cell::new(2, 1),
                ElementState::new(2, ElementKind::Box, Direction::Down),
            ),
            (
                Cell::new(3, 1),
                ElementState::new(3, ElementKind::Capsule, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.step(PlayerInput::Move(Direction::Right));
        assert_eq!(world.robbo_cell(), Some(Cell::new(1, 1)));
        assert!(
            world
                .elements
                .iter()
                .any(|(c, s)| *c == Cell::new(3, 1) && matches!(s.kind, ElementKind::Capsule))
        );
    }

    #[test]
    fn enemies_cannot_enter_capsule() {
        let w = 6u16;
        let h = 4u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (
                Cell::new(1, 1),
                ElementState::new(
                    2,
                    ElementKind::Bird {
                        variant: crate::element::BirdVariant::Horizontal,
                        shooting: false,
                    },
                    Direction::Right,
                ),
            ),
            (
                Cell::new(2, 1),
                ElementState::new(3, ElementKind::Capsule, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        assert!(world.is_blocked_for_enemy(Cell::new(2, 1)));
        for _ in 0..20 {
            world.step(PlayerInput::Wait);
        }
        assert!(
            world
                .elements
                .iter()
                .all(|(c, s)| !(matches!(s.kind, ElementKind::Bird { .. }) && *c == Cell::new(2, 1)))
        );
    }

    #[test]
    fn magnet_beam_stops_at_wall_and_object() {
        let w = 8u16;
        let h = 3u16;
        let open_tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (
                Cell::new(1, 1),
                ElementState::new(
                    2,
                    ElementKind::Magnet {
                        direction: Direction::Right,
                    },
                    Direction::Right,
                ),
            ),
            (
                Cell::new(4, 1),
                ElementState::new(3, ElementKind::Box, Direction::Down),
            ),
        ];
        let world = make_world(w, h, open_tiles, elements, 0);
        assert_eq!(
            world.magnet_beam_cells(Cell::new(1, 1), Direction::Right),
            vec![
                Cell::new(2, 1),
                Cell::new(3, 1),
                Cell::new(4, 1),
            ]
        );

        let mut tiles_with_wall = vec![TileKind::Empty; (w * h) as usize];
        tiles_with_wall[11] = TileKind::WallGrey; // (3, 1)
        let wall_only = make_world(
            w,
            h,
            tiles_with_wall,
            vec![(
                Cell::new(1, 1),
                ElementState::new(
                    2,
                    ElementKind::Magnet {
                        direction: Direction::Right,
                    },
                    Direction::Right,
                ),
            )],
            0,
        );
        assert_eq!(
            wall_only.magnet_beam_cells(Cell::new(1, 1), Direction::Right),
            vec![Cell::new(2, 1)]
        );
    }

    #[test]
    fn magnet_pulls_one_cell_then_waits_delay() {
        use crate::element::magnet_attract_delay;

        let w = 8u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (
                Cell::new(0, 1),
                ElementState::new(
                    2,
                    ElementKind::Magnet {
                        direction: Direction::Right,
                    },
                    Direction::Right,
                ),
            ),
            (Cell::new(4, 1), robbo_at(Cell::new(4, 1))),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let delay = magnet_attract_delay();

        // Tick 1: beam locks Robbo; no pull yet (GNU magnet scan runs after robbo phase).
        world.step(PlayerInput::Wait);
        assert!(world.robbo_magnet_locked);
        assert_eq!(world.robbo_cell(), Some(Cell::new(4, 1)));

        // Tick 2: first pull step toward magnet.
        world.step(PlayerInput::Wait);
        assert_eq!(world.robbo_cell(), Some(Cell::new(3, 1)));

        // Cooldown ticks: no movement.
        for _ in 0..delay - 1 {
            world.step(PlayerInput::Wait);
            assert_eq!(world.robbo_cell(), Some(Cell::new(3, 1)));
        }

        // Next allowed pull.
        world.step(PlayerInput::Wait);
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
    }

    #[test]
    fn opposing_magnets_first_in_scan_order_wins() {
        let w = 8u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        // Left magnet (0,1) scanned before right magnet (6,1); both beams hit Robbo at (3,1).
        let elements = vec![
            (
                Cell::new(0, 1),
                ElementState::new(
                    2,
                    ElementKind::Magnet {
                        direction: Direction::Right,
                    },
                    Direction::Right,
                ),
            ),
            (
                Cell::new(6, 1),
                ElementState::new(
                    3,
                    ElementKind::Magnet {
                        direction: Direction::Left,
                    },
                    Direction::Left,
                ),
            ),
            (Cell::new(3, 1), robbo_at(Cell::new(3, 1))),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);

        world.step(PlayerInput::Wait);
        assert!(world.robbo_magnet_locked);
        assert_eq!(world.magnet_pull_dir, Direction::Left);

        world.step(PlayerInput::Wait);
        // Pulled toward left magnet, not right.
        assert_eq!(world.robbo_cell(), Some(Cell::new(2, 1)));
    }

    #[test]
    fn adjacent_bombs_chain_with_delay_not_same_tick() {
        use crate::element::bomb_chain_delay_ticks;

        let w = 6u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (
                Cell::new(1, 1),
                ElementState::new(2, ElementKind::Bomb, Direction::Down),
            ),
            (
                Cell::new(2, 1),
                ElementState::new(3, ElementKind::Bomb, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        let delay = bomb_chain_delay_ticks();

        world.detonate_bomb(Cell::new(1, 1), &mut Vec::new());
        assert!(
            !world
                .elements
                .iter()
                .any(|(c, s)| *c == Cell::new(1, 1) && matches!(s.kind, ElementKind::Bomb)),
            "first bomb gone"
        );
        assert!(
            world
                .elements
                .iter()
                .any(|(c, s)| *c == Cell::new(2, 1) && matches!(s.kind, ElementKind::Bomb)),
            "neighbor bomb still present"
        );

        for _ in 0..delay.saturating_sub(1) {
            world.step(PlayerInput::Wait);
            assert!(
                world
                    .elements
                    .iter()
                    .any(|(c, s)| *c == Cell::new(2, 1) && matches!(s.kind, ElementKind::Bomb)),
                "chain bomb waits before detonating"
            );
        }

        world.step(PlayerInput::Wait);
        assert!(
            !world
                .elements
                .iter()
                .any(|(c, s)| *c == Cell::new(2, 1) && matches!(s.kind, ElementKind::Bomb)),
            "second bomb detonates after delay"
        );
    }

    #[test]
    fn bomb_does_not_destroy_capsule() {
        let w = 5u16;
        let h = 3u16;
        let tiles = vec![TileKind::Empty; (w * h) as usize];
        let elements = vec![
            (
                Cell::new(1, 1),
                ElementState::new(2, ElementKind::Bomb, Direction::Down),
            ),
            (
                Cell::new(2, 1),
                ElementState::new(3, ElementKind::Capsule, Direction::Down),
            ),
        ];
        let mut world = make_world(w, h, tiles, elements, 0);
        world.explode_at(Cell::new(1, 1), &mut Vec::new());
        assert!(
            world
                .elements
                .iter()
                .any(|(_, s)| matches!(s.kind, ElementKind::Capsule))
        );
    }
}
