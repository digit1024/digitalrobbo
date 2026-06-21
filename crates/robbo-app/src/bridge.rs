use std::collections::HashMap;

use bevy::prelude::*;
use robbo_core::{Direction, GameEvent, PlayerInput};

use crate::app_state::AppState;
use crate::input::SteeringState;
use crate::ui::LevelCountdown;

pub const TICK_SECS: f32 = 0.1; // ~7 ticks/s — enemies/guns advance at this rate
/// Visual step length — always equals one sim tick (Robbo, enemies, bullets, pushed boxes).
pub const ANIM_SECS: f32 = TICK_SECS;

#[derive(Event, Clone, Debug)]
pub struct CoreGameEvent(pub GameEvent);

#[derive(Event, Clone, Debug, Default)]
pub struct LoadLevelEvent {
    pub restart: bool,
}

#[derive(Event, Clone, Debug, Default)]
pub struct ReloadVisualsEvent;

#[derive(Resource, Default)]
pub struct GameSession {
    pub pack_name: String,
    pub level_index: usize,
    pub level_label: String,
}

#[derive(Resource, Default)]
pub struct EntityMap(pub HashMap<u32, Entity>);

#[derive(Resource, Default)]
pub struct TileEntityMap(pub HashMap<robbo_core::Cell, Entity>);

/// Drives the game simulation at a fixed tick rate.
#[derive(Resource)]
pub struct GameTickTimer(pub Timer);

impl Default for GameTickTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(TICK_SECS, TimerMode::Repeating))
    }
}

#[derive(Resource)]
pub struct CoreBridge {
    pub world: robbo_core::World,
    pub history: robbo_core::CommandHistory,
    pub events_queue: Vec<GameEvent>,
    /// Direction Robbo last actually moved (used for shooting).
    pub last_direction: Direction,
    /// Visual facing — updated immediately on turn or move.
    pub facing_direction: Direction,
    /// Which frame of the 6-frame walk cycle is currently shown (0-5).
    pub walk_frame: usize,
}

impl Default for CoreBridge {
    fn default() -> Self {
        Self {
            world: robbo_core::World::empty(16, 16),
            history: robbo_core::CommandHistory::new(),
            events_queue: Vec::new(),
            last_direction: Direction::Down,
            facing_direction: Direction::Down,
            walk_frame: 0,
        }
    }
}

/// Core game tick: fire every TICK_SECS regardless of visual animation.
pub fn game_tick_system(
    mut bridge: ResMut<CoreBridge>,
    mut steering: ResMut<SteeringState>,
    mut tick_timer: ResMut<GameTickTimer>,
    mut core_events: EventWriter<CoreGameEvent>,
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    time: Res<Time>,
) {
    if *state.get() != AppState::Playing || countdown.blocks_input() {
        return;
    }

    tick_timer.0.tick(time.delta());
    if !tick_timer.0.just_finished() {
        return;
    }

    let input = if steering.shoot_pending {
        steering.shoot_pending = false;
        PlayerInput::Shoot(bridge.last_direction)
    } else if let Some(dir) = steering.hold {
        PlayerInput::Move(dir)
    } else if let Some(dir) = steering.tap_move.take() {
        PlayerInput::Move(dir)
    } else {
        PlayerInput::Wait
    };

    if let PlayerInput::Move(dir) = input {
        bridge.last_direction = dir;
        bridge.facing_direction = dir;
    }

    let before = bridge.world.snapshot();
    let events = bridge.world.step(input);
    if !events.is_empty() {
        bridge.history.record(before);
    }

    let robbo_id = bridge.world.robbo_id;
    let robbo_moved = events
        .iter()
        .any(|e| matches!(e, GameEvent::Moved { entity_id, .. } if *entity_id == robbo_id));
    if robbo_moved {
        bridge.walk_frame = (bridge.walk_frame + 1) % 6;
    }

    for e in &events {
        bevy::log::debug!("core event: {e:?}");
        core_events.send(CoreGameEvent(e.clone()));
    }
    bridge.events_queue = events;
}
