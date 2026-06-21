//! Full-app smoke test: boot → level → move → screenshot.
//!
//! Needs a display/GPU. Run via `scripts/run_game_smoke_test.sh` (uses xvfb on headless machines).

use robbo_app::app_state::AppState;
use robbo_app::test_harness::GameTestHarness;

#[test]
fn boot_level_move_and_screenshot() {
    let mut harness = GameTestHarness::new();
    harness.tick(1);
    harness.skip_intro();
    assert_eq!(harness.state(), AppState::MainMenu);

    harness.start_level(0, 0);
    assert_eq!(harness.state(), AppState::Playing);

    let start = harness.robbo_cell().expect("Robbo should exist after level load");
    assert!(
        harness.move_once(bevy::prelude::KeyCode::ArrowRight, 180),
        "Robbo should move right on level 1"
    );
    let after = harness.robbo_cell().unwrap();
    assert!(
        after.col > start.col,
        "Robbo should be further right: {start:?} -> {after:?}"
    );
}
