use bevy::prelude::*;

#[derive(States, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum AppState {
    #[default]
    Boot,
    MainMenu,
    LevelSelect,
    Playing,
    Paused,
    LevelComplete,
    GameOver,
}
