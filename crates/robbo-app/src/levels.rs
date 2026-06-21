use bevy::prelude::*;
use robbo_formats::{LevelPack, parse_pack, parse_pack_str};
use std::path::Path;

#[derive(Resource, Default)]
pub struct LevelRegistry {
    pub packs: Vec<LevelPack>,
}

impl LevelRegistry {
    pub fn load_builtin() -> Self {
        let mut packs = Vec::new();

        // Real GPL level packs ship first; the 5x5 sample fixture is only a
        // fallback if no real pack can be loaded.
        let binary_packs = [
            include_bytes!("../../../assets/levels/original.dat").as_slice(),
            include_bytes!("../../../assets/levels/robbo01.dat").as_slice(),
        ];
        for bytes in binary_packs {
            match parse_pack(bytes) {
                Ok(pack) => {
                    bevy::log::info!(
                        "Loaded level pack '{}' ({} levels)",
                        pack.name,
                        pack.levels.len()
                    );
                    packs.push(pack);
                }
                Err(e) => bevy::log::warn!("Failed to parse level pack: {e}"),
            }
        }

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
