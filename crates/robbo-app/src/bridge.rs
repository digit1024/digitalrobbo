use std::collections::HashMap;

use bevy::prelude::*;
use robbo_core::{Direction, GameEvent, PlayerInput};

use crate::app_state::AppState;
use crate::input::{player_move_on_tick, SteeringState};
use crate::ui::LevelCountdown;

pub const TICK_SECS: f32 = 0.1; // 10 Hz sim — enemy delays scaled from GNU 25 Hz
/// Visual step length — always equals one sim tick (Robbo, enemies, bullets, pushed boxes).
pub const ANIM_SECS: f32 = TICK_SECS;

#[derive(Message, Clone, Debug)]
pub struct CoreGameEvent(pub GameEvent);

#[derive(Message, Clone, Debug, Default)]
pub struct LoadLevelEvent {
    pub restart: bool,
}

#[derive(Message, Clone, Debug, Default)]
pub struct ReloadVisualsEvent;

#[derive(Resource, Default, Debug)]
pub struct RenderAudit {
    pub interval_frames: u32,
    pub interval_spawned: u32,
    pub interval_despawned: u32,
    pub interval_reload_events: u32,
    pub interval_full_rebuilds: u32,
    pub interval_max_frame_ms: f32,
    pub interval_elapsed: f32,
    pub total_state_transitions: u32,
}

pub fn audit_frame_timing(
    time: Res<Time>,
    state: Res<State<AppState>>,
    mut audit: ResMut<RenderAudit>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    let dt_ms = time.delta_secs() * 1000.0;
    audit.interval_frames += 1;
    audit.interval_elapsed += time.delta_secs();
    audit.interval_max_frame_ms = audit.interval_max_frame_ms.max(dt_ms);
}

pub fn audit_reload_events(
    mut reload: MessageReader<ReloadVisualsEvent>,
    state: Res<State<AppState>>,
    mut audit: ResMut<RenderAudit>,
) {
    if *state.get() != AppState::Playing {
        return;
    }
    let mut count = 0u32;
    for _ in reload.read() {
        count += 1;
    }
    audit.interval_reload_events += count;
}

pub fn audit_state_transitions(
    state: Res<State<AppState>>,
    mut last_state: Local<Option<AppState>>,
    mut audit: ResMut<RenderAudit>,
) {
    let current = state.get().clone();
    match last_state.as_ref() {
        Some(prev) if *prev != current => {
            audit.total_state_transitions += 1;
            *last_state = Some(current);
        }
        None => {
            *last_state = Some(current);
        }
        _ => {}
    }
}

pub fn audit_report(mut audit: ResMut<RenderAudit>, state: Res<State<AppState>>) {
    if *state.get() != AppState::Playing || audit.interval_elapsed < 2.0 {
        return;
    }
    let fps = if audit.interval_elapsed > 0.0 {
        audit.interval_frames as f32 / audit.interval_elapsed
    } else {
        0.0
    };
    bevy::log::info!(
        "AUDIT render: fps={:.1} max_dt_ms={:.1} spawned={} despawned={} reload_events={} full_rebuilds={} state_transitions_total={}",
        fps,
        audit.interval_max_frame_ms,
        audit.interval_spawned,
        audit.interval_despawned,
        audit.interval_reload_events,
        audit.interval_full_rebuilds,
        audit.total_state_transitions
    );
    audit.interval_frames = 0;
    audit.interval_spawned = 0;
    audit.interval_despawned = 0;
    audit.interval_reload_events = 0;
    audit.interval_full_rebuilds = 0;
    audit.interval_max_frame_ms = 0.0;
    audit.interval_elapsed = 0.0;
}

#[derive(Resource, Default)]
pub struct GameSession {
    pub pack_name: String,
    pub level_index: usize,
    pub level_label: String,
    /// Attempts this level (1 on fresh load, +1 on each restart).
    pub tries: u32,
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
    mut core_events: MessageWriter<CoreGameEvent>,
    keys: Res<ButtonInput<KeyCode>>,
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
    } else if let Some(dir) = player_move_on_tick(&keys, &mut steering, bridge.facing_direction) {
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
        core_events.write(CoreGameEvent(e.clone()));
    }
    bridge.events_queue = events;
}
