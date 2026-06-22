use crate::cell::Cell;
use crate::direction::Direction;
use crate::element::{ElementKind, ElementState, GunType, QuestionMarkContent};
use crate::events::GameEvent;
use crate::world::World;

impl World {
    pub fn init_questionmarks(&mut self) {
        for (_, state) in &mut self.elements {
            if let ElementKind::QuestionMark { content } = &mut state.kind {
                *content = QuestionMarkContent::roll(&mut self.rng_state, self.sensible_questionmarks);
            }
        }
    }

    pub fn spawn_question_content(
        &mut self,
        at: Cell,
        content: QuestionMarkContent,
        events: &mut Vec<GameEvent>,
    ) {
        let content = if matches!(content, QuestionMarkContent::QuestionMark) {
            QuestionMarkContent::roll(&mut self.rng_state, self.sensible_questionmarks)
        } else {
            content
        };

        match content {
            QuestionMarkContent::Empty => {}
            QuestionMarkContent::Ground => {
                self.set_tile(at, crate::tile::TileKind::Ground);
            }
            QuestionMarkContent::Capsule => {
                self.open_capsule();
                let id = self.allocate_id();
                self.elements.push((
                    at,
                    ElementState::new(id, ElementKind::Capsule, Direction::Down),
                ));
            }
            QuestionMarkContent::Gun => {
                let dir = Self::random_direction(&mut self.rng_state);
                let id = self.allocate_id();
                self.elements.push((
                    at,
                    ElementState::new(
                        id,
                        ElementKind::Gun {
                            gun_type: GunType::Regular,
                            direction: dir,
                            move_dir: dir,
                            movable: false,
                            rotatable: true,
                            random_rotate: true,
                        },
                        dir,
                    ),
                ));
            }
            QuestionMarkContent::QuestionMark => {
                let inner = QuestionMarkContent::roll(&mut self.rng_state, self.sensible_questionmarks);
                let id = self.allocate_id();
                self.elements.push((
                    at,
                    ElementState::new(
                        id,
                        ElementKind::QuestionMark { content: inner },
                        Direction::Down,
                    ),
                ));
            }
            other => {
                let kind = match other {
                    QuestionMarkContent::PushBox => ElementKind::PushBox,
                    QuestionMarkContent::Screw => ElementKind::Screw,
                    QuestionMarkContent::BulletPickup => ElementKind::BulletPickup,
                    QuestionMarkContent::Key => ElementKind::Key,
                    QuestionMarkContent::Bomb => ElementKind::Bomb,
                    QuestionMarkContent::Butterfly => ElementKind::Butterfly,
                    _ => unreachable!(),
                };
                let direction = if matches!(kind, ElementKind::Butterfly) {
                    Direction::Right
                } else {
                    Direction::Down
                };
                let id = self.allocate_id();
                self.elements.push((at, ElementState::new(id, kind, direction)));
            }
        }
        events.push(GameEvent::Revealed { at });
    }

    pub fn open_capsule(&mut self) {
        self.capsule_open = true;
    }

    fn random_direction(rng: &mut u64) -> Direction {
        let dirs = [
            Direction::Right,
            Direction::Down,
            Direction::Left,
            Direction::Up,
        ];
        dirs[(crate::element::next_rand(rng) as usize) % dirs.len()]
    }
}
