use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaveData {
    pub version: u32,
    pub profile: ProfileData,
    pub packs: std::collections::HashMap<String, PackProgress>,
    pub settings: SettingsData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileData {
    pub last_pack: String,
    pub last_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackProgress {
    pub levels: std::collections::HashMap<String, LevelProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LevelProgress {
    pub completed: bool,
    pub best_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    #[serde(default = "default_stored_music")]
    pub stored_music_volume: f32,
    #[serde(default = "default_stored_sfx")]
    pub stored_sfx_volume: f32,
    pub colourblind_mode: bool,
    pub show_grid: bool,
    pub skin: String,
}

fn default_stored_music() -> f32 {
    0.5
}

fn default_stored_sfx() -> f32 {
    1.0
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            music_volume: 0.5,
            sfx_volume: 1.0,
            stored_music_volume: 0.5,
            stored_sfx_volume: 1.0,
            colourblind_mode: false,
            show_grid: false,
            skin: "default".into(),
        }
    }
}

pub trait SaveStorage: Send + Sync {
    fn load(&self) -> Option<SaveData>;
    fn save(&self, data: &SaveData) -> bool;
}

pub struct FileSaveStorage {
    pub path: PathBuf,
}

impl SaveStorage for FileSaveStorage {
    fn load(&self) -> Option<SaveData> {
        let s = std::fs::read_to_string(&self.path).ok()?;
        ron::from_str(&s).ok()
    }

    fn save(&self, data: &SaveData) -> bool {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Some(s) = ron::ser::to_string_pretty(data, Default::default()).ok() else {
            return false;
        };
        std::fs::write(&self.path, s).is_ok()
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WebSaveStorage;

#[cfg(target_arch = "wasm32")]
impl SaveStorage for WebSaveStorage {
    fn load(&self) -> Option<SaveData> {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        let s = storage.get_item("digitalrobbo_save").ok()??;
        ron::from_str(&s).ok()
    }

    fn save(&self, data: &SaveData) -> bool {
        let Some(s) = ron::ser::to_string_pretty(data, Default::default()).ok() else {
            return false;
        };
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                return storage.set_item("digitalrobbo_save", &s).is_ok();
            }
        }
        false
    }
}

#[derive(Resource)]
pub struct GameFont(pub Handle<Font>);

#[derive(Resource)]
pub struct GameSave(pub SaveData);

impl Default for GameSave {
    fn default() -> Self {
        Self(SaveData {
            version: 1,
            ..Default::default()
        })
    }
}

pub fn default_save_path() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config/digitalrobbo/save.ron"))
        .unwrap_or_else(|_| PathBuf::from(".digitalrobbo/save.ron"))
}

pub fn load_save() -> SaveData {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = WebSaveStorage;
        return storage.load().unwrap_or_default();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let storage = FileSaveStorage {
            path: default_save_path(),
        };
        storage.load().unwrap_or_default()
    }
}

pub fn persist_save(save: &SaveData) {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = WebSaveStorage;
        let _ = storage.save(save);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let storage = FileSaveStorage {
            path: default_save_path(),
        };
        let _ = storage.save(save);
    }
}
