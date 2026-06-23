use std::collections::{HashMap, HashSet};

use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackSettings, Volume};
use bevy::prelude::*;
use robbo_core::{Cell, ElementKind, GameEvent, GunType};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::bridge::{CoreBridge, CoreGameEvent, LoadLevelEvent};
use crate::levels::{LevelRegistry, LevelSelection};
use crate::persistence::{GameSave, persist_save};
use crate::ui::LevelCountdown;

const MANIFEST_STR: &str = include_str!("../../../assets/audio/manifest.ron");

/// Grid squares at or inside this range play at full SFX volume.
const SFX_FULL_RANGE: i16 = 4;
/// Grid squares at or beyond this range are silent.
const SFX_SILENT_RANGE: i16 = 12;

/// Chebyshev distance — how many orthogonal steps cover both axes (king moves on a grid).
fn grid_distance(a: Cell, b: Cell) -> i16 {
    (a.col - b.col).abs().max((a.row - b.row).abs())
}

/// Linear falloff between [`SFX_FULL_RANGE`] and [`SFX_SILENT_RANGE`].
fn distance_attenuation(distance: i16) -> f32 {
    if distance <= SFX_FULL_RANGE {
        1.0
    } else if distance >= SFX_SILENT_RANGE {
        0.0
    } else {
        let span = (SFX_SILENT_RANGE - SFX_FULL_RANGE) as f32;
        1.0 - (distance - SFX_FULL_RANGE) as f32 / span
    }
}

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

#[derive(Resource, Default)]
pub(crate) struct EnemyAmbientSounds {
    by_id: HashMap<u32, Entity>,
}

#[derive(Component)]
pub(crate) struct EnemyAmbientSound {
    element_id: u32,
}

fn shoot_sfx_key(gun_type: GunType) -> &'static str {
    match gun_type {
        GunType::Regular => "shoot_regular",
        GunType::Laser => "shoot_laser",
        GunType::Blaster => "shoot_blaster",
    }
}

fn enemy_ambient_sfx_key(kind: &ElementKind) -> Option<&'static str> {
    match kind {
        ElementKind::Bear { .. } | ElementKind::BlackBear { .. } => Some("enemy_bear"),
        ElementKind::Bird { .. } => Some("enemy_bird"),
        ElementKind::Butterfly => Some("enemy_butterfly"),
        _ => None,
    }
}

fn spatial_sfx_volume(save: &GameSave, source: Cell, listener: Option<Cell>) -> Option<f32> {
    let base = effective_sfx_volume(save);
    if base <= 0.001 {
        return None;
    }
    let attenuation = match listener {
        Some(ear) => distance_attenuation(grid_distance(source, ear)),
        None => 1.0,
    };
    if attenuation <= 0.001 {
        return None;
    }
    Some((base * attenuation).clamp(0.0, 1.0))
}

fn clear_enemy_ambient_sounds(commands: &mut Commands, pool: &mut EnemyAmbientSounds) {
    for entity in pool.by_id.drain().map(|(_, e)| e) {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn cleanup_enemy_ambient_sounds(
    mut commands: Commands,
    mut pool: ResMut<EnemyAmbientSounds>,
) {
    clear_enemy_ambient_sounds(&mut commands, &mut pool);
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
    #[cfg(not(target_arch = "wasm32"))]
    commands.insert_resource(AudioGate { unlocked: true });
    #[cfg(target_arch = "wasm32")]
    commands.insert_resource(AudioGate::default());
    commands.insert_resource(PendingLevelBgm::default());
    commands.insert_resource(EnemyAmbientSounds::default());
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
    asset_server: &AssetServer,
    bgm: &mut BgmState,
    path: &str,
    handle: Handle<AudioSource>,
    volume: f32,
) {
    if bgm.track_path.as_deref() == Some(path) && bgm.entity.is_some() {
        return;
    }
    if !asset_server.load_state(&handle).is_loaded() {
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
    asset_server: Res<AssetServer>,
    audio: Res<GameAudio>,
    manifest: Res<AudioManifest>,
    mut bgm: ResMut<BgmState>,
    save: Res<GameSave>,
    gate: Res<AudioGate>,
) {
    play_menu_bgm_now(
        &mut commands,
        &asset_server,
        &audio,
        &manifest,
        &mut bgm,
        &save,
        &gate,
    );
}

/// Keep menu music running while on the main menu (gate unlock, return from level, asset load).
pub fn ensure_menu_bgm(
    mut commands: Commands,
    state: Res<State<AppState>>,
    asset_server: Res<AssetServer>,
    audio: Res<GameAudio>,
    manifest: Res<AudioManifest>,
    mut bgm: ResMut<BgmState>,
    save: Res<GameSave>,
    gate: Res<AudioGate>,
) {
    if *state.get() != AppState::MainMenu || !gate.unlocked {
        return;
    }
    let menu_path = manifest.0.menu_music.as_str();
    if bgm.track_path.as_deref() == Some(menu_path) && bgm.entity.is_some() {
        if asset_server.load_state(&audio.menu_bgm).is_loaded() {
            return;
        }
        stop_bgm_internal(&mut commands, &mut bgm);
    }
    play_menu_bgm_now(
        &mut commands,
        &asset_server,
        &audio,
        &manifest,
        &mut bgm,
        &save,
        &gate,
    );
}

/// Callable from intro when the title card appears (original `showTitle` timing).
pub fn play_menu_bgm_now(
    commands: &mut Commands,
    asset_server: &AssetServer,
    audio: &GameAudio,
    manifest: &AudioManifest,
    bgm: &mut BgmState,
    save: &GameSave,
    gate: &AudioGate,
) {
    if !gate.unlocked {
        return;
    }
    start_bgm(
        commands,
        asset_server,
        bgm,
        &manifest.0.menu_music,
        audio.menu_bgm.clone(),
        effective_music_volume(save),
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
    asset_server: Res<AssetServer>,
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
        &asset_server,
        &mut bgm,
        path,
        handle,
        effective_music_volume(&save),
    );
}

pub fn resume_bgm_on_unpause(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
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
        &asset_server,
        &mut bgm,
        path,
        audio.level_bgm[idx].clone(),
        effective_music_volume(&save),
    );
}

fn play_music_stinger(
    commands: &mut Commands,
    audio: &GameAudio,
    save: &GameSave,
    key: &str,
) {
    let vol = effective_music_volume(save);
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

pub fn play_sfx(
    commands: &mut Commands,
    audio: &GameAudio,
    save: &GameSave,
    key: &str,
    source: Option<Cell>,
    listener: Option<Cell>,
) {
    let vol = effective_sfx_volume(save);
    if vol <= 0.001 {
        return;
    }
    let attenuation = match (source, listener) {
        (Some(src), Some(ear)) => distance_attenuation(grid_distance(src, ear)),
        _ => 1.0,
    };
    if attenuation <= 0.001 {
        return;
    }
    let vol = vol * attenuation;
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
    let listener = bridge.world.robbo_cell();
    for CoreGameEvent(event) in reader.read() {
        if matches!(event, GameEvent::LevelComplete) {
            play_music_stinger(&mut commands, &audio, &save, "level_complete");
            continue;
        }
        let mapped = match event {
            GameEvent::Moved { entity_id, to, .. } if *entity_id == bridge.world.robbo_id => {
                Some(("walk", Some(*to)))
            }
            GameEvent::Shot { from, gun_type, .. } => {
                Some((shoot_sfx_key(*gun_type), Some(*from)))
            }
            GameEvent::Collected { kind, at } => match kind {
                ElementKind::Screw => Some(("collected_screw", Some(*at))),
                ElementKind::Key => Some(("collected_key", Some(*at))),
                ElementKind::BulletPickup => Some(("collected_ammo", Some(*at))),
                _ => None,
            },
            GameEvent::Exploded { at } => Some(("explosion", Some(*at))),
            GameEvent::Revealed { at } => Some(("explosion", Some(*at))),
            GameEvent::DoorOpened { at } => Some(("door", Some(*at))),
            GameEvent::Teleported { to, .. } => Some(("teleport", Some(*to))),
            GameEvent::Died { entity_id, .. } if *entity_id == bridge.world.robbo_id => {
                Some(("death", listener))
            }
            _ => None,
        };
        if let Some((key, source)) = mapped {
            play_sfx(&mut commands, &audio, &save, key, source, listener);
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

pub const MUSIC_VOLUME_STEP: f32 = 0.1;

pub fn adjust_music_volume(save: &mut GameSave, delta: f32) {
    let s = &mut save.0.settings;
    let new = (s.music_volume + delta).clamp(0.0, 1.0);
    s.music_volume = new;
    s.stored_music_volume = new;
    persist_save(&save.0);
}

pub fn music_volume_percent(save: &GameSave) -> u8 {
    (save.0.settings.music_volume.clamp(0.0, 1.0) * 100.0).round() as u8
}

/// Apply saved music level to the currently playing BGM entity, if any.
pub fn apply_live_music_volume(save: &GameSave, bgm: &BgmState, sinks: &Query<&AudioSink>) {
    let Some(entity) = bgm.entity else {
        return;
    };
    let Ok(sink) = sinks.get(entity) else {
        return;
    };
    sink.set_volume(effective_music_volume(save));
}

pub fn play_countdown_tick(
    commands: &mut Commands,
    audio: &GameAudio,
    save: &GameSave,
) {
    play_sfx(commands, audio, save, "countdown", None, None);
}

/// Per-enemy looping ambient SFX with distance falloff from Robbo.
pub(crate) fn update_enemy_ambient_sounds(
    mut commands: Commands,
    mut pool: ResMut<EnemyAmbientSounds>,
    mut load_events: EventReader<LoadLevelEvent>,
    bridge: Res<CoreBridge>,
    audio: Res<GameAudio>,
    save: Res<GameSave>,
    gate: Res<AudioGate>,
    state: Res<State<AppState>>,
    countdown: Res<LevelCountdown>,
    sinks: Query<(&EnemyAmbientSound, &AudioSink)>,
) {
    for _ in load_events.read() {
        clear_enemy_ambient_sounds(&mut commands, &mut pool);
    }

    let active = *state.get() == AppState::Playing && gate.unlocked && !countdown.active;
    if !active {
        if !pool.by_id.is_empty() {
            clear_enemy_ambient_sounds(&mut commands, &mut pool);
        }
        return;
    }

    let listener = bridge.world.robbo_cell();
    let mut live_ids = HashSet::new();

    for (cell, el) in &bridge.world.elements {
        let Some(key) = enemy_ambient_sfx_key(&el.kind) else {
            continue;
        };
        if el.hidden {
            continue;
        }
        live_ids.insert(el.id);

        if pool.by_id.contains_key(&el.id) {
            continue;
        }
        let Some(vol) = spatial_sfx_volume(&save, *cell, listener) else {
            continue;
        };
        let Some(handle) = audio.sfx.get(key) else {
            continue;
        };
        let entity = commands
            .spawn((
                AudioPlayer::new(handle.clone()),
                PlaybackSettings::LOOP.with_volume(Volume::new(vol)),
                EnemyAmbientSound {
                    element_id: el.id,
                },
            ))
            .id();
        pool.by_id.insert(el.id, entity);
    }

    pool.by_id.retain(|id, entity| {
        if live_ids.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });

    for (marker, sink) in &sinks {
        let Some((cell, _)) = bridge
            .world
            .elements
            .iter()
            .find(|(_, s)| s.id == marker.element_id)
            .map(|(c, s)| (*c, s))
        else {
            continue;
        };
        let vol = spatial_sfx_volume(&save, cell, listener).unwrap_or(0.0);
        sink.set_volume(vol);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robbo_core::Cell;

    #[test]
    fn attenuation_full_inside_range() {
        assert_eq!(distance_attenuation(0), 1.0);
        assert_eq!(distance_attenuation(4), 1.0);
    }

    #[test]
    fn attenuation_silent_outside_range() {
        assert_eq!(distance_attenuation(12), 0.0);
        assert_eq!(distance_attenuation(20), 0.0);
    }

    #[test]
    fn attenuation_linear_mid_range() {
        assert!((distance_attenuation(8) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn shoot_sfx_keys_per_gun_type() {
        assert_eq!(shoot_sfx_key(GunType::Regular), "shoot_regular");
        assert_eq!(shoot_sfx_key(GunType::Laser), "shoot_laser");
        assert_eq!(shoot_sfx_key(GunType::Blaster), "shoot_blaster");
    }

    #[test]
    fn enemy_ambient_keys() {
        assert_eq!(
            enemy_ambient_sfx_key(&ElementKind::Bear { clockwise: true }),
            Some("enemy_bear")
        );
        assert_eq!(
            enemy_ambient_sfx_key(&ElementKind::Bird {
                variant: robbo_core::BirdVariant::Horizontal,
                shooting: false,
            }),
            Some("enemy_bird")
        );
        assert_eq!(
            enemy_ambient_sfx_key(&ElementKind::Butterfly),
            Some("enemy_butterfly")
        );
    }

    #[test]
    fn grid_distance_uses_chebyshev() {
        let a = Cell::new(0, 0);
        let b = Cell::new(3, 5);
        assert_eq!(grid_distance(a, b), 5);
    }
}
