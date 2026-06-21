use bevy::prelude::*;

#[derive(States, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum AppState {
    #[default]
    Boot,
    Intro,
    MainMenu,
    LevelSelect,
    Playing,
    Paused,
    LevelComplete,
    GameOver,
}
