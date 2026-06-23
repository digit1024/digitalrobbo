use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

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
    /// After the first full intro play-through, skip logo sequence on later launches.
    #[serde(default)]
    pub intro_seen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackProgress {
    pub levels: std::collections::HashMap<String, LevelProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LevelProgress {
    pub completed: bool,
    pub best_time_ms: u64,
    #[serde(default)]
    pub best_tries: u32,
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
    0.2
}

fn default_stored_sfx() -> f32 {
    1.0
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            music_volume: 0.2,
            sfx_volume: 1.0,
            stored_music_volume: 0.2,
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

#[cfg(target_os = "android")]
pub struct AndroidSaveStorage {
    path: PathBuf,
}

#[cfg(target_os = "android")]
impl AndroidSaveStorage {
    pub fn new() -> Self {
        let base = android_files_dir().unwrap_or_else(|| PathBuf::from("/data/data/org.bevyengine.digitalrobbo/files"));
        Self {
            path: base.join("digitalrobbo/save.ron"),
        }
    }
}

#[cfg(target_os = "android")]
impl SaveStorage for AndroidSaveStorage {
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

#[cfg(target_os = "android")]
fn android_files_dir() -> Option<PathBuf> {
    use jni::objects::JString;
    use jni::JavaVM;
    use ndk_context::android_context;

    let ctx = android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let files_dir = env
        .call_method(context, "getFilesDir", "()Ljava/io/File;", &[])
        .ok()?
        .l()
        .ok()?;
    let path_obj = env
        .call_method(files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    let path_jstr = JString::from(path_obj);
    let path: String = env.get_string(&path_jstr).ok()?.into();
    Some(PathBuf::from(path))
}

#[derive(Resource, Clone)]
pub struct SaveBackend(pub Arc<dyn SaveStorage>);

impl SaveBackend {
    pub fn platform_default() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            return Self(Arc::new(WebSaveStorage));
        }
        #[cfg(all(target_os = "android", not(target_arch = "wasm32")))]
        {
            return Self(Arc::new(AndroidSaveStorage::new()));
        }
        #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
        {
            Self(Arc::new(FileSaveStorage {
                path: default_save_path(),
            }))
        }
    }
}

#[derive(Resource)]
pub struct GameFont(pub Handle<Font>);

pub fn load_game_font(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(GameFont(
        asset_server.load("fonts/MarkerFelt.ttf"),
    ));
}

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

pub fn load_save(backend: &SaveBackend) -> SaveData {
    backend.0.load().unwrap_or_default()
}

pub fn persist_save(backend: &SaveBackend, save: &SaveData) {
    let _ = backend.0.save(save);
}
