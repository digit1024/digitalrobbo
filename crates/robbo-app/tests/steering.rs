//! Steering integration tests via the headless Bevy harness.
//!
//! All scenarios share one App — winit cannot recreate the event loop per test.

use bevy::prelude::KeyCode;
use robbo_app::app_state::AppState;
use robbo_app::test_harness::GameTestHarness;
use robbo_core::Direction;

#[test]
fn steering_behaviors() {
    let mut harness = GameTestHarness::new();
    harness.tick(1);
    harness.skip_intro();
    harness.start_level(0, 0);
    assert_eq!(harness.state(), AppState::Playing);

    // Tap different direction → turn only, no move.
    let start = harness.robbo_cell().expect("Robbo on level");
    assert_eq!(harness.facing(), Direction::Down);
    harness.press_key(KeyCode::ArrowUp);
    harness.tick(1);
    harness.release_all_keys();
    assert_eq!(harness.robbo_cell(), Some(start));
    assert_eq!(harness.facing(), Direction::Up);

    // Turn cancels a locked tap (no step on the locked direction).
    harness.press_key(KeyCode::ArrowUp);
    harness.tick(1);
    harness.release_all_keys();
    harness.press_key(KeyCode::ArrowRight);
    harness.tick(1);
    harness.release_all_keys();
    harness.wait_sim_ticks(1);
    assert_eq!(
        harness.robbo_cell(),
        Some(start),
        "turn should cancel locked tap without stepping"
    );
    assert_eq!(harness.facing(), Direction::Right);

    // Short tap locks the next step — even if released before the tick fires.
    let tap_start = harness.robbo_cell().unwrap();
    harness.press_key(KeyCode::ArrowRight);
    harness.tick(1);
    harness.release_all_keys();
    harness.wait_sim_ticks(1);
    let after_tap = harness.robbo_cell().unwrap();
    assert_eq!(
        after_tap.col,
        tap_start.col + 1,
        "locked tap should move exactly one cell on the next tick"
    );
    assert!(
        harness.steering().tap_move.is_none(),
        "locked tap must be consumed after the step"
    );

    // Turn on tick boundary must not also step.
    let turn_start = harness.robbo_cell().unwrap();
    harness.press_key(KeyCode::ArrowDown);
    harness.tick(8);
    harness.release_all_keys();
    harness.tick(1);
    assert_eq!(harness.robbo_cell(), Some(turn_start));
    assert_eq!(harness.facing(), Direction::Down);

    // Hold → move every sim tick.
    let hold_start = harness.robbo_cell().unwrap();
    let start_tick = harness.sim_tick();
    harness.press_key(KeyCode::ArrowDown);
    harness.wait_sim_ticks(3);
    harness.release_all_keys();
    let end = harness.robbo_cell().unwrap();
    assert!(
        end.row > hold_start.row,
        "expected vertical movement while holding"
    );
    assert!(harness.sim_tick() >= start_tick + 3);

    // Release → stop on next tick (no further movement).
    harness.press_key(KeyCode::ArrowDown);
    harness.wait_sim_ticks(1);
    let after_one = harness.robbo_cell();
    harness.release_all_keys();
    harness.wait_sim_ticks(2);
    assert_eq!(harness.robbo_cell(), after_one);

    // Space → shoot latched then consumed on next sim tick.
    let tick_before = harness.sim_tick();
    harness.press_key(KeyCode::Space);
    harness.tick(1);
    harness.wait_sim_ticks(1);
    assert!(
        !harness.steering().shoot_pending,
        "shoot latch should be consumed after a sim tick"
    );
    assert!(harness.sim_tick() > tick_before);
    harness.release_all_keys();
}
