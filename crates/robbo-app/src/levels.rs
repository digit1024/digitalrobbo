use bevy::prelude::*;
use robbo_formats::{LevelPack, parse_pack, parse_pack_str};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/level_packs.rs"));

#[derive(Resource, Default)]
pub struct LevelRegistry {
    pub packs: Vec<LevelPack>,
}

impl LevelRegistry {
    pub fn load_builtin() -> Self {
        let mut packs = Vec::new();

        for (filename, bytes) in PACK_FILES {
            match parse_pack(bytes) {
                Ok(pack) => {
                    bevy::log::info!(
                        "Loaded level pack '{}' from {filename} ({} levels)",
                        pack.name,
                        pack.levels.len()
                    );
                    packs.push(pack);
                }
                Err(e) => bevy::log::warn!("Failed to parse level pack {filename}: {e}"),
            }
        }

        sort_packs(&mut packs);

        if packs.is_empty() {
            bevy::log::warn!("No real level packs loaded — falling back to sample");
            if let Ok(pack) =
                parse_pack_str(include_str!("../../robbo-formats/tests/fixtures/sample.dat"))
            {
                packs.push(pack);
            }
        }

        Self { packs }
    }

    pub fn load_from_path(path: &Path) -> Option<LevelPack> {
        let bytes = std::fs::read(path).ok()?;
        parse_pack(&bytes).ok()
    }

    pub fn pack_by_index(&self, index: usize) -> Option<&LevelPack> {
        self.packs.get(index)
    }
}

/// Match GNU Robbo: alphabetical by pack name, with `Original` first.
fn sort_packs(packs: &mut Vec<LevelPack>) {
    packs.sort_by(|a, b| a.name.cmp(&b.name));
    if let Some(pos) = packs.iter().position(|p| p.name == "Original") {
        if pos != 0 {
            let original = packs.remove(pos);
            packs.insert(0, original);
        }
    }
}

#[derive(Resource, Default)]
pub struct LevelSelection {
    pub pack_index: usize,
    pub level_index: usize,
}

impl LevelSelection {
    pub fn selected_level<'a>(&self, registry: &'a LevelRegistry) -> Option<&'a robbo_formats::Level> {
        registry
            .pack_by_index(self.pack_index)
            .and_then(|p| p.levels.get(self.level_index))
    }
}

/// Restore pack/level from profile when starting the last game.
pub fn resolve_last_level(
    registry: &LevelRegistry,
    profile: &crate::persistence::ProfileData,
    selection: &mut LevelSelection,
) {
    if !profile.last_pack.is_empty() {
        if let Some((pi, pack)) = registry
            .packs
            .iter()
            .enumerate()
            .find(|(_, p)| p.name == profile.last_pack)
        {
            selection.pack_index = pi;
            let level_idx = profile.last_level.saturating_sub(1) as usize;
            selection.level_index = level_idx.min(pack.levels.len().saturating_sub(1));
            return;
        }
    }
    selection.pack_index = 0;
    selection.level_index = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_packs_parse() {
        let registry = LevelRegistry::load_builtin();
        assert!(
            registry.packs.len() >= 27,
            "expected all GNU Robbo packs, got {}",
            registry.packs.len()
        );
    }
}
