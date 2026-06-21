//! Headless-ish integration helpers: boot the real Bevy app, drive menus/input,
//! capture screenshots, and assert simulation state.
//!
//! Requires a GPU stack (use `scripts/run_game_smoke_test.sh` with xvfb on CI).

use std::path::{Path, PathBuf};

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use robbo_core::Cell;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, LoadLevelEvent};
use crate::input::InputCooldown;
use crate::levels::LevelSelection;

pub struct GameTestHarness {
    app: App,
}

impl GameTestHarness {
    pub fn new() -> Self {
        let mut app = crate::build_test_app();
        app.finish();
        app.cleanup();
        Self { app }
    }

    pub fn tick(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
        }
    }

    pub fn skip_intro(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::MainMenu);
        self.tick(3);
    }

    pub fn start_level(&mut self, pack_index: usize, level_index: usize) {
        {
            let world = self.app.world_mut();
            world.resource_mut::<LevelSelection>().pack_index = pack_index;
            world.resource_mut::<LevelSelection>().level_index = level_index;
            world.resource_mut::<InputCooldown>().frames_remaining = 0;
            world.send_event(LoadLevelEvent { restart: true });
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::Playing);
        }
        self.tick(60);
    }

    pub fn state(&self) -> AppState {
        self.app.world().resource::<State<AppState>>().get().clone()
    }

    pub fn robbo_cell(&self) -> Option<Cell> {
        self.app.world().resource::<CoreBridge>().world.robbo_cell()
    }

    pub fn is_animating(&self) -> bool {
        self.app.world().resource::<CoreBridge>().animating
    }

    pub fn press_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    pub fn release_all_keys(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }

    /// Simulate one tile move: hold `key` until Robbo's cell changes or we time out.
    pub fn move_once(&mut self, key: KeyCode, max_frames: usize) -> bool {
        let before = self.robbo_cell();
        self.press_key(key);
        let mut moved = false;
        for _ in 0..max_frames {
            self.tick(1);
            if !self.is_animating() && self.robbo_cell() != before {
                moved = true;
                break;
            }
        }
        self.release_all_keys();
        self.tick(8);
        moved
    }
}

#[derive(Resource)]
struct SmokeScenario {
    output_dir: PathBuf,
    step: SmokeStep,
    wait_frames: u32,
    start_cell: Option<Cell>,
    moved: bool,
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SmokeStep {
    Boot,
    SkipIntro,
    LoadLevel,
    Warmup,
    ShotBefore,
    Move,
    ShotAfter,
    Done,
}

impl SmokeScenario {
    fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            step: SmokeStep::Boot,
            wait_frames: 0,
            start_cell: None,
            moved: false,
            error: None,
        }
    }

    fn fail(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
        self.step = SmokeStep::Done;
    }
}

fn smoke_scenario_system(
    mut scenario: ResMut<SmokeScenario>,
    mut app_exit: EventWriter<AppExit>,
    state: Res<State<AppState>>,
    bridge: Res<CoreBridge>,
    mut next: ResMut<NextState<AppState>>,
    mut selection: ResMut<LevelSelection>,
    mut load: EventWriter<LoadLevelEvent>,
    mut cooldown: ResMut<InputCooldown>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if scenario.step == SmokeStep::Done {
        if scenario.wait_frames > 0 {
            scenario.wait_frames -= 1;
            return;
        }
        let after_path = scenario.output_dir.join("level1-after-move.png");
        if !after_path.is_file() {
            scenario.fail(format!("missing screenshot: {}", after_path.display()));
            app_exit.send(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
            return;
        }
        if scenario.error.is_some() {
            app_exit.send(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
        } else {
            app_exit.send(AppExit::Success);
        }
        return;
    }

    if scenario.wait_frames > 0 {
        scenario.wait_frames -= 1;
        return;
    }

    match scenario.step {
        SmokeStep::Boot => {
            scenario.step = SmokeStep::SkipIntro;
            scenario.wait_frames = 2;
        }
        SmokeStep::SkipIntro => {
            next.set(AppState::MainMenu);
            scenario.step = SmokeStep::LoadLevel;
            scenario.wait_frames = 5;
        }
        SmokeStep::LoadLevel => {
            selection.pack_index = 0;
            selection.level_index = 0;
            cooldown.frames_remaining = 0;
            load.send(LoadLevelEvent { restart: true });
            next.set(AppState::Playing);
            scenario.step = SmokeStep::Warmup;
            scenario.wait_frames = 45;
        }
        SmokeStep::Warmup => {
            if *state.get() != AppState::Playing {
                scenario.fail(format!("expected Playing, got {:?}", state.get()));
                return;
            }
            let Some(cell) = bridge.world.robbo_cell() else {
                scenario.fail("Robbo not found after level load");
                return;
            };
            scenario.start_cell = Some(cell);
            scenario.step = SmokeStep::ShotBefore;
        }
        SmokeStep::ShotBefore => {
            let path = scenario.output_dir.join("level1-start.png");
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            scenario.step = SmokeStep::Move;
            scenario.wait_frames = 45;
        }
        SmokeStep::Move => {
            keys.press(KeyCode::ArrowRight);
            scenario.step = SmokeStep::ShotAfter;
            scenario.wait_frames = 90;
        }
        SmokeStep::ShotAfter => {
            keys.clear();
            let after = bridge.world.robbo_cell();
            let moved = match (scenario.start_cell, after) {
                (Some(start), Some(end)) => end.col > start.col,
                _ => false,
            };
            if !moved {
                scenario.fail("Robbo did not move right");
                return;
            }
            scenario.moved = true;
            let path = scenario.output_dir.join("level1-after-move.png");
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            scenario.step = SmokeStep::Done;
            scenario.wait_frames = 180;
        }
        SmokeStep::Done => {}
    }
}

/// End-to-end smoke via the real winit loop: intro → level 1 → move → PNG screenshots.
pub fn run_smoke_scenario(output_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let before_path = output_dir.join("level1-start.png");
    let after_path = output_dir.join("level1-after-move.png");
    let _ = std::fs::remove_file(&before_path);
    let _ = std::fs::remove_file(&after_path);

    let mut app = crate::build_app();
    app.insert_resource(SmokeScenario::new(output_dir.to_path_buf()));
    app.add_systems(Update, smoke_scenario_system);

    match app.run() {
        AppExit::Success => {}
        AppExit::Error(code) => {
            return Err(format!("smoke scenario failed (exit code {code})"));
        }
    }

    if !before_path.is_file() {
        return Err(format!("missing screenshot: {}", before_path.display()));
    }
    if !after_path.is_file() {
        return Err(format!("missing screenshot: {}", after_path.display()));
    }

    bevy::log::info!(
        "Smoke test OK — {}, {}",
        before_path.display(),
        after_path.display()
    );
    Ok(())
}
