use std::collections::HashMap;

use bevy::prelude::*;
use robbo_core::{Direction, GameEvent, PlayerInput};

use crate::app_state::AppState;
use crate::ui::LevelCountdown;

pub const TICK_SECS: f32 = 0.14;  // ~7 ticks/s — enemies/guns advance at this rate
pub const ANIM_SECS: f32 = 0.10;  // visual tween duration (must be < TICK_SECS)

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
    /// Input for the NEXT tick (filled by keyboard_input, consumed by tick system).
    pub pending_input: Option<PlayerInput>,
    /// Input buffered during animation (promoted to pending when anim ends).
    pub queued_input: Option<PlayerInput>,
    pub animating: bool,
    pub events_queue: Vec<GameEvent>,
    /// Direction Robbo last actually moved (used for shooting).
    pub last_direction: Direction,
    /// Visual facing — updated *immediately* on keypress, no tick needed.
    pub facing_direction: Direction,
    /// Which frame of the 6-frame walk cycle is currently shown (0-5).
    pub walk_frame: usize,
}

impl Default for CoreBridge {
    fn default() -> Self {
        Self {
            world: robbo_core::World::empty(16, 16),
            history: robbo_core::CommandHistory::new(),
            pending_input: None,
            queued_input: None,
            animating: false,
            events_queue: Vec::new(),
            last_direction: Direction::Down,
            facing_direction: Direction::Down,
            walk_frame: 0,
        }
    }
}

/// Buffer player input that arrives while a visual tween is running.
pub fn buffer_input_while_animating(mut bridge: ResMut<CoreBridge>, state: Res<State<AppState>>) {
    if *state.get() != AppState::Playing || !bridge.animating {
        return;
    }
    if let Some(input) = bridge.pending_input.take() {
        bridge.queued_input = Some(input);
    }
}

/// Core game tick: fire every TICK_SECS (or immediately on player input).
/// Uses the buffered player input if available, otherwise sends PlayerInput::Wait
/// so enemies, guns, and bullets advance even when Robbo stands still.
pub fn game_tick_system(
    mut bridge: ResMut<CoreBridge>,
    mut tick_timer: ResMut<GameTickTimer>,
    mut core_events: EventWriter<CoreGameEvent>,
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    time: Res<Time>,
) {
    if *state.get() != AppState::Playing || countdown.blocks_input() {
        return;
    }
    if bridge.animating {
        // Timer keeps ticking but we can't step while animating.
        tick_timer.0.tick(time.delta());
        return;
    }

    let has_player_input = bridge.pending_input.is_some();
    tick_timer.0.tick(time.delta());
    let timer_fired = tick_timer.0.just_finished();

    // Step when: the player just gave input (immediate) OR the tick timer fired.
    if !has_player_input && !timer_fired {
        return;
    }
    // Player input resets the timer so the next auto-tick is a full interval away.
    if has_player_input {
        tick_timer.0.reset();
    }

    let input = bridge.pending_input.take().unwrap_or(PlayerInput::Wait);
    if let PlayerInput::Move(dir) = input {
        bridge.last_direction = dir;
        bridge.facing_direction = dir; // keep in sync on actual movement
    }

    let before = bridge.world.snapshot();
    let events = bridge.world.step(input);
    if !events.is_empty() {
        bridge.history.record(before);
    }

    // Advance walk cycle when Robbo actually moved (not blocked).
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
    if !bridge.events_queue.is_empty() {
        bridge.animating = true;
    }
}

/// When animation completes, promote buffered input to pending so the next
/// tick can use it immediately.
pub fn release_queued_input(mut bridge: ResMut<CoreBridge>) {
    if !bridge.animating && bridge.pending_input.is_none() {
        if let Some(q) = bridge.queued_input.take() {
            bridge.pending_input = Some(q);
        }
    }
}
