use std::collections::HashMap;

use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
use bevy::prelude::*;
use robbo_core::{ElementKind, GameEvent};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, CoreGameEvent, LoadLevelEvent};
use crate::levels::{LevelRegistry, LevelSelection};
use crate::persistence::{GameSave, persist_save};
use crate::ui::LevelCountdown;

const MANIFEST_STR: &str = include_str!("../../../assets/audio/manifest.ron");

#[derive(Debug, Clone, Deserialize)]
pub struct AudioManifestData {
    pub menu_music: String,
    pub level_music: Vec<String>,
    pub sfx: HashMap<String, String>,
}

#[derive(Resource, Clone)]
pub struct AudioManifest(pub AudioManifestData);

#[derive(Resource, Default)]
pub struct GameAudio {
    pub menu_bgm: Handle<AudioSource>,
    pub level_bgm: Vec<Handle<AudioSource>>,
    pub sfx: HashMap<String, Handle<AudioSource>>,
}

#[derive(Resource, Default)]
pub struct BgmState {
    pub track_path: Option<String>,
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct AudioGate {
    pub unlocked: bool,
}

#[derive(Resource, Default)]
pub struct PendingLevelBgm {
    pub path: Option<String>,
}

pub fn load_audio_manifest(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let manifest: AudioManifestData = ron::from_str(MANIFEST_STR).expect("valid audio manifest");
    let menu_bgm = asset_server.load(&manifest.menu_music);
    let level_bgm = manifest
        .level_music
        .iter()
        .map(|p| asset_server.load(p))
        .collect();
    let sfx = manifest
        .sfx
        .iter()
        .map(|(k, v)| (k.clone(), asset_server.load(v)))
        .collect();

    commands.insert_resource(AudioManifest(manifest));
    commands.insert_resource(GameAudio {
        menu_bgm,
        level_bgm,
        sfx,
    });
    commands.insert_resource(BgmState::default());
    commands.insert_resource(AudioGate::default());
    commands.insert_resource(PendingLevelBgm::default());
}

pub fn unlock_audio_on_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut gate: ResMut<AudioGate>,
) {
    if gate.unlocked {
        return;
    }
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        gate.unlocked = true;
    }
}

fn effective_music_volume(save: &GameSave) -> f32 {
    save.0.settings.master_volume * save.0.settings.music_volume
}

fn effective_sfx_volume(save: &GameSave) -> f32 {
    save.0.settings.master_volume * save.0.settings.sfx_volume
}

pub fn stop_bgm(
    mut commands: Commands,
    mut bgm: ResMut<BgmState>,
) {
    if let Some(entity) = bgm.entity.take() {
        commands.entity(entity).despawn();
    }
    bgm.track_path = None;
}

fn stop_bgm_internal(commands: &mut Commands, bgm: &mut BgmState) {
    if let Some(entity) = bgm.entity.take() {
        commands.entity(entity).despawn();
    }
}

fn start_bgm(
    commands: &mut Commands,
    bgm: &mut BgmState,
    path: &str,
    handle: Handle<AudioSource>,
    volume: f32,
) {
    if bgm.track_path.as_deref() == Some(path) && bgm.entity.is_some() {
        return;
    }
    stop_bgm_internal(commands, bgm);
    if volume <= 0.001 {
        bgm.track_path = Some(path.to_string());
        return;
    }
    let entity = commands
        .spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::LOOP.with_volume(Volume::new(volume.clamp(0.0, 1.0))),
        ))
        .id();
    bgm.track_path = Some(path.to_string());
    bgm.entity = Some(entity);
}

pub fn play_menu_bgm(
    mut commands: Commands,
    audio: Res<GameAudio>,
    manifest: Res<AudioManifest>,
    mut bgm: ResMut<BgmState>,
    save: Res<GameSave>,
    gate: Res<AudioGate>,
) {
    if !gate.unlocked {
        return;
    }
    start_bgm(
        &mut commands,
        &mut bgm,
        &manifest.0.menu_music,
        audio.menu_bgm.clone(),
        effective_music_volume(&save),
    );
}

pub fn queue_level_bgm_on_load(
    mut events: EventReader<LoadLevelEvent>,
    registry: Res<LevelRegistry>,
    selection: Res<LevelSelection>,
    manifest: Res<AudioManifest>,
    mut pending: ResMut<PendingLevelBgm>,
) {
    for _ev in events.read() {
        let Some(level) = selection.selected_level(&registry) else {
            continue;
        };
        let seed = robbo_formats::level_content_seed(level);
        let idx = robbo_formats::pick_level_music_index(seed, manifest.0.level_music.len());
        pending.path = idx.map(|i| manifest.0.level_music[i].clone());
    }
}

pub fn start_level_bgm_after_countdown(
    mut commands: Commands,
    countdown: Res<LevelCountdown>,
    pending: Res<PendingLevelBgm>,
    audio: Res<GameAudio>,
    manifest: Res<AudioManifest>,
    mut bgm: ResMut<BgmState>,
    save: Res<GameSave>,
    gate: Res<AudioGate>,
    state: Res<State<AppState>>,
) {
    if *state.get() != AppState::Playing || countdown.active {
        return;
    }
    if !gate.unlocked {
        return;
    }
    let Some(path) = pending.path.as_ref() else {
        return;
    };
    let Some(idx) = manifest.0.level_music.iter().position(|p| p == path) else {
        return;
    };
    let handle = audio.level_bgm[idx].clone();
    start_bgm(
        &mut commands,
        &mut bgm,
        path,
        handle,
        effective_music_volume(&save),
    );
}

pub fn resume_bgm_on_unpause(
    mut commands: Commands,
    pending: Res<PendingLevelBgm>,
    audio: Res<GameAudio>,
    manifest: Res<AudioManifest>,
    mut bgm: ResMut<BgmState>,
    save: Res<GameSave>,
    gate: Res<AudioGate>,
    countdown: Res<LevelCountdown>,
) {
    if !gate.unlocked || countdown.blocks_input() {
        return;
    }
    if bgm.entity.is_some() {
        return;
    }
    let Some(path) = pending.path.as_ref() else {
        return;
    };
    let Some(idx) = manifest.0.level_music.iter().position(|p| p == path) else {
        return;
    };
    start_bgm(
        &mut commands,
        &mut bgm,
        path,
        audio.level_bgm[idx].clone(),
        effective_music_volume(&save),
    );
}

pub fn play_sfx(
    commands: &mut Commands,
    audio: &GameAudio,
    save: &GameSave,
    key: &str,
) {
    let vol = effective_sfx_volume(save);
    if vol <= 0.001 {
        return;
    }
    let Some(handle) = audio.sfx.get(key) else {
        return;
    };
    commands.spawn((
        AudioPlayer::new(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::new(vol.clamp(0.0, 1.0))),
    ));
}

pub fn sfx_on_core_events(
    mut commands: Commands,
    mut reader: EventReader<CoreGameEvent>,
    bridge: Res<CoreBridge>,
    audio: Res<GameAudio>,
    save: Res<GameSave>,
) {
    for CoreGameEvent(event) in reader.read() {
        let key = match event {
            GameEvent::Moved { entity_id, .. } if *entity_id == bridge.world.robbo_id => {
                Some("walk")
            }
            GameEvent::Shot { .. } => Some("shoot"),
            GameEvent::Collected { kind, .. } => match kind {
                ElementKind::Screw => Some("collected_screw"),
                ElementKind::Key => Some("collected_key"),
                ElementKind::BulletPickup => Some("collected_ammo"),
                ElementKind::ExtraLife => Some("collected_life"),
                _ => None,
            },
            GameEvent::Exploded { .. } => Some("explosion"),
            GameEvent::DoorOpened => Some("door"),
            GameEvent::Teleported { .. } => Some("teleport"),
            GameEvent::Died { entity_id, .. } if *entity_id == bridge.world.robbo_id => {
                Some("death")
            }
            GameEvent::LevelComplete => Some("level_complete"),
            _ => None,
        };
        if let Some(k) = key {
            play_sfx(&mut commands, &audio, &save, k);
        }
    }
}

pub fn toggle_mute(save: &mut GameSave) {
    let s = &mut save.0.settings;
    if s.music_volume < 0.001 && s.sfx_volume < 0.001 {
        s.music_volume = s.stored_music_volume.max(0.5);
        s.sfx_volume = s.stored_sfx_volume.max(1.0);
    } else {
        s.stored_music_volume = s.music_volume;
        s.stored_sfx_volume = s.sfx_volume;
        s.music_volume = 0.0;
        s.sfx_volume = 0.0;
    }
    persist_save(&save.0);
}

pub fn is_muted(save: &GameSave) -> bool {
    save.0.settings.music_volume < 0.001 && save.0.settings.sfx_volume < 0.001
}

pub fn play_countdown_tick(
    commands: &mut Commands,
    audio: &GameAudio,
    save: &GameSave,
) {
    play_sfx(commands, audio, save, "countdown");
}
