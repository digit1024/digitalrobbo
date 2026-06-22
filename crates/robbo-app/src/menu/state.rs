//! Main-menu navigation state (screen stack, selection index).

use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MainMenuScreen {
    #[default]
    Root,
    Settings,
    LevelSelect,
}

#[derive(Resource, Default)]
pub struct MainMenuState {
    pub screen: MainMenuScreen,
    pub selection: usize,
}

#[derive(Resource, Default)]
pub struct MainMenuUiDirty(pub bool);

#[derive(Component, Clone, Copy)]
pub enum MainMenuAction {
    Start,
    OpenLevelSelect,
    OpenSettings,
    Back,
    MusicLess,
    MusicMore,
    ToggleMute,
    PackPrev,
    PackNext,
    LevelPrev,
    LevelNext,
    PlayLevel,
}

#[derive(Component)]
pub struct MainMenuItem {
    pub index: usize,
}

#[derive(Component)]
pub struct MainMenuUiRoot;

#[derive(Component)]
pub struct MainMenuPlanet;

#[derive(Component)]
pub struct MainMenuBackground;

#[derive(Component)]
pub struct MusicVolumeLabel;

#[derive(Component)]
pub struct LevelSelectPackLabel;

#[derive(Component)]
pub struct LevelSelectLevelLabel;

#[derive(Component)]
pub struct LevelSelectStatusLabel;

impl MainMenuScreen {
    pub fn item_count(self) -> usize {
        match self {
            MainMenuScreen::Root => 3,
            MainMenuScreen::Settings => 3,
            MainMenuScreen::LevelSelect => 4,
        }
    }

    pub fn header(self) -> &'static str {
        match self {
            MainMenuScreen::Root => "DIGITAL ROBBO",
            MainMenuScreen::Settings => "SETTINGS",
            MainMenuScreen::LevelSelect => "SELECT LEVEL",
        }
    }
}
